//! Headroom editor: Full, Strip and Compact layouts around one two-layer display (true-peak waveform + short-term LUFS).
use crate::meter::{Bar, OverEvent, Snapshot};
use crate::state::*;
use crate::{HeadParams, Shared};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Align2, Color32, Pos2, Rect, Sense, Stroke, UiBuilder};
use nih_plug_egui::{create_egui_editor, EguiState};
use phub_common::ui::{self, *};
use std::sync::atomic::Ordering;
use std::sync::Arc;

const AMBER: Color32 = Color32::from_rgb(0xff, 0xb3, 0x47);
const WAVE: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x60);
const FLOOR_TP: f32 = -48.0;
const CEIL_TP: f32 = 3.0;

pub struct Local {
    pub selected_over: Option<usize>,
    scroll_to_overs: bool,
    held: [f32; 4],
    held_t: [f32; 4],
    synced: Option<Mode>,
    grip_start: Option<Pos2>,
    pub snap: Snapshot,
    pub bars: Vec<Bar>,
    pub first_index: u64,
    pub bar_beats: f64,
    pub playing: bool,
}

pub fn new_local() -> Local {
    Local { selected_over: None, scroll_to_overs: false, held: [f32::NEG_INFINITY; 4], held_t: [0.0; 4], synced: None, grip_start: None, snap: Snapshot::default(), bars: Vec::new(), first_index: 0, bar_beats: 4.0, playing: false }
}

pub fn create(params: Arc<HeadParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, new_local(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
}

fn fmt_lufs(v: f32) -> String {
    if v.is_finite() { format!("{v:.1}") } else { "\u{2014}".to_string() }
}
fn fmt_db(v: f32, unit: &str) -> String {
    if v.is_finite() { format!("{v:+.1} {unit}") } else { "\u{2014}".to_string() }
}

/// Integrated: accent within 1 LU of target, amber to 3 LU, red beyond.
fn int_color(i: f32, target: f32) -> Color32 {
    if !i.is_finite() {
        return MUTED;
    }
    let d = (i - target).abs();
    if d <= 1.0 { ACCENT } else if d <= 3.0 { AMBER } else { RED }
}
fn tp_color(tp: f32, ceiling: f32) -> Color32 {
    if tp.is_finite() && tp > ceiling { RED } else { TEXT }
}

pub fn format_position(o: &OverEvent, bar_beats: f64) -> String {
    let secs = o.seconds.max(0.0);
    let clock = format!("{}:{:02}", (secs / 60.0) as u32, (secs % 60.0) as u32);
    match o.beats {
        Some(b) if b >= 0.0 => format!("Bar {}.{} \u{b7} {} \u{b7} {:+.1} dBTP", (b / bar_beats) as u32 + 1, (b % bar_beats) as u32 + 1, clock, o.peak_db),
        _ => format!("{} \u{b7} {:+.1} dBTP", clock, o.peak_db),
    }
}

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<HeadParams>, shared: &Arc<Shared>) {
    let _ = setter;
    let mut st = *params.settings.read();
    let before = st;
    // pull the latest numbers and bars
    if let Some(s) = shared.snap.try_lock() {
        local.snap = s.clone();
    }
    if let Some(b) = shared.bars.try_lock() {
        local.first_index = b.first_index;
        local.bars = b.bars.iter().copied().collect();
    }
    local.bar_beats = *shared.bar_beats.lock();
    if local.bar_beats <= 0.0 {
        local.bar_beats = 4.0;
    }
    local.playing = shared.playing.load(Ordering::Relaxed);
    // a window mode owns a fixed size
    let (mw, _) = st.size();
    if local.synced != Some(st.mode) || params.editor_state.size() != st.size() {
        if local.synced != Some(st.mode) || params.editor_state.size().0 != mw {
            params.editor_state.set_requested_size(st.size());
        }
        local.synced = Some(st.mode);
    }
    let (ww, _) = params.editor_state.size();
    ctx.set_zoom_factor((ww as f32 / mw as f32).clamp(0.5, 2.5));

    let target = st.target();
    let ceiling = st.ceiling(local.snap.integrated);
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(0))).show(ctx, |ui| {
        let r = ui.max_rect();
        match st.mode {
            Mode::Compact => compact(ui, r, &mut st, local, shared, target, ceiling),
            Mode::Strip => strip(ui, r, &mut st, local, shared, target, ceiling),
            Mode::Full => full(ui, r, &mut st, local, shared, target, ceiling),
        }
        grip(ui, r, &mut st, local);
    });
    if st != before {
        *params.settings.write() = st;
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

// ---- shared pieces ----

fn big_number(ui: &mut egui::Ui, r: Rect, label: &str, value: &str, color: Color32, size: f32) -> egui::Response {
    let resp = ui.allocate_rect(r, Sense::click());
    ui.painter().text(r.left_top(), Align2::LEFT_TOP, label, sora(9.0), MUTED);
    ui.painter().text(r.left_top() + vec2(0.0, 11.0), Align2::LEFT_TOP, value, mono(size), color);
    resp
}

fn target_menu(ui: &mut egui::Ui, st: &mut HeadSettings, label: String, width: f32) {
    ui.menu_button(egui::RichText::new(label).font(sora(11.0)), |ui| {
        ui.set_min_width(width);
        for (i, (name, t, c)) in TARGETS.iter().enumerate() {
            let txt = if i < 5 { format!("{name}   {t:.0} LUFS / {c:.0} dBTP") } else { "Custom".to_string() };
            if ui.selectable_label(st.preset == i, txt).clicked() {
                st.preset = i;
                ui.close_menu();
            }
        }
        if st.preset == 5 {
            ui.add(egui::Slider::new(&mut st.custom_target, -30.0..=-6.0).text("LUFS"));
            ui.add(egui::Slider::new(&mut st.custom_ceiling, -3.0..=0.0).text("dBTP"));
        }
        ui.separator();
        ui.checkbox(&mut st.auto_reset, "Reset when playback starts");
        ui.checkbox(&mut st.measure_stopped, "Measure while stopped");
    });
}

fn turned_down(st: &HeadSettings, integrated: f32) -> Option<f32> {
    integrated.is_finite().then(|| integrated - st.target())
}

fn mode_buttons(ui: &mut egui::Ui, st: &mut HeadSettings) {
    for (m, name) in [(Mode::Compact, "Compact"), (Mode::Strip, "Strip"), (Mode::Full, "Full")] {
        let on = st.mode == m;
        let b = egui::Button::new(egui::RichText::new(name).font(sora(11.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { ACCENT } else { INSET }).min_size(vec2(60.0, 22.0));
        let resp = ui.add(b);
        if resp.clicked() {
            st.mode = m;
        }
        if m == Mode::Strip {
            resp.context_menu(|ui| {
                if ui.selectable_label(!st.strip_narrow, "Wide (1200)").clicked() {
                    st.strip_narrow = false;
                    st.mode = Mode::Strip;
                    ui.close_menu();
                }
                if ui.selectable_label(st.strip_narrow, "Narrow (860)").clicked() {
                    st.strip_narrow = true;
                    st.mode = Mode::Strip;
                    ui.close_menu();
                }
            });
        }
    }
}

fn reset_button(ui: &mut egui::Ui, shared: &Arc<Shared>, local: &mut Local) {
    if ui.add(egui::Button::new(egui::RichText::new("Reset").font(sora(10.0))).min_size(vec2(44.0, 22.0))).on_hover_text("Reset all meters").clicked() {
        shared.reset.store(true, Ordering::Relaxed);
        local.bars.clear();
    }
}

/// Bottom-right grip: drag it left or right to step to the next smaller or larger layout.
fn grip(ui: &mut egui::Ui, r: Rect, st: &mut HeadSettings, local: &mut Local) {
    let g = Rect::from_min_size(r.right_bottom() - vec2(16.0, 16.0), vec2(16.0, 16.0));
    let resp = ui.interact(g, egui::Id::new("grip"), Sense::drag());
    for k in 0..3 {
        let o = 3.0 + k as f32 * 4.0;
        ui.painter().line_segment([g.right_bottom() - vec2(o, 1.0), g.right_bottom() - vec2(1.0, o)], Stroke::new(1.0_f32, WIRE));
    }
    if resp.drag_started() {
        local.grip_start = resp.interact_pointer_pos();
    }
    if resp.drag_stopped() {
        if let (Some(a), Some(b)) = (local.grip_start.take(), resp.interact_pointer_pos()) {
            let d = b - a;
            let order = [Mode::Compact, Mode::Strip, Mode::Full];
            let cur = order.iter().position(|m| *m == st.mode).unwrap_or(2);
            if d.x + d.y > 60.0 && cur < 2 {
                st.mode = order[cur + 1];
            } else if d.x + d.y < -60.0 && cur > 0 {
                st.mode = order[cur - 1];
            }
        }
    }
}

/// The two-layer display. `bars` is the visible window (oldest first), `per_col` bars are merged into each column.
fn wave(ui: &mut egui::Ui, r: Rect, bars: &[Bar], span: usize, ceiling: f32, target: f32, highlight: Option<usize>) {
    let p = ui.painter_at(r);
    p.rect_filled(r, 10.0, PANEL);
    let inner = r.shrink2(vec2(8.0, 6.0));
    let mid = inner.center().y;
    let half = inner.height() * 0.5;
    let n = span.max(1);
    let cols = inner.width().max(1.0) as usize;
    let first = bars.len().saturating_sub(n);
    let visible = &bars[first..];
    let col_w = inner.width() / cols as f32;
    let hf = |db: f32| ((db - FLOOR_TP) / (CEIL_TP - FLOOR_TP)).clamp(0.0, 1.0) * half;
    // target band on the LUFS scale (-30..0)
    let ly = |l: f32| inner.bottom() - ((l + 30.0) / 30.0).clamp(0.0, 1.0) * inner.height();
    p.rect_filled(Rect::from_min_max(pos2(inner.left(), ly(target + 1.0)), pos2(inner.right(), ly(target - 1.0))), 0.0, ACCENT.gamma_multiply(0.12));
    let right_aligned_n = n;
    let mut line: Vec<Pos2> = Vec::new();
    let mut line_amber: Vec<bool> = Vec::new();
    for c in 0..cols {
        // bars (from the right edge): column c covers slice [a, b) of the window
        let a = c * right_aligned_n / cols;
        let b = ((c + 1) * right_aligned_n / cols).max(a + 1);
        let off = right_aligned_n.saturating_sub(visible.len());
        let (sa, sb) = (a.saturating_sub(off), b.saturating_sub(off));
        if sb <= sa || sa >= visible.len() {
            continue;
        }
        let slice = &visible[sa..sb.min(visible.len())];
        let tp = slice.iter().fold(0.0f32, |m, b| m.max(b.tp));
        let over = slice.iter().any(|b| b.over);
        let st = slice.iter().map(|b| b.st).fold(f32::NEG_INFINITY, f32::max);
        let x = inner.left() + c as f32 * col_w;
        let h = hf(20.0 * tp.max(1e-6).log10());
        let hl = highlight.map_or(false, |hi| hi >= first + sa && hi < first + sb.min(visible.len()));
        if over {
            p.line_segment([pos2(x, mid - half), pos2(x, mid + half)], Stroke::new(col_w.max(1.0), RED));
            p.line_segment([pos2(x, mid - half - 1.0), pos2(x, mid - half - 5.0)], Stroke::new(1.5_f32, RED));
        } else {
            p.line_segment([pos2(x, mid - h), pos2(x, mid + h)], Stroke::new(col_w.max(1.0), WAVE));
        }
        if hl {
            p.rect_filled(Rect::from_min_max(pos2(x - 3.0, inner.top()), pos2(x + 3.0, inner.bottom())), 0.0, Color32::from_white_alpha(40));
        }
        if st.is_finite() {
            line.push(pos2(x, ly(st)));
            line_amber.push(st > target + 1.0);
        }
    }
    for w in 0..line.len().saturating_sub(1) {
        p.line_segment([line[w], line[w + 1]], Stroke::new(1.5_f32, if line_amber[w] { AMBER } else { TEXT }));
    }
    // ceiling: red dashed
    let cy = mid - hf(ceiling);
    let mut x = inner.left();
    while x < inner.right() {
        p.line_segment([pos2(x, cy), pos2((x + 5.0).min(inner.right()), cy)], Stroke::new(1.0_f32, RED));
        x += 10.0;
    }
    p.text(pos2(inner.right() - 2.0, cy - 1.0), Align2::RIGHT_BOTTOM, format!("{ceiling:.0} dBTP"), mono(8.0), RED);
    // hover read-out
    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
        if r.contains(pos) && !visible.is_empty() {
            let c = ((pos.x - inner.left()) / col_w).clamp(0.0, cols as f32 - 1.0) as usize;
            let idx = (c * right_aligned_n / cols).saturating_sub(right_aligned_n.saturating_sub(visible.len()));
            if let Some(b) = visible.get(idx.min(visible.len() - 1)) {
                let ago = (right_aligned_n - c * right_aligned_n / cols) as f32 * 0.02;
                let txt = format!("-{ago:.1} s   S-T {} LUFS   TP {:+.1} dBTP", fmt_lufs(b.st), 20.0 * b.tp.max(1e-6).log10());
                p.line_segment([pos2(pos.x, inner.top()), pos2(pos.x, inner.bottom())], Stroke::new(1.0_f32, Color32::from_white_alpha(60)));
                p.text(pos2(inner.left() + 6.0, inner.top() + 2.0), Align2::LEFT_TOP, txt, mono(9.0), TEXT);
            }
        }
    }
}

fn history_menu(resp: &egui::Response, st: &mut HeadSettings) {
    resp.context_menu(|ui| {
        for s in [10u32, 30, 60] {
            if ui.selectable_label(st.history_s == s, format!("Last {s} s")).clicked() {
                st.history_s = s;
                ui.close_menu();
            }
        }
    });
}

// ---- layouts ----

fn compact(ui: &mut egui::Ui, r: Rect, st: &mut HeadSettings, local: &mut Local, shared: &Arc<Shared>, target: f32, ceiling: f32) {
    let s = local.snap.clone();
    let numbers = Rect::from_min_size(r.min + vec2(10.0, 8.0), vec2(r.width() - 20.0, 40.0));
    let cell = (numbers.width() - 195.0) / 4.0;
    let items = [("INT", fmt_lufs(s.integrated), int_color(s.integrated, target)), ("TP", fmt_db(s.tp_max[0].max(s.tp_max[1]), "").trim().to_string(), tp_color(s.tp_max[0].max(s.tp_max[1]), ceiling)), ("S-T", fmt_lufs(s.short_term), int_color(s.short_term, target)), ("OVERS", s.overs.to_string(), if s.overs > 0 { RED } else { TEXT })];
    for (k, (l, v, c)) in items.iter().enumerate() {
        let rr = Rect::from_min_size(numbers.min + vec2(k as f32 * cell, 0.0), vec2(cell, 40.0));
        let resp = big_number(ui, rr, l, v, *c, 17.0);
        if k == 0 && resp.double_clicked() {
            shared.reset.store(true, Ordering::Relaxed);
        }
        if k == 3 && resp.clicked() {
            st.mode = Mode::Full;
            local.scroll_to_overs = true;
        }
    }
    let tail = Rect::from_min_size(pos2(numbers.right() - 190.0, numbers.top()), vec2(190.0, 40.0));
    ui.scope_builder(UiBuilder::new().max_rect(tail).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        let down = turned_down(st, s.integrated).map_or(String::new(), |d| if d > 0.0 { format!(" -{d:.1}") } else { String::new() });
        target_menu(ui, st, format!("{}{}", st.name(), down), 220.0);
        reset_button(ui, shared, local);
        if ui.add(egui::Button::new(egui::RichText::new("Full").font(sora(10.0))).min_size(vec2(34.0, 22.0))).on_hover_text("Full view").clicked() {
            st.mode = Mode::Full;
        }
    });
    let w = Rect::from_min_max(pos2(r.left() + 8.0, numbers.bottom() + 6.0), r.max - vec2(8.0, 8.0));
    let resp = ui.allocate_rect(w, Sense::click());
    wave(ui, w, &local.bars, 30 * 50, ceiling, target, None);
    history_menu(&resp, st);
}

fn strip(ui: &mut egui::Ui, r: Rect, st: &mut HeadSettings, local: &mut Local, shared: &Arc<Shared>, target: f32, ceiling: f32) {
    let s = local.snap.clone();
    let left = Rect::from_min_size(r.min + vec2(10.0, 6.0), vec2(120.0, r.height() - 12.0));
    let tp = s.tp_max[0].max(s.tp_max[1]);
    let rows = [("INT", fmt_lufs(s.integrated), int_color(s.integrated, target)), ("S-T", fmt_lufs(s.short_term), int_color(s.short_term, target)), ("TP", fmt_db(tp, ""), tp_color(tp, ceiling)), ("LRA", if s.lra > 0.0 { format!("{:.1}", s.lra) } else { "\u{2014}".into() }, TEXT)];
    for (k, (l, v, c)) in rows.iter().enumerate() {
        let rr = Rect::from_min_size(left.min + vec2(0.0, k as f32 * 29.0), vec2(120.0, 28.0));
        ui.painter().text(rr.left_top() + vec2(0.0, 6.0), Align2::LEFT_TOP, *l, sora(9.0), MUTED);
        let resp = ui.allocate_rect(rr, Sense::click());
        ui.painter().text(rr.left_top() + vec2(34.0, 0.0), Align2::LEFT_TOP, v, mono(18.0), *c);
        if k == 0 && resp.double_clicked() {
            shared.reset.store(true, Ordering::Relaxed);
        }
    }
    let right = Rect::from_min_size(pos2(r.right() - 150.0, r.top() + 6.0), vec2(142.0, r.height() - 12.0));
    let w = Rect::from_min_max(pos2(left.right() + 6.0, r.top() + 6.0), pos2(right.left() - 6.0, r.bottom() - 6.0));
    let resp = ui.allocate_rect(w, Sense::click());
    wave(ui, w, &local.bars, st.history_s as usize * 50, ceiling, target, None);
    history_menu(&resp, st);
    ui.scope_builder(UiBuilder::new().max_rect(right).layout(egui::Layout::top_down(egui::Align::LEFT)), |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let down = turned_down(st, s.integrated).map_or("\u{2014}".to_string(), |d| if d > 0.0 { format!("-{d:.1} dB") } else { "quieter than target".to_string() });
        target_menu(ui, st, st.name().to_string(), 220.0);
        ui.label(egui::RichText::new(down).font(mono(11.0)).color(MUTED));
        ui.horizontal(|ui| {
            if ui.add(egui::Label::new(egui::RichText::new(format!("OVERS {}", s.overs)).font(mono(12.0)).color(if s.overs > 0 { RED } else { TEXT })).sense(Sense::click())).clicked() {
                st.mode = Mode::Full;
                local.scroll_to_overs = true;
            }
            reset_button(ui, shared, local);
        });
        if ui.add(egui::Button::new(egui::RichText::new("Full view").font(sora(10.0))).min_size(vec2(70.0, 20.0))).clicked() {
            st.mode = Mode::Full;
        }
    });
    let _ = local.scroll_to_overs;
}

fn bar_meter(ui: &mut egui::Ui, r: Rect, value: f32, hold: f32, lo: f32, hi: f32, color: Color32, label: &str, text: &str) {
    let frac = |v: f32| if v.is_finite() { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 };
    let bar = Rect::from_min_max(r.min, pos2(r.right(), r.bottom() - 26.0));
    meter_bar(ui.painter(), bar, frac(value), frac(hold), color, false);
    meter_label(ui.painter(), r.center().x, bar.bottom() + 3.0, label, text);
}

fn full(ui: &mut egui::Ui, r: Rect, st: &mut HeadSettings, local: &mut Local, shared: &Arc<Shared>, target: f32, ceiling: f32) {
    let s = local.snap.clone();
    // top bar
    let top = Rect::from_min_size(r.min, vec2(r.width(), 36.0));
    ui.painter().rect_filled(top, 0.0, PANEL);
    ui.scope_builder(UiBuilder::new().max_rect(top.shrink2(vec2(12.0, 5.0))).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        ui.label(egui::RichText::new("HEADROOM").font(sora(15.0)).color(ACCENT));
        ui.add_space(14.0);
        target_menu(ui, st, st.name().to_string(), 240.0);
        let down = match turned_down(st, s.integrated) {
            Some(d) if d > 0.0 => format!("{}: \u{2212}{d:.1} dB", st.name()),
            Some(_) => "quieter than target".to_string(),
            None => String::new(),
        };
        ui.label(egui::RichText::new(down).font(mono(11.0)).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            mode_buttons(ui, st);
            ui.add_space(6.0);
            reset_button(ui, shared, local);
        });
    });
    // numbers
    let nums = Rect::from_min_size(pos2(r.left() + 16.0, top.bottom() + 8.0), vec2(r.width() - 32.0, 44.0));
    let tp = s.tp_max[0].max(s.tp_max[1]);
    let items = [("MOMENTARY", fmt_lufs(s.momentary), TEXT), ("SHORT-TERM", fmt_lufs(s.short_term), int_color(s.short_term, target)), ("INTEGRATED", fmt_lufs(s.integrated), int_color(s.integrated, target)), ("LRA", if s.lra > 0.0 { format!("{:.1}", s.lra) } else { "\u{2014}".into() }, TEXT), ("TRUE PEAK", fmt_db(tp, ""), tp_color(tp, ceiling)), ("PLR", if s.plr().is_finite() { format!("{:.1}", s.plr()) } else { "\u{2014}".into() }, TEXT)];
    let cw = nums.width() / items.len() as f32;
    for (k, (l, v, c)) in items.iter().enumerate() {
        let resp = big_number(ui, Rect::from_min_size(nums.min + vec2(k as f32 * cw, 0.0), vec2(cw, 44.0)), l, v, *c, 22.0);
        if k == 2 && resp.double_clicked() {
            shared.reset.store(true, Ordering::Relaxed);
        }
    }
    // whole-song history
    let hist = Rect::from_min_size(pos2(r.left() + 12.0, nums.bottom() + 8.0), vec2(r.width() - 24.0 - 150.0, 170.0));
    let highlight = local.selected_over.and_then(|i| s.over_list.get(i)).map(|o| o.bar_index.saturating_sub(local.first_index) as usize);
    ui.allocate_rect(hist, Sense::hover());
    wave(ui, hist, &local.bars, local.bars.len().max(500), ceiling, target, highlight);
    // big meters to the right of the history
    let mrect = Rect::from_min_size(pos2(hist.right() + 14.0, hist.top()), vec2(124.0, hist.height()));
    let hold_update = [s.momentary, s.short_term, s.tp_max[0], s.tp_max[1]];
    for (k, v) in hold_update.iter().enumerate() {
        if *v > local.held[k] || local.held_t[k] > 2.0 {
            local.held[k] = *v;
            local.held_t[k] = 0.0;
        } else {
            local.held_t[k] += 1.0 / 60.0;
        }
    }
    let defs = [("M", s.momentary, -40.0, 0.0, TEXT), ("S", s.short_term, -40.0, 0.0, TEXT), ("L", s.tp_max[0], -24.0, 3.0, tp_color(s.tp_max[0], ceiling)), ("R", s.tp_max[1], -24.0, 3.0, tp_color(s.tp_max[1], ceiling))];
    for (k, (l, v, lo, hi, c)) in defs.iter().enumerate() {
        let x = mrect.left() + k as f32 * 31.0;
        let col = if *c == TEXT { ACCENT } else { *c };
        bar_meter(ui, Rect::from_min_size(pos2(x, mrect.top()), vec2(22.0, mrect.height())), *v, local.held[k], *lo, *hi, col, l, &format!("{:.0}", if v.is_finite() { *v } else { -99.0 }));
    }
    // lower area: overs list | stats
    let lower = Rect::from_min_max(pos2(r.left() + 12.0, hist.bottom() + 10.0), r.max - vec2(12.0, 14.0));
    let list = Rect::from_min_size(lower.min, vec2(lower.width() * 0.55, lower.height()));
    let stats = Rect::from_min_max(pos2(list.right() + 14.0, lower.top()), lower.max);
    ui.painter().rect_filled(list, 10.0, PANEL);
    ui.painter().text(list.left_top() + vec2(12.0, 8.0), Align2::LEFT_TOP, format!("OVERS ({})   ceiling {ceiling:.0} dBTP", s.overs), sora(10.0), MUTED);
    let inner = Rect::from_min_max(list.min + vec2(8.0, 28.0), list.max - vec2(8.0, 8.0));
    ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if s.over_list.is_empty() {
                ui.label(egui::RichText::new("none yet").font(mono(11.0)).color(MUTED));
            }
            for (i, o) in s.over_list.iter().enumerate().rev() {
                let sel = local.selected_over == Some(i);
                let resp = ui.selectable_label(sel, egui::RichText::new(format_position(o, local.bar_beats)).font(mono(11.0)).color(if sel { ACCENT } else { TEXT }));
                if resp.clicked() {
                    local.selected_over = Some(i);
                }
                if local.scroll_to_overs {
                    resp.scroll_to_me(Some(egui::Align::TOP));
                }
            }
            local.scroll_to_overs = false;
        });
    });
    ui.painter().rect_filled(stats, 10.0, PANEL);
    let rows = [
        ("Integrated", format!("{} LUFS", fmt_lufs(s.integrated))),
        ("LRA", format!("{:.1} LU", s.lra)),
        ("PLR", if s.plr().is_finite() { format!("{:.1} dB", s.plr()) } else { "\u{2014}".into() }),
        ("Max momentary", format!("{} LUFS", fmt_lufs(s.max_momentary))),
        ("Max short-term", format!("{} LUFS", fmt_lufs(s.max_short))),
        ("True peak L / R", format!("{} / {}", fmt_db(s.tp_max[0], ""), fmt_db(s.tp_max[1], ""))),
        ("Sample peak", fmt_db(s.sample_peak, "dBFS")),
        ("Clips", s.clips.to_string()),
        ("Correlation", format!("{:+.2}", s.correlation)),
    ];
    for (k, (l, v)) in rows.iter().enumerate() {
        let y = stats.top() + 10.0 + k as f32 * ((stats.height() - 14.0) / rows.len() as f32);
        ui.painter().text(pos2(stats.left() + 14.0, y), Align2::LEFT_TOP, *l, sora(10.0), MUTED);
        ui.painter().text(pos2(stats.right() - 14.0, y), Align2::RIGHT_TOP, v, mono(11.0), TEXT);
    }
    let _ = ui::GRID;
}

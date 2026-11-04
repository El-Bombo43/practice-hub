//! Snap editor: key/scale and note toggles, a scrolling pitch graph, note readout with cents meter, and the knobs.
use crate::params::*;
use crate::presets::PRESETS;
use crate::scale::{mask_for, NOTE_NAMES};
use crate::Shared;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, pos2, vec2, Align2, Color32, Rect, Sense, Stroke, UiBuilder};
use nih_plug_egui::{create_egui_editor, EguiState};
use phub_common::ui::*;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

const DESIGN_W: f32 = 920.0;
const HISTORY: usize = 360; // 6 s at 60 fps

pub struct Local {
    ed: Editing,
    last: Option<Instant>,
    pub fixed_dt: Option<f32>,
    /// (input MIDI, corrected MIDI), NaN when unvoiced
    hist: VecDeque<(f32, f32)>,
    centre: f32,
    span: f32,
    pan: f32,
    target: Option<i32>,
    last_in: f32,
    in_bar: Ballistics,
    out_bar: Ballistics,
}

pub fn new_local() -> Local {
    Local { ed: Editing::default(), last: None, fixed_dt: None, hist: VecDeque::new(), centre: 62.0, span: 18.0, pan: 0.0, target: None, last_in: f32::NAN, in_bar: Ballistics::default(), out_bar: Ballistics::default() }
}

pub fn create(params: Arc<SnapParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, new_local(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
}

/// Choosing a key or scale writes the 12 toggles to match.
pub fn apply_key_scale(setter: &ParamSetter, p: &SnapParams, key: KeyParam, scale: ScaleParam) {
    set_enum(setter, &p.key, key);
    set_enum(setter, &p.scale, scale);
    if scale != ScaleParam::Custom {
        let m = mask_for(key as usize, scale as usize);
        for (b, on) in p.toggle_params().into_iter().zip(m) {
            if b.value() != on {
                set_bool(setter, b, on);
            }
        }
    }
}

/// Clicking a toggle sets Scale to Custom and leaves Key alone.
pub fn click_toggle(setter: &ParamSetter, p: &SnapParams, note: usize) {
    let mut m = p.notes();
    m[note] = !m[note];
    set_enum(setter, &p.scale, ScaleParam::Custom);
    for (b, on) in p.toggle_params().into_iter().zip(m) {
        if b.value() != on {
            set_bool(setter, b, on);
        }
    }
}

fn note_label(m: f32) -> (String, f32) {
    let n = m.round() as i32;
    (format!("{}{}", NOTE_NAMES[n.rem_euclid(12) as usize], n.div_euclid(12) - 1), (m - n as f32) * 100.0)
}

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<SnapParams>, shared: &Arc<Shared>) {
    apply_zoom(ctx, &params.editor_state, DESIGN_W);
    let now = Instant::now();
    let dt = local.fixed_dt.unwrap_or_else(|| local.last.map_or(1.0 / 60.0, |l| now.duration_since(l).as_secs_f32())).clamp(0.0, 0.1);
    local.last = Some(now);
    let (ip, op) = (shared.in_peak.take(), shared.out_peak.take());
    let db = |v: f32| 20.0 * v.max(1e-6).log10();
    local.in_bar.update(db(ip).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    local.out_bar.update(db(op).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    let in_m = f32::from_bits(shared.in_midi.load(Ordering::Relaxed));
    let out_m = f32::from_bits(shared.out_midi.load(Ordering::Relaxed));
    let tgt = shared.target.load(Ordering::Relaxed) as i32;
    local.target = if in_m.is_finite() && tgt >= 0 { Some(tgt) } else { None };
    local.last_in = in_m;
    local.hist.push_back((in_m, out_m));
    while local.hist.len() > HISTORY {
        local.hist.pop_front();
    }
    if in_m.is_finite() {
        local.centre += (in_m - local.centre) * (1.0 - (-dt / 1.2).exp());
    }

    egui::TopBottomPanel::top("top").exact_height(34.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("SNAP").font(sora(15.0)).color(ACCENT));
            ui.add_space(10.0);
            egui::ComboBox::from_id_salt("preset").selected_text("Preset").width(150.0).show_ui(ui, |ui| {
                for pr in PRESETS {
                    if ui.selectable_label(false, pr.name).clicked() {
                        apply_key_scale(setter, params, pr.key, pr.scale);
                        set_float(setter, &params.retune, pr.retune);
                        set_float(setter, &params.humanize, pr.humanize);
                        set_float(setter, &params.flex, pr.flex);
                        set_float(setter, &params.amount, pr.amount);
                        if let Some(t) = pr.track {
                            set_enum(setter, &params.mode, if t { ModeParam::Track } else { ModeParam::Mix });
                        }
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Key").font(sora(10.0)).color(MUTED));
            let key = params.key.value();
            egui::ComboBox::from_id_salt("key").selected_text(key_name(key as usize)).width(50.0).show_ui(ui, |ui| {
                for k in KEYS {
                    if ui.selectable_label(k == key, key_name(k as usize)).clicked() {
                        let sc = params.scale.value();
                        apply_key_scale(setter, params, k, if sc == ScaleParam::Custom { ScaleParam::Chromatic } else { sc });
                    }
                }
            });
            ui.label(egui::RichText::new("Scale").font(sora(10.0)).color(MUTED));
            let sc = params.scale.value();
            egui::ComboBox::from_id_salt("scale").selected_text(scale_name(sc as usize)).width(130.0).show_ui(ui, |ui| {
                for s in SCALE_LIST {
                    if ui.selectable_label(s == sc, scale_name(s as usize)).clicked() {
                        apply_key_scale(setter, params, key, s);
                    }
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                size_buttons(ui, &params.editor_state, &SIZES);
                ui.add_space(8.0);
                segmented(ui, setter, &params.mode, &[(ModeParam::Mix, "MIX"), (ModeParam::Track, "TRACK")], ACCENT);
            });
        });
    });
    egui::TopBottomPanel::bottom("bottom").exact_height(118.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(14, 6))).show(ctx, |ui| controls(ui, setter, params, local));
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(0))).show(ctx, |ui| {
        let full = ui.available_rect_before_wrap();
        let strip = Rect::from_min_size(full.min, vec2(64.0, full.height()));
        let side = Rect::from_min_max(pos2(full.right() - 190.0, full.top()), full.max);
        let graph = Rect::from_min_max(pos2(strip.right(), full.top()), pos2(side.left(), full.bottom()));
        note_strip(ui, strip, setter, params, local);
        pitch_graph(ui, graph, params, local);
        readout(ui, side, local);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

fn note_strip(ui: &mut egui::Ui, r: Rect, setter: &ParamSetter, p: &Arc<SnapParams>, local: &Local) {
    let notes = p.notes();
    let row_h = (r.height() - 16.0) / 12.0;
    for (row, note) in (0..12).rev().enumerate() {
        let rr = Rect::from_min_size(pos2(r.left() + 8.0, r.top() + 8.0 + row as f32 * row_h), vec2(r.width() - 12.0, row_h - 3.0));
        let on = notes[note];
        let active = local.target.map_or(false, |t| t.rem_euclid(12) as usize == note);
        let resp = ui.allocate_rect(rr, Sense::click());
        let pulse = if active { 0.7 + 0.3 * ((ui.input(|i| i.time) * 6.0).sin() as f32 * 0.5 + 0.5) } else { 1.0 };
        let fill = if on { ACCENT.gamma_multiply(if active { pulse } else { 0.55 }) } else { INSET };
        ui.painter().rect(rr, 6.0, fill, Stroke::new(if resp.hovered() { 1.0_f32 } else { 0.0_f32 }, TEXT), egui::StrokeKind::Inside);
        ui.painter().text(rr.center(), Align2::CENTER_CENTER, NOTE_NAMES[note], sora(11.0), if on { Color32::BLACK } else { MUTED });
        if resp.clicked() {
            click_toggle(setter, p, note);
        }
    }
}

fn pitch_graph(ui: &mut egui::Ui, r: Rect, p: &Arc<SnapParams>, local: &mut Local) {
    let resp = ui.allocate_rect(r, Sense::click_and_drag());
    if resp.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y + i.raw_scroll_delta.x);
        if scroll != 0.0 {
            local.span = (local.span * (1.0 - scroll * 0.002)).clamp(6.0, 36.0);
        }
    }
    if resp.dragged() {
        local.pan += resp.drag_delta().y / r.height() * local.span;
    }
    if resp.double_clicked() {
        local.pan = 0.0;
        local.span = 18.0;
    }
    let notes = p.notes();
    let painter = ui.painter_at(r);
    painter.rect_filled(r, 0.0, BG);
    let mid = local.centre + local.pan;
    let (lo, hi) = (mid - local.span * 0.5, mid + local.span * 0.5);
    let y_of = |m: f32| r.bottom() - (m - lo) / (hi - lo) * r.height();
    let first = lo.floor() as i32;
    for n in first..=(hi.ceil() as i32) {
        let y = y_of(n as f32);
        let on = notes[n.rem_euclid(12) as usize];
        let band = Rect::from_min_max(pos2(r.left(), y_of(n as f32 + 0.5)), pos2(r.right(), y_of(n as f32 - 0.5)));
        let active = local.target == Some(n);
        if on {
            painter.rect_filled(band, 0.0, ACCENT.gamma_multiply(if active { 0.16 } else { 0.07 }));
        }
        painter.line_segment([pos2(r.left(), y), pos2(r.right(), y)], Stroke::new(1.0_f32, if on { ACCENT.gamma_multiply(0.35) } else { GRID }));
        painter.text(pos2(r.left() + 6.0, y - 1.0), Align2::LEFT_BOTTOM, format!("{}{}", NOTE_NAMES[n.rem_euclid(12) as usize], n.div_euclid(12) - 1), mono(9.0), if on { MUTED } else { WIRE });
    }
    let n = local.hist.len();
    let step = r.width() / HISTORY as f32;
    let x_of = |k: usize| r.right() - (n - k) as f32 * step;
    // faint fill where corrected and input differ, then the two lines (gaps where unvoiced)
    let mut prev: Option<(f32, f32, f32)> = None;
    let mut in_run: Vec<egui::Pos2> = Vec::new();
    let mut out_run: Vec<egui::Pos2> = Vec::new();
    let mut flush = |painter: &egui::Painter, a: &mut Vec<egui::Pos2>, b: &mut Vec<egui::Pos2>| {
        if a.len() > 1 {
            painter.add(epaint::Shape::line(std::mem::take(a), Stroke::new(1.0_f32, MUTED)));
        }
        if b.len() > 1 {
            painter.add(epaint::Shape::line(std::mem::take(b), Stroke::new(2.0_f32, ACCENT)));
        }
        a.clear();
        b.clear();
    };
    for (k, (im, om)) in local.hist.iter().enumerate() {
        if im.is_finite() && om.is_finite() {
            let x = x_of(k);
            let (yi, yo) = (y_of(*im), y_of(*om));
            if let Some((px, pyi, pyo)) = prev {
                let mut mesh = epaint::Mesh::default();
                let c = ACCENT.gamma_multiply(0.12);
                for q in [pos2(px, pyi), pos2(x, yi), pos2(x, yo), pos2(px, pyo)] {
                    mesh.colored_vertex(q, c);
                }
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(epaint::Shape::mesh(mesh));
            }
            prev = Some((x, yi, yo));
            in_run.push(pos2(x, yi));
            out_run.push(pos2(x, yo));
        } else {
            prev = None;
            flush(&painter, &mut in_run, &mut out_run);
        }
    }
    flush(&painter, &mut in_run, &mut out_run);
}

fn readout(ui: &mut egui::Ui, r: Rect, local: &Local) {
    let painter = ui.painter();
    painter.rect_filled(r, 0.0, PANEL);
    let c = r.center_top() + vec2(0.0, 52.0);
    if local.last_in.is_finite() {
        let (name, cents) = note_label(local.last_in);
        painter.text(c, Align2::CENTER_CENTER, name, sora(40.0), TEXT);
        painter.text(c + vec2(0.0, 40.0), Align2::CENTER_CENTER, format!("{:+.0} \u{a2}", cents), mono(16.0), if cents.abs() <= 10.0 { GREEN } else { MUTED });
        // centre-zero cents meter
        let m = Rect::from_center_size(c + vec2(0.0, 78.0), vec2(150.0, 14.0));
        painter.rect_filled(m, 7.0, INSET);
        let good = Rect::from_center_size(m.center(), vec2(m.width() * 0.2, m.height()));
        painter.rect_filled(good, 3.0, GREEN.gamma_multiply(0.25));
        let x = m.center().x + cents.clamp(-50.0, 50.0) / 50.0 * m.width() * 0.5;
        painter.line_segment([pos2(x, m.top() - 3.0), pos2(x, m.bottom() + 3.0)], Stroke::new(3.0_f32, if cents.abs() <= 10.0 { GREEN } else { ACCENT }));
        painter.line_segment([pos2(m.center().x, m.top()), pos2(m.center().x, m.bottom())], Stroke::new(1.0_f32, MUTED));
    } else {
        painter.text(c, Align2::CENTER_CENTER, "--", sora(40.0), WIRE);
    }
    painter.text(c + vec2(0.0, 108.0), Align2::CENTER_CENTER, "input pitch (before correction)", sora(9.0), MUTED);
    for (k, (b, name)) in [(local.in_bar, "IN"), (local.out_bar, "OUT")].into_iter().enumerate() {
        let x = r.center().x - 24.0 + k as f32 * 48.0;
        let bar = Rect::from_min_max(pos2(x - 6.0, r.bottom() - 120.0), pos2(x + 6.0, r.bottom() - 28.0));
        meter_bar(painter, bar, b.level / 60.0, b.hold / 60.0, ACCENT, false);
        meter_label(painter, x, bar.bottom() + 3.0, name, "");
    }
}

fn controls(ui: &mut egui::Ui, setter: &ParamSetter, p: &Arc<SnapParams>, local: &mut Local) {
    let mut ed = std::mem::take(&mut local.ed);
    let r = ui.max_rect();
    ui.scope_builder(UiBuilder::new().max_rect(r).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        knob(ui, setter, &p.retune, "RETUNE", ACCENT, 60.0, &mut ed);
        ui.add_space(8.0);
        knob(ui, setter, &p.humanize, "Humanize", ACCENT, 34.0, &mut ed);
        knob(ui, setter, &p.flex, "Flex", ACCENT, 34.0, &mut ed);
        knob(ui, setter, &p.amount, "Amount", ACCENT, 34.0, &mut ed);
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Voice").font(sora(10.0)).color(MUTED));
            ui.horizontal(|ui| segmented(ui, setter, &p.voice, &[(VoiceParam::High, "High"), (VoiceParam::Mid, "Mid"), (VoiceParam::Low, "Low")], ACCENT));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            knob(ui, setter, &p.out_gain, "Output", ACCENT, 34.0, &mut ed);
            knob(ui, setter, &p.mix, "Mix", ACCENT, 34.0, &mut ed);
        });
    });
    local.ed = ed;
}

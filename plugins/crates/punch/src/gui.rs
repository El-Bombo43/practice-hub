//! Punch editor: a scrolling waveform coloured by what Punch is doing, big centre-zero Attack and Sustain knobs.
use crate::params::*;
use crate::presets::PRESETS;
use crate::Shared;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Color32, Rect, Sense, Stroke, UiBuilder};
use nih_plug_egui::{create_egui_editor, EguiState};
use phub_common::ui::*;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

const DESIGN_W: f32 = 660.0;
const HISTORY: usize = 240; // ~4 s at 60 fps (the spec asks for ~3 s: the strip shows the last 180)
const SHOWN: usize = 180;

pub struct Local {
    ed: Editing,
    last: Option<Instant>,
    pub fixed_dt: Option<f32>,
    /// (input peak, output peak, signed gain dB)
    hist: VecDeque<[f32; 3]>,
    in_bar: Ballistics,
    out_bar: Ballistics,
}

pub fn new_local() -> Local {
    Local { ed: Editing::default(), last: None, fixed_dt: None, hist: VecDeque::new(), in_bar: Ballistics::default(), out_bar: Ballistics::default() }
}

pub fn create(params: Arc<PunchParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, new_local(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
}

pub fn apply_settings(setter: &ParamSetter, p: &PunchParams, s: &crate::dsp::Settings) {
    set_float(setter, &p.attack, s.attack * 100.0);
    set_float(setter, &p.sustain, s.sustain * 100.0);
    set_float(setter, &p.attack_len, s.attack_len_ms);
    set_float(setter, &p.sustain_len, s.sustain_len_ms);
    set_float(setter, &p.focus, s.focus);
    set_bool(setter, &p.link, s.link);
    set_bool(setter, &p.clip, s.clip);
    set_float(setter, &p.out_gain, s.out_gain_db);
    set_float(setter, &p.mix, s.mix * 100.0);
}

fn db(v: f32) -> f32 {
    20.0 * v.max(1e-6).log10()
}

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<PunchParams>, shared: &Arc<Shared>) {
    apply_zoom(ctx, &params.editor_state, DESIGN_W);
    let now = Instant::now();
    let dt = local.fixed_dt.unwrap_or_else(|| local.last.map_or(1.0 / 60.0, |l| now.duration_since(l).as_secs_f32())).clamp(0.0, 0.1);
    local.last = Some(now);
    let (ip, op, up, down) = (shared.in_peak.take(), shared.out_peak.take(), shared.gain_up.take(), shared.gain_down.take());
    let g = if up >= down { up } else { -down };
    local.in_bar.update(db(ip).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    local.out_bar.update(db(op).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    local.hist.push_back([ip, op, g]);
    while local.hist.len() > HISTORY {
        local.hist.pop_front();
    }

    egui::TopBottomPanel::top("top").exact_height(34.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("PUNCH").font(sora(15.0)).color(ACCENT));
            ui.add_space(12.0);
            egui::ComboBox::from_id_salt("preset").selected_text("Preset").width(150.0).show_ui(ui, |ui| {
                for p in PRESETS {
                    if ui.selectable_label(false, p.name).clicked() {
                        apply_settings(setter, params, &(p.settings)());
                    }
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| size_buttons(ui, &params.editor_state, &SIZES));
        });
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(8))).show(ctx, |ui| {
        let full = ui.available_rect_before_wrap();
        let wave = Rect::from_min_size(full.min, vec2(full.width() - 46.0, 130.0));
        waveform(ui, wave, local);
        let bars = Rect::from_min_size(pos2(wave.right() + 6.0, wave.top()), vec2(40.0, wave.height()));
        for (k, (b, name)) in [(local.in_bar, "IN"), (local.out_bar, "OUT")].into_iter().enumerate() {
            let x = bars.left() + 10.0 + k as f32 * 20.0;
            let bar = Rect::from_min_max(pos2(x - 5.0, bars.top() + 2.0), pos2(x + 5.0, bars.bottom() - 22.0));
            meter_bar(ui.painter(), bar, b.level / 60.0, b.hold / 60.0, ACCENT, false);
            meter_label(ui.painter(), x, bar.bottom() + 3.0, name, "");
        }
        let body = Rect::from_min_max(pos2(full.left(), wave.bottom() + 8.0), full.max);
        controls(ui, body, setter, params, local);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

fn waveform(ui: &mut egui::Ui, r: Rect, local: &Local) {
    let p = ui.painter();
    p.rect_filled(r, 12.0, PANEL);
    let inner = r.shrink2(vec2(10.0, 8.0));
    let mid = inner.center().y;
    p.line_segment([pos2(inner.left(), mid), pos2(inner.right(), mid)], Stroke::new(1.0_f32, GRID));
    let n = local.hist.len().min(SHOWN);
    let step = inner.width() / SHOWN as f32;
    let skip = local.hist.len() - n;
    for (k, h) in local.hist.iter().skip(skip).enumerate() {
        let x = inner.right() - (n - k) as f32 * step;
        let amp = |v: f32| db_frac(db(v), -60.0) * inner.height() * 0.5;
        let a_out = amp(h[1]);
        let g = h[2];
        let inten = (g.abs() / 12.0).clamp(0.0, 1.0);
        let col = if g.abs() < 0.3 { Color32::from_rgb(0x70, 0x70, 0x78) } else if g > 0.0 { ACCENT.gamma_multiply(0.35 + 0.65 * inten) } else { RED.gamma_multiply(0.35 + 0.65 * inten) };
        p.line_segment([pos2(x, mid - a_out), pos2(x, mid + a_out)], Stroke::new(step.max(1.0), col));
        let a_in = amp(h[0]);
        p.line_segment([pos2(x, mid - a_in), pos2(x, mid - a_in - 1.0)], Stroke::new(step.max(1.0), MUTED));
        p.line_segment([pos2(x, mid + a_in), pos2(x, mid + a_in + 1.0)], Stroke::new(step.max(1.0), MUTED));
    }
}

fn controls(ui: &mut egui::Ui, body: Rect, setter: &ParamSetter, p: &Arc<PunchParams>, local: &mut Local) {
    let mut ed = std::mem::take(&mut local.ed);
    let big = Rect::from_min_size(body.min, vec2(body.width(), body.height() * 0.55));
    let small = Rect::from_min_max(pos2(body.left(), big.bottom()), body.max);
    ui.scope_builder(UiBuilder::new().max_rect(big).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        let w = (ui.available_width() - 2.0 * 110.0) / 2.0;
        ui.add_space(w);
        knob_ex(ui, setter, &p.attack, "ATTACK", ACCENT, 64.0, &mut ed, Some(RED));
        ui.add_space(30.0);
        knob_ex(ui, setter, &p.sustain, "SUSTAIN", ACCENT, 64.0, &mut ed, Some(RED));
    });
    ui.scope_builder(UiBuilder::new().max_rect(small).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        knob(ui, setter, &p.attack_len, "Atk Len", ACCENT, 26.0, &mut ed);
        knob(ui, setter, &p.sustain_len, "Sus Len", ACCENT, 26.0, &mut ed);
        knob(ui, setter, &p.focus, "Focus", ACCENT, 26.0, &mut ed);
        ui.vertical(|ui| {
            toggle(ui, setter, &p.link, "Link", ACCENT);
            toggle(ui, setter, &p.clip, "Clip", ACCENT);
        });
        knob(ui, setter, &p.mix, "Mix", ACCENT, 26.0, &mut ed);
        knob(ui, setter, &p.out_gain, "Out", ACCENT, 26.0, &mut ed);
    });
    local.ed = ed;
    let _ = Sense::hover();
}

//! Vox Strip editor: level history on top, then one panel per module in signal order.
use crate::params::*;
use crate::presets::PRESETS;
use crate::Shared;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Align2, Color32, Rect, Sense, Stroke, UiBuilder};
use nih_plug_egui::{create_egui_editor, EguiState};
use phub_common::ui::*;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

const DESIGN_W: f32 = 960.0;
const HISTORY: usize = 300; // ~5 s at 60 fps

pub struct Local {
    ed: Editing,
    last: Option<Instant>,
    pub fixed_dt: Option<f32>,
    hist: VecDeque<[f32; 3]>,
    in_bar: Ballistics,
    out_bar: Ballistics,
    ds_bar: Ballistics,
    comp_bar: Ballistics,
    gate_open: bool,
}

pub fn new_local() -> Local {
    Local { ed: Editing::default(), last: None, fixed_dt: None, hist: VecDeque::new(), in_bar: Ballistics::default(), out_bar: Ballistics::default(), ds_bar: Ballistics::default(), comp_bar: Ballistics::default(), gate_open: false }
}

pub fn create(params: Arc<VoxParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, new_local(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
}

pub fn apply_settings(setter: &ParamSetter, p: &VoxParams, s: &crate::dsp::Settings) {
    set_float(setter, &p.in_gain, s.in_gain_db);
    set_bool(setter, &p.gate_on, s.on[0]);
    set_float(setter, &p.gate_thresh, s.gate_thresh);
    set_float(setter, &p.gate_range, s.gate_range);
    set_float(setter, &p.gate_attack, s.gate_attack_ms);
    set_float(setter, &p.gate_release, s.gate_release_ms);
    set_bool(setter, &p.hpf_on, s.on[1]);
    set_float(setter, &p.hpf_freq, s.hpf_freq);
    set_enum(setter, &p.hpf_slope, if s.hpf_24 { SlopeParam::S24 } else { SlopeParam::S12 });
    set_bool(setter, &p.ds_on, s.on[2]);
    set_float(setter, &p.ds_freq, s.ds_freq);
    set_float(setter, &p.ds_thresh, s.ds_thresh);
    set_float(setter, &p.ds_range, s.ds_range);
    set_enum(setter, &p.ds_mode, if s.ds_wide { DsMode::Wide } else { DsMode::Split });
    set_bool(setter, &p.comp_on, s.on[3]);
    set_enum(setter, &p.comp_style, if s.comp_opto { CompStyle::Opto } else { CompStyle::Clean });
    set_float(setter, &p.comp_thresh, s.comp_thresh);
    set_float(setter, &p.comp_ratio, s.comp_ratio);
    set_float(setter, &p.comp_attack, s.comp_attack_ms);
    set_float(setter, &p.comp_release, s.comp_release_ms);
    set_float(setter, &p.comp_knee, s.comp_knee);
    set_float(setter, &p.comp_makeup, s.comp_makeup);
    set_bool(setter, &p.comp_auto, s.comp_auto);
    set_bool(setter, &p.sat_on, s.on[4]);
    set_enum(setter, &p.sat_type, if s.sat_tube { SatType::Tube } else { SatType::Tape });
    set_float(setter, &p.sat_drive, s.sat_drive * 100.0);
    set_float(setter, &p.sat_mix, s.sat_mix * 100.0);
    set_float(setter, &p.out_gain, s.out_gain_db);
    set_float(setter, &p.mix, s.mix * 100.0);
}

fn db(v: f32) -> f32 {
    20.0 * v.max(1e-6).log10()
}

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<VoxParams>, shared: &Arc<Shared>) {
    apply_zoom(ctx, &params.editor_state, DESIGN_W);
    let now = Instant::now();
    let dt = local.fixed_dt.unwrap_or_else(|| local.last.map_or(1.0 / 60.0, |l| now.duration_since(l).as_secs_f32())).clamp(0.0, 0.1);
    local.last = Some(now);
    let (ip, op, dg, cg) = (shared.in_peak.take(), shared.out_peak.take(), shared.ds_gr.take(), shared.comp_gr.take());
    local.gate_open = shared.gate_open.load(Ordering::Relaxed);
    local.in_bar.update(db(ip).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    local.out_bar.update(db(op).clamp(-60.0, 0.0) + 60.0, dt, 20.0);
    local.ds_bar.update(dg.min(20.0), dt, 20.0);
    local.comp_bar.update(cg.min(20.0), dt, 20.0);
    local.hist.push_back([ip, op, cg]);
    while local.hist.len() > HISTORY {
        local.hist.pop_front();
    }

    egui::TopBottomPanel::top("top").exact_height(34.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            ui.label(egui::RichText::new("VOX STRIP").font(sora(15.0)).color(ACCENT));
            ui.add_space(12.0);
            egui::ComboBox::from_id_salt("preset").selected_text("Preset").width(160.0).show_ui(ui, |ui| {
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
        let hist = Rect::from_min_size(full.min, vec2(full.width(), 96.0));
        history(ui, hist, local);
        let body = Rect::from_min_max(pos2(full.left(), hist.bottom() + 8.0), full.max);
        modules(ui, body, setter, params, shared, local);
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

fn history(ui: &mut egui::Ui, r: Rect, local: &Local) {
    let p = ui.painter();
    p.rect_filled(r, 10.0, PANEL);
    let inner = r.shrink2(vec2(10.0, 6.0));
    let mid = inner.center().y;
    let n = local.hist.len();
    let step = inner.width() / HISTORY as f32;
    for (k, h) in local.hist.iter().enumerate() {
        let x = inner.right() - (n - k) as f32 * step;
        let amp = |v: f32| (db_frac(db(v), -60.0)) * inner.height() * 0.5;
        let (a_in, a_out) = (amp(h[0]), amp(h[1]));
        p.line_segment([pos2(x, mid - a_in), pos2(x, mid + a_in)], Stroke::new(step.max(1.0), WIRE));
        p.line_segment([pos2(x, mid - a_out), pos2(x, mid + a_out)], Stroke::new(step.max(1.0), ACCENT.gamma_multiply(0.8)));
        if h[2] > 0.05 {
            p.line_segment([pos2(x, inner.top()), pos2(x, inner.top() + (h[2] / 20.0).min(1.0) * inner.height() * 0.5)], Stroke::new(step.max(1.0), GREEN.gamma_multiply(0.7)));
        }
    }
    p.text(r.left_top() + vec2(14.0, 8.0), Align2::LEFT_TOP, "LEVEL  in · out · comp gain reduction", sora(9.0), MUTED);
}

fn panel(ui: &mut egui::Ui, r: Rect, title: &str, color: Color32, on: Option<(&BoolParam, &ParamSetter)>, content: impl FnOnce(&mut egui::Ui)) {
    ui.painter().rect_filled(r, 14.0, PANEL);
    ui.painter().rect_filled(Rect::from_min_size(r.min + vec2(14.0, 0.0), vec2(r.width() - 28.0, 3.0)), 1.5, color);
    let is_on = on.map_or(true, |(p, _)| p.value());
    ui.scope_builder(UiBuilder::new().max_rect(r.shrink2(vec2(8.0, 8.0))).layout(egui::Layout::top_down(egui::Align::Center)), |ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 3.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(title).font(sora(11.0)).color(color));
            if let Some((p, setter)) = on {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 18.0), Sense::click());
                    ui.painter().rect(rect, 9.0, if is_on { color } else { INSET }, Stroke::new(1.0_f32, if is_on { color } else { WIRE }), egui::StrokeKind::Inside);
                    ui.painter().circle_filled(pos2(if is_on { rect.right() - 9.0 } else { rect.left() + 9.0 }, rect.center().y), 5.5, if is_on { Color32::BLACK } else { MUTED });
                    if resp.clicked() {
                        set_bool(setter, p, !is_on);
                    }
                });
            }
        });
        if !is_on {
            ui.set_opacity(0.4);
        }
        content(ui);
    });
}

fn modules(ui: &mut egui::Ui, body: Rect, setter: &ParamSetter, p: &Arc<VoxParams>, shared: &Arc<Shared>, local: &mut Local) {
    let widths = [84.0, 138.0, 108.0, 168.0, 218.0, 128.0, 112.0];
    let gap = 6.0;
    let total: f32 = widths.iter().sum::<f32>() + gap * 6.0;
    let scale = body.width() / total;
    let mut x = body.left();
    let mut rect = |i: usize| {
        let w = widths[i] * scale;
        let r = Rect::from_min_size(pos2(x, body.top()), vec2(w, body.height()));
        x += w + gap * scale;
        r
    };
    let k = 28.0;
    let mut ed = std::mem::take(&mut local.ed);

    // IN
    let r = rect(0);
    panel(ui, r, "IN", MUTED, None, |ui| {
        knob(ui, setter, &p.in_gain, "Input", ACCENT, k, &mut ed);
        let (bar, _) = ui.allocate_exact_size(vec2(20.0, (ui.available_height() - 28.0).max(20.0)), Sense::hover());
        meter_bar(ui.painter(), bar, local.in_bar.level / 60.0, local.in_bar.hold / 60.0, ACCENT, false);
        meter_label(ui.painter(), bar.center().x, bar.bottom() + 3.0, "IN", &format!("{:.0}", local.in_bar.hold - 60.0));
    });
    // GATE
    let r = rect(1);
    panel(ui, r, "GATE", RED, Some((&p.gate_on, setter)), |ui| {
        ui.horizontal(|ui| {
            knob(ui, setter, &p.gate_thresh, "Threshold", RED, k, &mut ed);
            knob(ui, setter, &p.gate_range, "Range", RED, k, &mut ed);
        });
        ui.horizontal(|ui| {
            knob(ui, setter, &p.gate_attack, "Attack", RED, k, &mut ed);
            knob(ui, setter, &p.gate_release, "Release", RED, k, &mut ed);
        });
        let (rect, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, if local.gate_open { GREEN } else { Color32::from_rgb(0x3a, 0x20, 0x20) });
    });
    // HPF
    let r = rect(2);
    panel(ui, r, "HPF", ORANGE, Some((&p.hpf_on, setter)), |ui| {
        knob(ui, setter, &p.hpf_freq, "Freq", ORANGE, k + 6.0, &mut ed);
        ui.horizontal(|ui| {
            for (e, n) in [(SlopeParam::S12, "12"), (SlopeParam::S24, "24")] {
                let on = p.hpf_slope.value() == e;
                let b = egui::Button::new(egui::RichText::new(n).font(sora(11.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { ORANGE } else { INSET }).min_size(vec2(36.0, 22.0));
                if ui.add(b).on_hover_text("dB/oct").clicked() {
                    set_enum(setter, &p.hpf_slope, e);
                }
            }
        });
    });
    // DE-ESS
    let r = rect(3);
    panel(ui, r, "DE-ESS", YELLOW, Some((&p.ds_on, setter)), |ui| {
        ui.horizontal(|ui| {
            knob(ui, setter, &p.ds_freq, "Freq", YELLOW, k, &mut ed);
            knob(ui, setter, &p.ds_thresh, "Threshold", YELLOW, k, &mut ed);
        });
        ui.horizontal(|ui| {
            knob(ui, setter, &p.ds_range, "Range", YELLOW, k, &mut ed);
            let (bar, _) = ui.allocate_exact_size(vec2(16.0, 54.0), Sense::hover());
            meter_bar(ui.painter(), bar, local.ds_bar.level / 20.0, local.ds_bar.hold / 20.0, YELLOW, true);
        });
        ui.horizontal(|ui| {
            for (e, n) in [(DsMode::Split, "Split"), (DsMode::Wide, "Wide")] {
                let on = p.ds_mode.value() == e;
                let b = egui::Button::new(egui::RichText::new(n).font(sora(10.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { YELLOW } else { INSET }).min_size(vec2(44.0, 20.0));
                if ui.add(b).clicked() {
                    set_enum(setter, &p.ds_mode, e);
                }
            }
            let listen = shared.listen.load(Ordering::Relaxed);
            let b = egui::Button::new(egui::RichText::new("Listen").font(sora(10.0)).color(if listen { Color32::BLACK } else { TEXT })).fill(if listen { YELLOW } else { INSET }).min_size(vec2(44.0, 20.0));
            if ui.add(b).on_hover_text("Hear only the detector band (not saved)").clicked() {
                shared.listen.store(!listen, Ordering::Relaxed);
            }
        });
    });
    // COMP
    let r = rect(4);
    panel(ui, r, "COMP", GREEN, Some((&p.comp_on, setter)), |ui| {
        ui.horizontal(|ui| {
            segmented(ui, setter, &p.comp_style, &[(CompStyle::Clean, "Clean"), (CompStyle::Opto, "Opto")], GREEN);
            toggle(ui, setter, &p.comp_auto, "Auto", GREEN);
        });
        ui.horizontal(|ui| {
            knob(ui, setter, &p.comp_thresh, "Threshold", GREEN, k, &mut ed);
            knob(ui, setter, &p.comp_ratio, "Ratio", GREEN, k, &mut ed);
            knob(ui, setter, &p.comp_knee, "Knee", GREEN, k, &mut ed);
        });
        ui.horizontal(|ui| {
            knob(ui, setter, &p.comp_attack, "Attack", GREEN, k, &mut ed);
            knob(ui, setter, &p.comp_release, "Release", GREEN, k, &mut ed);
            knob(ui, setter, &p.comp_makeup, "Makeup", GREEN, k, &mut ed);
        });
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width().min(150.0), 10.0), Sense::hover());
        // horizontal GR bar: filled from the left
        ui.painter().rect_filled(bar, 3.0, INSET);
        let w = bar.width() * (local.comp_bar.level / 20.0).clamp(0.0, 1.0);
        ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(w, bar.height())), 3.0, GREEN);
    });
    // SAT
    let r = rect(5);
    panel(ui, r, "SAT", CYAN, Some((&p.sat_on, setter)), |ui| {
        segmented(ui, setter, &p.sat_type, &[(SatType::Tape, "Tape"), (SatType::Tube, "Tube")], CYAN);
        knob(ui, setter, &p.sat_drive, "Drive", CYAN, k + 4.0, &mut ed);
        knob(ui, setter, &p.sat_mix, "Mix", CYAN, k, &mut ed);
    });
    // OUT
    let r = rect(6);
    panel(ui, r, "OUT", MUTED, None, |ui| {
        knob(ui, setter, &p.out_gain, "Output", ACCENT, k, &mut ed);
        knob(ui, setter, &p.mix, "Mix", ACCENT, k, &mut ed);
        let (bar, _) = ui.allocate_exact_size(vec2(20.0, (ui.available_height() - 22.0).max(16.0)), Sense::hover());
        meter_bar(ui.painter(), bar, local.out_bar.level / 60.0, local.out_bar.hold / 60.0, ACCENT, false);
        meter_label(ui.painter(), bar.center().x, bar.bottom() + 3.0, "OUT", &format!("{:.0}", local.out_bar.hold - 60.0));
    });
    local.ed = ed;
}

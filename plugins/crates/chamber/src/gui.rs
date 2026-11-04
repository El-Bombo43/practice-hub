//! The editor: top bar (title, presets, mode, size), the animated space with meters and Freeze, and the knob rows.
use crate::dsp::Settings;
use crate::params::{ChamberParams, DivParam, ModeParam, SIZES};
use crate::presets::PRESETS;
use crate::space::{self, SpaceState};
use crate::Shared;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, pos2, vec2, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Pos2, Rect, Sense, Stroke, UiBuilder};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::sync::Arc;
use std::time::Instant;

const BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
const GRID: Color32 = Color32::from_rgb(0x22, 0x22, 0x26);
const WIRE: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x40);
const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xea);
const MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x90);
const ACCENT: Color32 = Color32::from_rgb(0xd4, 0xff, 0x3a);
const FREEZE: Color32 = Color32::from_rgb(0xbf, 0xe7, 0xff);
const DESIGN_W: f32 = 820.0;

fn sora(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sora".into()))
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono".into()))
}

#[derive(Default, Clone, Copy)]
struct Bar {
    level: f32,
    hold: f32,
    hold_t: f32,
}

pub struct Local {
    editing: Option<(Id, String)>,
    space: SpaceState,
    last: Option<Instant>,
    /// Tests and the offscreen renderer set this to drive the animation deterministically.
    pub fixed_dt: Option<f32>,
    bars: [Bar; 3],
    in_peak: f32,
    wet_level: f32,
}

pub fn new_local() -> Local {
    Local { editing: None, space: SpaceState::default(), last: None, fixed_dt: None, bars: [Bar::default(); 3], in_peak: 0.0, wet_level: 0.0 }
}

pub fn setup(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("sora".into(), Arc::new(FontData::from_static(include_bytes!("../assets/Sora-Regular.ttf"))));
    fonts.font_data.insert("sora-semibold".into(), Arc::new(FontData::from_static(include_bytes!("../assets/Sora-SemiBold.ttf"))));
    fonts.font_data.insert("plex".into(), Arc::new(FontData::from_static(include_bytes!("../assets/IBMPlexMono-Regular.ttf"))));
    fonts.font_data.insert("plex-bold".into(), Arc::new(FontData::from_static(include_bytes!("../assets/IBMPlexMono-Bold.ttf"))));
    fonts.families.insert(FontFamily::Name("sora".into()), vec!["sora".into(), "sora-semibold".into()]);
    fonts.families.insert(FontFamily::Name("mono".into()), vec!["plex".into(), "plex-bold".into()]);
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "sora".into());
    fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "plex".into());
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.override_text_color = Some(TEXT);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.inactive.bg_fill = Color32::from_rgb(0x1c, 0x1c, 0x20);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x1c, 0x1c, 0x20);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x26, 0x26, 0x2c);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.window_corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

pub fn create(params: Arc<ChamberParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, new_local(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
}

// ---------------------------------------------------------------------------------------------
// parameter helpers (each is one host gesture)

fn set_float(setter: &ParamSetter, p: &FloatParam, v: f32) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn set_bool(setter: &ParamSetter, p: &BoolParam, v: bool) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn set_enum<E: Enum + PartialEq>(setter: &ParamSetter, p: &EnumParam<E>, v: E) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
fn reset_param<P: Param>(setter: &ParamSetter, p: &P) {
    setter.begin_set_parameter(p);
    setter.set_parameter_normalized(p, p.default_normalized_value());
    setter.end_set_parameter(p);
}

const DIVS: [DivParam; 10] = [DivParam::D64, DivParam::D32T, DivParam::D32, DivParam::D16T, DivParam::D16, DivParam::D16D, DivParam::D8T, DivParam::D8, DivParam::D8D, DivParam::D4];

/// Apply a whole preset, one host gesture per parameter.
pub fn apply_settings(setter: &ParamSetter, p: &ChamberParams, s: &Settings) {
    set_enum(setter, &p.mode, [ModeParam::Plate, ModeParam::Hall, ModeParam::Room][s.mode.min(2)]);
    set_float(setter, &p.mix, s.mix * 100.0);
    set_float(setter, &p.predelay, s.predelay_ms);
    set_bool(setter, &p.predelay_sync, s.predelay_sync);
    set_enum(setter, &p.predelay_div, DIVS[s.predelay_div.min(9)]);
    set_float(setter, &p.size, s.size * 100.0);
    set_float(setter, &p.decay, s.decay_s);
    set_float(setter, &p.diffusion, s.diffusion * 100.0);
    set_float(setter, &p.early, s.early_db);
    set_float(setter, &p.low_cut, s.low_cut);
    set_float(setter, &p.high_cut, s.high_cut);
    set_float(setter, &p.damping, s.damping_hz);
    set_float(setter, &p.bass_decay, s.bass_decay);
    set_float(setter, &p.mod_rate, s.mod_rate);
    set_float(setter, &p.mod_depth, s.mod_depth * 100.0);
    set_float(setter, &p.width, s.width * 100.0);
    set_float(setter, &p.duck_amt, s.duck_db);
    set_float(setter, &p.duck_thresh, s.duck_thresh_db);
    set_float(setter, &p.duck_release, s.duck_release_ms);
    set_bool(setter, &p.freeze, s.freeze);
    set_float(setter, &p.out_gain, s.out_gain_db);
}

// ---------------------------------------------------------------------------------------------

fn db_of(v: f32) -> f32 {
    20.0 * (v.max(1e-6)).log10()
}

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<ChamberParams>, shared: &Arc<Shared>) {
    let (w, _) = params.editor_state.size();
    ctx.set_zoom_factor((w as f32 / DESIGN_W).clamp(0.5, 2.5));

    // animation clock and meter ballistics: peak hold 1 s, then fall at ~20 dB/s
    let now = Instant::now();
    let dt = local.fixed_dt.unwrap_or_else(|| local.last.map_or(1.0 / 60.0, |l| now.duration_since(l).as_secs_f32())).clamp(0.0, 0.1);
    local.last = Some(now);
    let in_pk = Shared::take(&shared.in_peak);
    let wet_pk = Shared::take(&shared.wet_peak);
    let gr = Shared::take(&shared.gr_db);
    local.in_peak = in_pk;
    local.wet_level = Shared::get(&shared.wet_level);
    let vals = [db_of(in_pk), db_of(wet_pk), -gr];
    for (k, b) in local.bars.iter_mut().enumerate() {
        let v = if k == 2 { -vals[2] } else { vals[k] }; // GR is kept as a positive number of dB
        let v = if k == 2 { v } else { v.clamp(-60.0, 0.0) + 60.0 }; // 0..60 for level bars, 0..30 for GR
        b.level = if v > b.level { v } else { (b.level - 20.0 * dt).max(v).max(0.0) };
        if v >= b.hold {
            b.hold = v;
            b.hold_t = 0.0;
        } else {
            b.hold_t += dt;
            if b.hold_t > 1.0 {
                b.hold = (b.hold - 20.0 * dt).max(b.level);
            }
        }
    }
    let inputs = space::Inputs {
        mode: params.mode.value() as usize,
        size: params.size.value() / 100.0,
        decay_s: params.decay.value(),
        width: params.width.value() / 100.0,
        damping_hz: params.damping.value(),
        high_cut_hz: params.high_cut.value(),
        mod_rate: params.mod_rate.value(),
        mod_depth: params.mod_depth.value() / 100.0,
        duck_gr_db: local.bars[2].level,
        freeze: params.freeze.value(),
        wet_level: local.wet_level,
        in_peak: local.in_peak,
    };
    local.space.step(dt, &inputs);

    egui::TopBottomPanel::top("top").exact_height(34.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| top_bar(ui, setter, params));
    egui::TopBottomPanel::bottom("bottom").exact_height(200.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 6))).show(ctx, |ui| knob_rows(ui, setter, params, local));
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin::same(0))).show(ctx, |ui| {
        let full = ui.available_rect_before_wrap();
        let split = full.left() + full.width() * 0.68;
        let space_rect = Rect::from_min_max(full.min, pos2(split, full.bottom()));
        let side = Rect::from_min_max(pos2(split, full.top()), full.max);
        let painter = ui.painter_at(space_rect);
        space::draw(&painter, space_rect, &local.space, &inputs);
        ui.scope_builder(UiBuilder::new().max_rect(side.shrink2(vec2(14.0, 10.0))), |ui| side_panel(ui, setter, params, local));
    });
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

// ---------------------------------------------------------------------------------------------
// bars and panels

fn top_bar(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ChamberParams>) {
    ui.horizontal_centered(|ui| {
        ui.label(egui::RichText::new("CHAMBER").font(sora(15.0)).color(ACCENT));
        ui.add_space(12.0);
        egui::ComboBox::from_id_salt("preset").selected_text("Preset").width(150.0).show_ui(ui, |ui| {
            for p in PRESETS {
                if ui.selectable_label(false, p.name).clicked() {
                    apply_settings(setter, params, &(p.settings)());
                }
            }
        });
        ui.add_space(20.0);
        let cur = params.mode.value();
        for (m, name) in [(ModeParam::Plate, "PLATE"), (ModeParam::Hall, "HALL"), (ModeParam::Room, "ROOM")] {
            let on = cur == m;
            let b = egui::Button::new(egui::RichText::new(name).font(sora(11.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { ACCENT } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }).min_size(vec2(62.0, 24.0));
            if ui.add(b).clicked() && !on {
                set_enum(setter, &params.mode, m);
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let cur = params.editor_state.size();
            for (i, name) in ["L", "M", "S"].iter().enumerate() {
                let idx = 2 - i;
                let on = cur == SIZES[idx];
                let b = egui::Button::new(egui::RichText::new(*name).font(sora(12.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { ACCENT } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }).min_size(vec2(28.0, 24.0));
                if ui.add(b).on_hover_text("Window size").clicked() && !on {
                    params.editor_state.set_requested_size(SIZES[idx]);
                }
            }
        });
    });
}

fn side_panel(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ChamberParams>, local: &mut Local) {
    let r = ui.max_rect();
    let freeze_h = 54.0;
    let meters = Rect::from_min_max(r.min, pos2(r.right(), r.bottom() - freeze_h - 12.0));
    let names = ["IN", "WET", "GR"];
    let slot = meters.width() / 3.0;
    let painter = ui.painter();
    for k in 0..3 {
        let x0 = meters.left() + slot * k as f32;
        let bar = Rect::from_min_max(pos2(x0 + slot * 0.3, meters.top() + 6.0), pos2(x0 + slot * 0.7, meters.bottom() - 22.0));
        painter.rect_filled(bar, 3.0, Color32::from_rgb(0x1c, 0x1c, 0x20));
        let b = local.bars[k];
        let (range, fill) = if k == 2 { (30.0, ACCENT.gamma_multiply(0.0)) } else { (60.0, ACCENT) };
        let h = bar.height();
        let color = if k == 2 { Color32::from_rgb(0xff, 0x9f, 0x43) } else { fill };
        if k == 2 {
            // gain reduction hangs from the top
            let y = bar.top() + h * (b.level / range).clamp(0.0, 1.0);
            painter.rect_filled(Rect::from_min_max(bar.min, pos2(bar.right(), y)), 3.0, color);
            let yh = bar.top() + h * (b.hold / range).clamp(0.0, 1.0);
            painter.line_segment([pos2(bar.left(), yh), pos2(bar.right(), yh)], Stroke::new(1.5_f32, TEXT));
        } else {
            let y = bar.bottom() - h * (b.level / range).clamp(0.0, 1.0);
            painter.rect_filled(Rect::from_min_max(pos2(bar.left(), y), bar.max), 3.0, color);
            let yh = bar.bottom() - h * (b.hold / range).clamp(0.0, 1.0);
            painter.line_segment([pos2(bar.left(), yh), pos2(bar.right(), yh)], Stroke::new(1.5_f32, TEXT));
        }
        for db in [-48.0f32, -36.0, -24.0, -12.0] {
            let y = if k == 2 { bar.top() + h * (-db / 2.0 / 30.0).min(1.0) } else { bar.bottom() - h * ((db + 60.0) / 60.0) };
            let _ = y;
        }
        painter.text(pos2(bar.center().x, bar.bottom() + 4.0), Align2::CENTER_TOP, names[k], sora(10.0), MUTED);
        let txt = if k == 2 { format!("{:.0}", -b.hold) } else { format!("{:.0}", b.hold - 60.0) };
        painter.text(pos2(bar.center().x, bar.bottom() + 14.0), Align2::CENTER_TOP, txt, mono(9.0), MUTED);
    }
    // Freeze
    let fr = Rect::from_min_max(pos2(r.left(), r.bottom() - freeze_h), r.max);
    let on = params.freeze.value();
    let resp = ui.allocate_rect(fr, Sense::click());
    let painter = ui.painter();
    painter.rect(fr, 14.0, if on { FREEZE } else { Color32::TRANSPARENT }, Stroke::new(1.5_f32, if on { FREEZE } else { WIRE }), egui::StrokeKind::Inside);
    painter.text(fr.center(), Align2::CENTER_CENTER, "FREEZE", sora(15.0), if on { Color32::BLACK } else if resp.hovered() { TEXT } else { MUTED });
    if resp.clicked() {
        set_bool(setter, &params.freeze, !on);
    }
    let _ = local;
}

fn knob_rows(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ChamberParams>, local: &mut Local) {
    let r = ui.max_rect();
    let right_w = 150.0;
    let rows_rect = Rect::from_min_max(r.min, pos2(r.right() - right_w, r.bottom()));
    let big_rect = Rect::from_min_max(pos2(r.right() - right_w, r.top()), r.max);
    let row_h = r.height() / 3.0;
    let plate = params.mode.value() == ModeParam::Plate;
    for row in 0..3 {
        let rr = Rect::from_min_size(pos2(rows_rect.left(), rows_rect.top() + row_h * row as f32), vec2(rows_rect.width(), row_h));
        ui.scope_builder(UiBuilder::new().max_rect(rr).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let label = |ui: &mut egui::Ui, t: &str| {
                ui.allocate_ui_with_layout(vec2(58.0, row_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.add_space(row_h * 0.35);
                    ui.label(egui::RichText::new(t).font(sora(10.0)).color(MUTED));
                });
            };
            match row {
                0 => {
                    label(ui, "SPACE");
                    predelay_knob(ui, setter, params, local);
                    knob(ui, setter, &params.size, "Size", ACCENT, 30.0, local);
                    knob(ui, setter, &params.decay, "Decay", ACCENT, 30.0, local);
                    knob(ui, setter, &params.diffusion, "Diffusion", ACCENT, 30.0, local);
                    ui.add_enabled_ui(!plate, |ui| knob(ui, setter, &params.early, "Early", ACCENT, 30.0, local));
                }
                1 => {
                    label(ui, "TONE");
                    knob(ui, setter, &params.low_cut, "Low Cut", ACCENT, 30.0, local);
                    knob(ui, setter, &params.high_cut, "High Cut", ACCENT, 30.0, local);
                    knob(ui, setter, &params.damping, "Damping", ACCENT, 30.0, local);
                    knob(ui, setter, &params.bass_decay, "Bass Decay", ACCENT, 30.0, local);
                }
                _ => {
                    label(ui, "MOTION");
                    knob(ui, setter, &params.mod_rate, "Mod Rate", ACCENT, 30.0, local);
                    knob(ui, setter, &params.mod_depth, "Mod Depth", ACCENT, 30.0, local);
                    knob(ui, setter, &params.width, "Width", ACCENT, 30.0, local);
                    ui.add_space(14.0);
                    label(ui, "DUCK");
                    knob(ui, setter, &params.duck_amt, "Amount", ACCENT, 30.0, local);
                    knob(ui, setter, &params.duck_thresh, "Threshold", ACCENT, 30.0, local);
                    knob(ui, setter, &params.duck_release, "Release", ACCENT, 30.0, local);
                }
            }
        });
    }
    // Mix and Output: larger, on the right
    let half = big_rect.height() * 0.5;
    ui.painter().line_segment([pos2(big_rect.left() + 4.0, big_rect.top() + 8.0), pos2(big_rect.left() + 4.0, big_rect.bottom() - 8.0)], Stroke::new(1.0_f32, GRID));
    let a = Rect::from_min_size(pos2(big_rect.left() + 10.0, big_rect.top()), vec2(big_rect.width() - 10.0, half));
    let b = Rect::from_min_size(pos2(big_rect.left() + 10.0, big_rect.top() + half), vec2(big_rect.width() - 10.0, half));
    ui.scope_builder(UiBuilder::new().max_rect(a).layout(egui::Layout::top_down(egui::Align::Center)), |ui| knob(ui, setter, &params.mix, "MIX", ACCENT, 46.0, local));
    ui.scope_builder(UiBuilder::new().max_rect(b).layout(egui::Layout::top_down(egui::Align::Center)), |ui| knob(ui, setter, &params.out_gain, "OUTPUT", ACCENT, 46.0, local));
}

/// Pre-delay: a ms knob, or with the sync toggle on, a knob that steps through note values.
fn predelay_knob(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ChamberParams>, local: &mut Local) {
    let sync = params.predelay_sync.value();
    ui.allocate_ui_with_layout(vec2(68.0, 62.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        if sync {
            note_knob(ui, setter, &params.predelay_div, "Pre-delay", 30.0);
        } else {
            knob(ui, setter, &params.predelay, "Pre-delay", ACCENT, 30.0, local);
        }
    });
    // the sync toggle sits beside the knob
    let (rect, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
    let painter = ui.painter();
    painter.rect(rect, 11.0, if sync { ACCENT } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }, Stroke::new(1.0_f32, if sync { ACCENT } else { WIRE }), egui::StrokeKind::Inside);
    // a drawn eighth note (the fonts have no glyph for it)
    let ink = if sync { Color32::BLACK } else { MUTED };
    let c = rect.center();
    painter.circle_filled(c + vec2(-2.0, 4.0), 2.6, ink);
    painter.line_segment([c + vec2(0.4, 3.6), c + vec2(0.4, -6.0)], Stroke::new(1.4_f32, ink));
    painter.line_segment([c + vec2(0.4, -6.0), c + vec2(4.5, -2.5)], Stroke::new(1.4_f32, ink));
    if resp.on_hover_text("Sync pre-delay to the host tempo").clicked() {
        set_bool(setter, &params.predelay_sync, !sync);
    }
}

// ---------------------------------------------------------------------------------------------
// widgets

fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = 28;
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        center + vec2(a.cos(), a.sin()) * r
    }).collect()
}

fn draw_dial(painter: &egui::Painter, rect: Rect, norm: f32, hot: bool, color: Color32) {
    let c = rect.center();
    let r = rect.width() * 0.5 - 3.0;
    let (a0, a1) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 2.25);
    painter.add(epaint::Shape::line(arc_points(c, r, a0, a1), Stroke::new(3.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x30))));
    painter.add(epaint::Shape::line(arc_points(c, r, a0, a0 + (a1 - a0) * norm.max(0.001)), Stroke::new(3.0_f32, if hot { color } else { color.gamma_multiply(0.85) })));
    let ang = a0 + (a1 - a0) * norm;
    painter.circle_filled(c + vec2(ang.cos(), ang.sin()) * (r - 6.0), 2.5, TEXT);
}

/// A knob with an arc, a value under it (click to type, units accepted), double-click to reset, Shift-drag for fine.
fn knob(ui: &mut egui::Ui, setter: &ParamSetter, p: &FloatParam, label: &str, color: Color32, size: f32, local: &mut Local) {
    let id = Id::new(("knob", p.name()));
    let w = (size + 30.0).max(62.0);
    ui.allocate_ui_with_layout(vec2(w, size + 30.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
        let norm = p.unmodulated_normalized_value();
        draw_dial(ui.painter(), rect, norm, resp.hovered() || resp.dragged(), color);
        if resp.drag_started() {
            setter.begin_set_parameter(p);
        }
        if resp.dragged() {
            let fine = ui.input(|i| i.modifiers.shift);
            let dy = -resp.drag_delta().y;
            let n = (p.unmodulated_normalized_value() + dy / 180.0 * if fine { 0.1 } else { 1.0 }).clamp(0.0, 1.0);
            setter.set_parameter_normalized(p, n);
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(p);
        }
        if resp.double_clicked() {
            reset_param(setter, p);
        }
        let editing_here = local.editing.as_ref().map_or(false, |(i, _)| *i == id);
        if editing_here {
            let (_, text) = local.editing.as_mut().unwrap();
            let te = egui::TextEdit::singleline(text).font(mono(11.0)).desired_width(w - 6.0).horizontal_align(egui::Align::Center);
            let r = ui.add(te);
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if enter || r.lost_focus() {
                if !esc {
                    if let Some(n) = p.string_to_normalized_value(text) {
                        set_float(setter, p, p.preview_plain(n));
                    }
                }
                local.editing = None;
            }
        } else {
            let val = p.normalized_value_to_string(norm, true);
            let lbl = ui.add(egui::Label::new(egui::RichText::new(val).font(mono(11.0)).color(TEXT)).sense(Sense::click()));
            if lbl.clicked() {
                local.editing = Some((id, p.normalized_value_to_string(norm, false)));
            }
        }
        ui.label(egui::RichText::new(label).font(sora(10.0)).color(MUTED));
    });
}

/// A knob that steps through the entries of an enum (the pre-delay note values).
fn note_knob(ui: &mut egui::Ui, setter: &ParamSetter, p: &EnumParam<DivParam>, label: &str, size: f32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
    let n = DIVS.len() as f32 - 1.0;
    let cur = p.value() as usize;
    draw_dial(ui.painter(), rect, cur as f32 / n, resp.hovered() || resp.dragged(), ACCENT);
    if resp.dragged() {
        let id = Id::new("note-knob-acc");
        let mut acc = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0) - resp.drag_delta().y / 14.0;
        let steps = acc.trunc();
        acc -= steps;
        ui.data_mut(|d| d.insert_temp(id, acc));
        if steps != 0.0 {
            let next = (cur as i32 + steps as i32).clamp(0, 9) as usize;
            if next != cur {
                set_enum(setter, p, DIVS[next]);
            }
        }
    }
    if resp.double_clicked() {
        reset_param(setter, p);
    }
    ui.label(egui::RichText::new(p.to_string()).font(mono(11.0)).color(TEXT));
    ui.label(egui::RichText::new(label).font(sora(10.0)).color(MUTED));
}

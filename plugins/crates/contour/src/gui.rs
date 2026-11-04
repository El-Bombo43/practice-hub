//! The editor: a Pro-Q-style graph (spectrum, curve, draggable nodes), a floating band panel, top and bottom bars.

use crate::analyzer::{Analyzer, Shared, DB_FLOOR};
use crate::dsp::response::band_db;
use crate::dsp::{BandSettings, BAND_COUNT};
use crate::fmt::{fmt_freq, note_name};
use crate::geom::*;
use crate::params::{BandParams, ContourParams, SlopeParam, TypeParam};
use crate::presets::{PresetBand, PRESETS};
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, pos2, vec2, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Pos2, Rect, Sense, Stroke};
use nih_plug_egui::{create_egui_editor, EguiState};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

const BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
const GRID: Color32 = Color32::from_rgb(0x22, 0x22, 0x26);
const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xea);
const MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x90);
const ACCENT: Color32 = Color32::from_rgb(0xd4, 0xff, 0x3a);
const BAND_COLORS: [Color32; 8] = [
    Color32::from_rgb(0xff, 0x6b, 0x6b),
    Color32::from_rgb(0xff, 0xb3, 0x47),
    Color32::from_rgb(0xff, 0xe6, 0x6d),
    Color32::from_rgb(0x7b, 0xd8, 0x8f),
    Color32::from_rgb(0x4e, 0xcd, 0xc4),
    Color32::from_rgb(0x5a, 0xa9, 0xff),
    Color32::from_rgb(0xa7, 0x8b, 0xfa),
    Color32::from_rgb(0xf4, 0x72, 0xb6),
];
const NODE_R: f32 = 5.0;
const HIT_R: f32 = 11.0;
const DESIGN_W: f32 = 960.0;
/// Window sizes: S, M, L (logical pixels at 100 % scaling).
pub const SIZES: [(u32, u32); 3] = [(760, 440), (960, 560), (1280, 740)];

fn band_color(i: usize) -> Color32 {
    BAND_COLORS[i % 8]
}

fn sora(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sora".into()))
}
fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono".into()))
}

pub struct Local {
    analyzer: Analyzer,
    drag: Option<Drag>,
    editing: Option<(Id, String)>,
    menu_band: Option<usize>,
    solo_hold: Option<(usize, Instant)>,
    solo_latched: Option<usize>,
}

struct Drag {
    band: usize,
    start: Pos2,
    f_ln: f32,
    gain: f32,
    q_ln: f32,
    axis_horizontal: Option<bool>,
    gain_type: bool,
    cut: bool,
}

impl Local {
    fn new() -> Self {
        Self { analyzer: Analyzer::new(480), drag: None, editing: None, menu_band: None, solo_hold: None, solo_latched: None }
    }
}

/// Fonts and the dark theme. Called once when the window opens (and by the offscreen renderer in the tests).
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

pub fn new_local() -> Local {
    Local::new()
}

pub fn create(params: Arc<ContourParams>, shared: Arc<Shared>) -> Option<Box<dyn Editor>> {
    let state: Arc<EguiState> = params.editor_state.clone();
    create_egui_editor(state, Local::new(), |ctx, _| setup(ctx), move |ctx, setter, local| update(ctx, setter, local, &params, &shared))
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

/// Every parameter of a band back to its default (which includes Used = off).
fn reset_band(setter: &ParamSetter, b: &BandParams) {
    for (_, ptr, _) in b.param_map() {
        unsafe {
            setter.raw_context.raw_begin_set_parameter(ptr);
            setter.raw_context.raw_set_parameter_normalized(ptr, ptr.default_normalized_value());
            setter.raw_context.raw_end_set_parameter(ptr);
        }
    }
}

fn apply_preset_band(setter: &ParamSetter, b: &BandParams, p: &PresetBand) {
    reset_band(setter, b);
    set_bool(setter, &b.used, true);
    set_enum(setter, &b.ftype, match p.ftype {
        crate::dsp::FilterType::Bell => TypeParam::Bell,
        crate::dsp::FilterType::LowShelf => TypeParam::LowShelf,
        crate::dsp::FilterType::HighShelf => TypeParam::HighShelf,
        crate::dsp::FilterType::TiltShelf => TypeParam::TiltShelf,
        crate::dsp::FilterType::LowCut => TypeParam::LowCut,
        crate::dsp::FilterType::HighCut => TypeParam::HighCut,
        crate::dsp::FilterType::Notch => TypeParam::Notch,
        crate::dsp::FilterType::BandPass => TypeParam::BandPass,
    });
    set_float(setter, &b.freq, p.freq);
    set_float(setter, &b.gain, p.gain);
    set_float(setter, &b.q, p.q);
    set_enum(setter, &b.slope, SlopeParam::from_db_per_oct(p.slope));
    if let Some((th, range, atk, rel)) = p.dynamic {
        set_bool(setter, &b.dynamic, true);
        set_float(setter, &b.thresh, th);
        set_float(setter, &b.range, range);
        set_float(setter, &b.attack, atk);
        set_float(setter, &b.release, rel);
    }
}

// ---------------------------------------------------------------------------------------------

pub fn update(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<ContourParams>, shared: &Arc<Shared>) {
    let (w, _) = params.editor_state.size();
    ctx.set_zoom_factor((w as f32 / DESIGN_W).clamp(0.5, 2.5));
    let mut prefs = params.prefs.read().clone();
    let before = (prefs.selected, prefs.range_idx, prefs.analyzer_mode, prefs.analyzer_speed);

    egui::TopBottomPanel::top("top").exact_height(36.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
        top_bar(ui, setter, params, &mut prefs);
    });
    egui::TopBottomPanel::bottom("bottom").exact_height(72.0).frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(12, 4))).show(ctx, |ui| {
        bottom_bar(ui, setter, params, local);
    });
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(ctx, |ui| {
        graph(ui, ctx, setter, local, params, shared, &mut prefs);
    });

    if before != (prefs.selected, prefs.range_idx, prefs.analyzer_mode, prefs.analyzer_speed) {
        *params.prefs.write() = prefs;
    }
    shared.solo.store(local.solo_now().map_or(-1, |i| i as i32), Ordering::Relaxed);
    ctx.request_repaint_after(std::time::Duration::from_millis(16));
}

impl Local {
    /// The band currently soloed: held down, or latched by a click.
    fn solo_now(&self) -> Option<usize> {
        self.solo_hold.map(|(b, _)| b).or(self.solo_latched)
    }
}

// ---------------------------------------------------------------------------------------------
// bars

fn top_bar(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ContourParams>, prefs: &mut crate::params::GuiPrefs) {
    ui.horizontal_centered(|ui| {
        ui.label(egui::RichText::new("CONTOUR").font(sora(15.0)).color(ACCENT));
        ui.add_space(14.0);
        egui::ComboBox::from_id_salt("preset").selected_text("Preset").width(170.0).show_ui(ui, |ui| {
            for p in PRESETS {
                if ui.selectable_label(false, p.name).clicked() {
                    for i in 0..BAND_COUNT {
                        match p.bands.get(i) {
                            Some(pb) => apply_preset_band(setter, params.band(i), pb),
                            None => reset_band(setter, params.band(i)),
                        }
                    }
                    prefs.selected = None;
                }
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let cur = params.editor_state.size();
            for (i, name) in ["L", "M", "S"].iter().enumerate() {
                let idx = 2 - i;
                let on = cur == SIZES[idx];
                let b = egui::Button::new(egui::RichText::new(*name).font(sora(12.0)).color(if on { BG } else { TEXT })).fill(if on { ACCENT } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }).min_size(vec2(28.0, 24.0));
                if ui.add(b).on_hover_text("Window size").clicked() && !on {
                    params.editor_state.set_requested_size(SIZES[idx]);
                }
            }
            ui.add_space(10.0);
            let ranges = ["±3 dB", "±6 dB", "±12 dB", "±30 dB"];
            egui::ComboBox::from_id_salt("range").selected_text(ranges[prefs.range_idx as usize % 4]).width(80.0).show_ui(ui, |ui| {
                for (i, r) in ranges.iter().enumerate() {
                    if ui.selectable_label(prefs.range_idx as usize == i, *r).clicked() {
                        prefs.range_idx = i as u8;
                    }
                }
            });
            let modes = ["Off", "Post", "Pre", "Pre+Post"];
            let speeds = ["Slow", "Medium", "Fast"];
            egui::ComboBox::from_id_salt("speed").selected_text(speeds[prefs.analyzer_speed as usize % 3]).width(76.0).show_ui(ui, |ui| {
                for (i, s) in speeds.iter().enumerate() {
                    if ui.selectable_label(prefs.analyzer_speed as usize == i, *s).clicked() {
                        prefs.analyzer_speed = i as u8;
                    }
                }
            });
            egui::ComboBox::from_id_salt("mode").selected_text(format!("Analyzer: {}", modes[prefs.analyzer_mode as usize % 4])).width(150.0).show_ui(ui, |ui| {
                for (i, m) in modes.iter().enumerate() {
                    if ui.selectable_label(prefs.analyzer_mode as usize == i, *m).clicked() {
                        prefs.analyzer_mode = i as u8;
                    }
                }
            });
        });
    });
}

fn bottom_bar(ui: &mut egui::Ui, setter: &ParamSetter, params: &Arc<ContourParams>, local: &mut Local) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        knob(ui, setter, &params.out_gain, "Output", ACCENT, 30.0, local);
        ui.add_space(16.0);
        toggle(ui, setter, &params.auto_gain, "Auto Gain", ACCENT);
        ui.add_space(16.0);
        knob(ui, setter, &params.gain_scale, "Gain Scale", ACCENT, 30.0, local);
    });
}

// ---------------------------------------------------------------------------------------------
// widgets

fn toggle(ui: &mut egui::Ui, setter: &ParamSetter, p: &BoolParam, label: &str, color: Color32) {
    let on = p.value();
    let text = egui::RichText::new(label).font(sora(12.0)).color(if on { BG } else { TEXT });
    let b = egui::Button::new(text).fill(if on { color } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }).min_size(vec2(60.0, 26.0));
    if ui.add(b).clicked() {
        set_bool(setter, p, !on);
    }
}

fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = 28;
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        center + vec2(a.cos(), a.sin()) * r
    }).collect()
}

/// A knob with an arc, a value field underneath (click to type, units accepted) and double-click to reset.
fn knob(ui: &mut egui::Ui, setter: &ParamSetter, p: &FloatParam, label: &str, color: Color32, size: f32, local: &mut Local) {
    let id = Id::new(("knob", p.name()));
    let w = (size + 30.0).max(64.0);
    ui.allocate_ui_with_layout(vec2(w, size + 30.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
        let painter = ui.painter();
        let c = rect.center();
        let r = size * 0.5 - 3.0;
        let (a0, a1) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 2.25);
        painter.add(epaint::Shape::line(arc_points(c, r, a0, a1), Stroke::new(3.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x30))));
        let norm = p.unmodulated_normalized_value();
        let hot = resp.hovered() || resp.dragged();
        painter.add(epaint::Shape::line(arc_points(c, r, a0, a0 + (a1 - a0) * norm.max(0.001)), Stroke::new(3.0_f32, if hot { color } else { color.gamma_multiply(0.85) })));
        let ang = a0 + (a1 - a0) * norm;
        painter.circle_filled(c + vec2(ang.cos(), ang.sin()) * (r - 6.0), 2.5, TEXT);

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

        // value text: click to type
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

fn combo_enum<E: Enum + PartialEq + Copy + 'static>(ui: &mut egui::Ui, setter: &ParamSetter, p: &EnumParam<E>, id: &str, all: &[E], name: fn(E) -> &'static str, enabled: bool) {
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(id).selected_text(name(p.value())).width(104.0).show_ui(ui, |ui| {
            for e in all {
                if ui.selectable_label(p.value() == *e, name(*e)).clicked() {
                    set_enum(setter, p, *e);
                }
            }
        });
    });
}

fn slope_name(s: SlopeParam) -> &'static str {
    match s {
        SlopeParam::S6 => "6 dB/oct",
        SlopeParam::S12 => "12 dB/oct",
        SlopeParam::S18 => "18 dB/oct",
        SlopeParam::S24 => "24 dB/oct",
        SlopeParam::S36 => "36 dB/oct",
        SlopeParam::S48 => "48 dB/oct",
        SlopeParam::S72 => "72 dB/oct",
        SlopeParam::S96 => "96 dB/oct",
    }
}

/// Filled area under a polyline, drawn as a triangle strip (the shape is not convex).
fn fill_under(painter: &egui::Painter, pts: &[Pos2], bottom: f32, color: Color32) {
    if pts.len() < 2 {
        return;
    }
    let mut mesh = epaint::Mesh::default();
    for p in pts {
        mesh.colored_vertex(*p, color);
        mesh.colored_vertex(pos2(p.x, bottom), color);
    }
    for i in 0..pts.len() - 1 {
        let k = (i * 2) as u32;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(epaint::Shape::mesh(mesh));
}

/// Filled area between two polylines of the same length.
fn fill_between(painter: &egui::Painter, a: &[Pos2], b: &[Pos2], color: Color32) {
    if a.len() < 2 || a.len() != b.len() {
        return;
    }
    let mut mesh = epaint::Mesh::default();
    for (p, q) in a.iter().zip(b.iter()) {
        mesh.colored_vertex(*p, color);
        mesh.colored_vertex(*q, color);
    }
    for i in 0..a.len() - 1 {
        let k = (i * 2) as u32;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    painter.add(epaint::Shape::mesh(mesh));
}

// ---------------------------------------------------------------------------------------------
// the graph

fn type_of(i: usize, params: &ContourParams) -> TypeParam {
    params.band(i).ftype.value()
}

#[allow(clippy::too_many_arguments)]
fn graph(ui: &mut egui::Ui, ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<ContourParams>, shared: &Arc<Shared>, prefs: &mut crate::params::GuiPrefs) {
    let full = ui.available_rect_before_wrap();
    // the bottom 18 px of the graph area carry the frequency labels, so the band panel never covers them
    let g = Rect::from_min_max(full.min, pos2(full.max.x - 42.0, full.max.y - 18.0));
    let painter = ui.painter_at(full);
    let fs = f32::from_bits(shared.sample_rate.load(Ordering::Relaxed)) as f64;
    let settings = params.band_settings();
    let gs = (params.gain_scale.value() / 100.0) as f64;
    let dyn_db: [f32; BAND_COUNT] = std::array::from_fn(|i| f32::from_bits(shared.dyn_db[i].load(Ordering::Relaxed)));
    let range = RANGES_DB[prefs.range_idx as usize % 4];

    let x_of = |f: f32| g.left() + freq_to_frac(f) * g.width();
    let y_of_g = |gain: f32| g.top() + gain_to_frac(gain, range) * g.height();
    let y_of_an = |db: f32| g.top() + (-db.clamp(DB_FLOOR, 0.0)) / -DB_FLOOR * g.height();

    painter.rect_filled(g, 0.0, BG);
    painter.rect_filled(Rect::from_min_max(pos2(full.left(), g.bottom()), pos2(g.right(), full.bottom())), 0.0, PANEL);

    // grid
    for f in [20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10_000.0, 20_000.0f32] {
        let x = x_of(f);
        painter.line_segment([pos2(x, g.top()), pos2(x, g.bottom())], Stroke::new(1.0_f32, GRID));
        let label = if f >= 1000.0 { format!("{}k", (f / 1000.0) as u32) } else { format!("{}", f as u32) };
        painter.text(pos2(x, g.bottom() + 9.0), Align2::CENTER_CENTER, label, mono(10.0), MUTED);
    }
    let step = grid_step(range);
    let mut gain = -range;
    while gain <= range + 0.01 {
        let y = y_of_g(gain);
        let zero = gain.abs() < 0.01;
        painter.line_segment([pos2(g.left(), y), pos2(g.right(), y)], Stroke::new(1.0_f32, if zero { Color32::from_rgb(0x3a, 0x3a, 0x42) } else { GRID }));
        if (gain / step).round() == gain / step && (zero || gain.abs() as i32 % (step as i32 * 2).max(1) == 0 || range <= 6.0) {
            painter.text(pos2(g.left() + 4.0, y - 2.0), Align2::LEFT_BOTTOM, if zero { "0".to_string() } else { format!("{gain:+.0}") }, mono(10.0), MUTED);
        }
        gain += step;
    }
    for db in [0.0f32, -18.0, -36.0, -54.0, -72.0, -90.0] {
        painter.text(pos2(g.right() + 6.0, y_of_an(db).clamp(g.top() + 6.0, g.bottom() - 6.0)), Align2::LEFT_CENTER, format!("{db:.0}"), mono(10.0), MUTED);
    }

    // analyzer
    let (post, pre) = match prefs.analyzer_mode {
        1 => (true, false),
        2 => (false, true),
        3 => (true, true),
        _ => (false, false),
    };
    if post || pre {
        let decay = [0.5, 1.5, 4.0][prefs.analyzer_speed as usize % 3];
        {
            let ring = shared.ring.lock();
            if post {
                local.analyzer.update(&ring, false, fs as f32, decay);
            }
            if pre {
                local.analyzer.update(&ring, true, fs as f32, decay);
            }
        }
        let trace = |db: &[f32]| -> Vec<Pos2> {
            db.iter().enumerate().filter(|(i, _)| local.analyzer.point_freq(*i) < fs as f32 * 0.5).map(|(i, v)| pos2(x_of(local.analyzer.point_freq(i)), y_of_an(*v))).collect()
        };
        if post {
            let pts = trace(&local.analyzer.post.db);
            fill_under(&painter, &pts, g.bottom(), TEXT.gamma_multiply(0.10));
            painter.add(epaint::Shape::line(pts, Stroke::new(1.0_f32, TEXT.gamma_multiply(0.45))));
        }
        if pre {
            painter.add(epaint::Shape::line(trace(&local.analyzer.pre.db), Stroke::new(1.0_f32, MUTED.gamma_multiply(0.7))));
        }
    }

    // curves
    let n = ((g.width() / 2.0) as usize).clamp(120, 700);
    let freqs: Vec<f32> = (0..n).map(|i| frac_to_freq(i as f32 / (n - 1) as f32)).collect();
    let zero_y = y_of_g(0.0);
    let mut combined = vec![0.0f64; n];
    let mut band_pts: Vec<Option<Vec<Pos2>>> = vec![None; BAND_COUNT];
    let sel = prefs.selected.filter(|&i| settings[i].used);
    let hover_pos = ui.input(|i| i.pointer.hover_pos()).filter(|p| g.contains(*p));

    // node positions first, so hover can brighten the right curve
    let node_pts: Vec<Option<(f32, f32)>> = settings.iter().map(|s| {
        s.used.then(|| (x_of(s.freq), if s.ftype.uses_gain() { y_of_g(s.gain_db) } else { zero_y }))
    }).collect();
    let hit = hover_pos.and_then(|p| hit_node(&node_pts, (p.x, p.y), HIT_R));

    for (bi, s) in settings.iter().enumerate() {
        if !(s.used && s.enabled) {
            continue;
        }
        let total = gs * s.gain_db as f64 + dyn_db[bi] as f64;
        let curve: Vec<f64> = freqs.iter().map(|&f| band_db(s, total, fs, f as f64)).collect();
        for (c, v) in combined.iter_mut().zip(curve.iter()) {
            *c += v;
        }
        let pts: Vec<Pos2> = freqs.iter().zip(curve.iter()).map(|(&f, &v)| pos2(x_of(f), y_of_g(v as f32))).collect();
        if s.dynamic && s.ftype.allows_dynamic() {
            let hi: Vec<Pos2> = freqs.iter().map(|&f| pos2(x_of(f), y_of_g(band_db(s, gs * (s.gain_db + s.range_db) as f64, fs, f as f64) as f32))).collect();
            let lo: Vec<Pos2> = freqs.iter().map(|&f| pos2(x_of(f), y_of_g(band_db(s, gs * s.gain_db as f64, fs, f as f64) as f32))).collect();
            fill_between(&painter, &lo, &hi, band_color(bi).gamma_multiply(0.18));
        }
        band_pts[bi] = Some(pts);
    }
    // combined curve: faint fill to 0 dB, glow, line
    let comb: Vec<Pos2> = freqs.iter().zip(combined.iter()).map(|(&f, &v)| pos2(x_of(f), y_of_g(v as f32))).collect();
    for (bi, pts) in band_pts.iter().enumerate() {
        if let Some(pts) = pts {
            let strong = sel == Some(bi) || hit == Some(bi);
            painter.add(epaint::Shape::line(pts.clone(), Stroke::new(if strong { 1.6_f32 } else { 1.0_f32 }, band_color(bi).gamma_multiply(if strong { 0.95 } else { 0.35 }))));
        }
    }
    let zero_line: Vec<Pos2> = comb.iter().map(|p| pos2(p.x, zero_y)).collect();
    fill_between(&painter, &zero_line, &comb, ACCENT.gamma_multiply(0.07));
    painter.add(epaint::Shape::line(comb.clone(), Stroke::new(6.0_f32, ACCENT.gamma_multiply(0.12))));
    painter.add(epaint::Shape::line(comb, Stroke::new(2.0_f32, ACCENT)));

    // interaction
    let resp = ui.interact(g, Id::new("graph"), Sense::click_and_drag());
    graph_input(ctx, ui, &resp, setter, local, params, prefs, &settings, &node_pts, hit, g, range);

    // nodes
    for (bi, s) in settings.iter().enumerate() {
        let Some((x, y)) = node_pts[bi] else { continue };
        let c = band_color(bi);
        let center = pos2(x, y);
        if s.enabled {
            painter.circle_filled(center, NODE_R, c);
        } else {
            painter.circle_stroke(center, NODE_R, Stroke::new(1.5_f32, c.gamma_multiply(0.4)));
        }
        if sel == Some(bi) {
            painter.circle_stroke(center, NODE_R + 3.5, Stroke::new(1.8_f32, ACCENT));
        }
        painter.text(center + vec2(0.0, -NODE_R - 9.0), Align2::CENTER_CENTER, format!("{}", bi + 1), mono(10.0), if s.enabled { TEXT } else { MUTED });
    }

    // hover tooltip / readout
    if let Some(p) = hover_pos {
        if let Some(b) = hit {
            let s = &settings[b];
            let txt = format!("Band {} · {} · {} · {:+.1} dB · Q {:.2}", b + 1, type_of(b, params).name(), fmt_freq(s.freq), s.gain_db, s.q);
            let r = painter.text(p + vec2(14.0, 16.0), Align2::LEFT_TOP, txt, sora(11.0), TEXT);
            painter.rect_stroke(r.expand(4.0), 4.0, Stroke::new(1.0_f32, GRID), epaint::StrokeKind::Outside);
        } else {
            let f = frac_to_freq((p.x - g.left()) / g.width());
            let tr = if post || !pre { &local.analyzer.post } else { &local.analyzer.pre };
            let idx = (0..local.analyzer.points).min_by_key(|&i| ((local.analyzer.point_freq(i) - f).abs() * 1000.0) as i64).unwrap_or(0);
            let db = tr.db[idx];
            let txt = if post || pre { format!("{} · {} · {:.0} dB", fmt_freq(f), note_name(f), db) } else { format!("{} · {}", fmt_freq(f), note_name(f)) };
            painter.text(p + vec2(14.0, 16.0), Align2::LEFT_TOP, txt, mono(11.0), MUTED);
        }
    }

    if let Some(b) = sel {
        band_panel(ctx, setter, local, params, b, g, &settings[b], dyn_db[b]);
    }
}

#[allow(clippy::too_many_arguments)]
fn graph_input(
    ctx: &egui::Context, ui: &mut egui::Ui, resp: &egui::Response, setter: &ParamSetter, local: &mut Local, params: &Arc<ContourParams>,
    prefs: &mut crate::params::GuiPrefs, settings: &[BandSettings; BAND_COUNT], node_pts: &[Option<(f32, f32)>], hit: Option<usize>, g: Rect, range: f32,
) {
    let (shift, ctrl, alt) = ctx.input(|i| (i.modifiers.shift, i.modifiers.ctrl || i.modifiers.command, i.modifiers.alt));

    // Delete key removes the selected band
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        if let Some(b) = prefs.selected.filter(|&b| settings[b].used) {
            reset_band(setter, params.band(b));
            prefs.selected = None;
        }
    }

    // wheel over a node: Q (Ctrl: dynamic range)
    let scroll = ctx.input(|i| i.raw_scroll_delta.y);
    if let (Some(b), true) = (hit, scroll.abs() > 0.1 && resp.hovered()) {
        let bp = params.band(b);
        if ctrl {
            if settings[b].ftype.allows_dynamic() {
                if !settings[b].dynamic {
                    set_bool(setter, &bp.dynamic, true);
                }
                let r = (settings[b].range_db + scroll / 50.0 * 0.5 * if shift { 0.2 } else { 1.0 }).clamp(-30.0, 30.0);
                set_float(setter, &bp.range, r);
            }
        } else {
            let q = (settings[b].q * (scroll / 50.0 * 0.095 * if shift { 0.25 } else { 1.0 }).exp()).clamp(0.025, 40.0);
            set_float(setter, &bp.q, q);
        }
        prefs.selected = Some(b);
    }

    // press: start dragging a node
    if resp.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin());
        if let Some(b) = origin.and_then(|o| hit_node(node_pts, (o.x, o.y), HIT_R)) {
            let s = &settings[b];
            let bp = params.band(b);
            prefs.selected = Some(b);
            setter.begin_set_parameter(&bp.freq);
            if s.ftype.uses_gain() {
                setter.begin_set_parameter(&bp.gain);
            } else if s.ftype.is_cut() {
                setter.begin_set_parameter(&bp.q);
            }
            local.drag = Some(Drag {
                band: b, start: origin.unwrap(), f_ln: s.freq.ln(), gain: s.gain_db, q_ln: s.q.ln(), axis_horizontal: None,
                gain_type: s.ftype.uses_gain(), cut: s.ftype.is_cut(),
            });
        }
    }
    if resp.dragged() {
        if let (Some(d), Some(p)) = (local.drag.as_mut(), ui.input(|i| i.pointer.latest_pos())) {
            let speed = if shift { 0.1 } else { 1.0 };
            let (mut dx, mut dy) = ((p.x - d.start.x) * speed, (p.y - d.start.y) * speed);
            if ctrl {
                // constrain to the axis moved first
                if d.axis_horizontal.is_none() && (dx.abs() > 4.0 || dy.abs() > 4.0) {
                    d.axis_horizontal = Some(dx.abs() >= dy.abs());
                }
                match d.axis_horizontal {
                    Some(true) => dy = 0.0,
                    Some(false) => dx = 0.0,
                    None => {}
                }
            }
            let bp = params.band(d.band);
            let span = (F_MAX / F_MIN).ln();
            let f = (d.f_ln + dx / g.width() * span).exp().clamp(F_MIN, F_MAX);
            setter.set_parameter(&bp.freq, f);
            if d.gain_type {
                let gain = (d.gain - dy / g.height() * 2.0 * range).clamp(-30.0, 30.0);
                setter.set_parameter(&bp.gain, gain);
            } else if d.cut {
                setter.set_parameter(&bp.q, (d.q_ln - dy * 0.01).exp().clamp(0.025, 40.0));
            }
        }
    }
    if resp.drag_stopped() {
        if let Some(d) = local.drag.take() {
            let bp = params.band(d.band);
            setter.end_set_parameter(&bp.freq);
            if d.gain_type {
                setter.end_set_parameter(&bp.gain);
            } else if d.cut {
                setter.end_set_parameter(&bp.q);
            }
        }
    }

    // clicks
    if resp.clicked() {
        match hit {
            Some(b) if alt => {
                reset_band(setter, params.band(b));
                if prefs.selected == Some(b) {
                    prefs.selected = None;
                }
            }
            Some(b) => prefs.selected = Some(b),
            None => prefs.selected = None,
        }
    }
    if resp.double_clicked() {
        match hit {
            Some(b) => set_bool(setter, &params.band(b).on, !settings[b].enabled),
            None => {
                if let Some(p) = resp.interact_pointer_pos() {
                    let used: Vec<bool> = settings.iter().map(|s| s.used).collect();
                    if let Some(b) = first_unused(&used) {
                        let fx = (p.x - g.left()) / g.width();
                        let fy = (p.y - g.top()) / g.height();
                        let bp = params.band(b);
                        reset_band(setter, bp);
                        set_bool(setter, &bp.used, true);
                        set_float(setter, &bp.freq, frac_to_freq(fx));
                        match created_type(fx) {
                            CreateKind::LowCut => set_enum(setter, &bp.ftype, TypeParam::LowCut),
                            CreateKind::HighCut => set_enum(setter, &bp.ftype, TypeParam::HighCut),
                            CreateKind::Bell => set_float(setter, &bp.gain, frac_to_gain(fy, range).clamp(-30.0, 30.0)),
                        }
                        prefs.selected = Some(b);
                    }
                }
            }
        }
    }

    // right-click menu
    if resp.hovered() && ctx.input(|i| i.pointer.secondary_pressed()) {
        local.menu_band = hit;
        if let Some(b) = hit {
            prefs.selected = Some(b);
        }
    }
    let menu_band = local.menu_band;
    resp.context_menu(|ui| {
        let Some(b) = menu_band.filter(|&b| settings[b].used) else {
            ui.close_menu();
            return;
        };
        let bp = params.band(b);
        ui.menu_button("Type", |ui| {
            for t in TypeParam::ALL {
                if ui.selectable_label(bp.ftype.value() == t, t.name()).clicked() {
                    set_enum(setter, &bp.ftype, t);
                    ui.close_menu();
                }
            }
        });
        if settings[b].ftype.is_cut() {
            ui.menu_button("Slope", |ui| {
                for s in SlopeParam::ALL {
                    if ui.selectable_label(bp.slope.value() == s, slope_name(s)).clicked() {
                        set_enum(setter, &bp.slope, s);
                        ui.close_menu();
                    }
                }
            });
        }
        if ui.button(if local.solo_latched == Some(b) { "Unsolo" } else { "Solo" }).clicked() {
            local.solo_latched = if local.solo_latched == Some(b) { None } else { Some(b) };
            ui.close_menu();
        }
        if settings[b].ftype.allows_dynamic() && ui.button(if settings[b].dynamic { "Dynamic off" } else { "Dynamic on" }).clicked() {
            set_bool(setter, &bp.dynamic, !settings[b].dynamic);
            ui.close_menu();
        }
        if ui.button(if settings[b].ext { "Ext Sidechain off" } else { "Ext Sidechain on" }).clicked() {
            set_bool(setter, &bp.ext, !settings[b].ext);
            ui.close_menu();
        }
        if ui.button("Reset band").clicked() {
            reset_band(setter, bp);
            set_bool(setter, &bp.used, true);
            ui.close_menu();
        }
        if ui.button("Delete").clicked() {
            reset_band(setter, bp);
            if prefs.selected == Some(b) {
                prefs.selected = None;
            }
            ui.close_menu();
        }
    });
}

// ---------------------------------------------------------------------------------------------
// the floating band panel

#[allow(clippy::too_many_arguments)]
fn band_panel(ctx: &egui::Context, setter: &ParamSetter, local: &mut Local, params: &Arc<ContourParams>, b: usize, g: Rect, s: &BandSettings, dyn_now: f32) {
    let bp = params.band(b);
    let color = band_color(b);
    let width = 640.0;
    let pos = pos2(g.center().x - width / 2.0, g.bottom() - 178.0);
    egui::Area::new(Id::new("band_panel")).fixed_pos(pos).order(egui::Order::Foreground).show(ctx, |ui| {
        egui::Frame::new().fill(PANEL.gamma_multiply(0.97)).stroke(Stroke::new(1.0_f32, GRID)).corner_radius(6).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.set_width(width - 24.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("Band {}", b + 1)).font(sora(13.0)).color(color));
                combo_enum(ui, setter, &bp.ftype, "ptype", &TypeParam::ALL, TypeParam::name, true);
                combo_enum(ui, setter, &bp.slope, "pslope", &SlopeParam::ALL, slope_name, s.ftype.is_cut());
                ui.add_space(6.0);
                knob(ui, setter, &bp.freq, "Freq", color, 38.0, local);
                ui.add_enabled_ui(s.ftype.uses_gain(), |ui| knob(ui, setter, &bp.gain, "Gain", color, 38.0, local));
                knob(ui, setter, &bp.q, "Q", color, 38.0, local);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        // Solo: click latches, press-and-hold solos while held
                        let held = local.solo_hold.map_or(false, |(hb, _)| hb == b);
                        let on = held || local.solo_latched == Some(b);
                        let r = ui.add(egui::Button::new(egui::RichText::new("Solo").font(sora(11.0)).color(if on { BG } else { TEXT })).fill(if on { ACCENT } else { Color32::from_rgb(0x1c, 0x1c, 0x20) }).min_size(vec2(44.0, 24.0)));
                        if r.is_pointer_button_down_on() {
                            if local.solo_hold.is_none() {
                                local.solo_hold = Some((b, Instant::now()));
                            }
                        } else if let Some((hb, t0)) = local.solo_hold {
                            local.solo_hold = None;
                            if t0.elapsed().as_millis() < 300 {
                                local.solo_latched = if local.solo_latched == Some(hb) { None } else { Some(hb) };
                            }
                        }
                        toggle(ui, setter, &bp.on, "On", color);
                    });
                    if ui.add(egui::Button::new(egui::RichText::new("Delete").font(sora(11.0))).min_size(vec2(104.0, 24.0))).clicked() {
                        reset_band(setter, bp);
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                let allowed = s.ftype.allows_dynamic();
                ui.add_enabled_ui(allowed, |ui| {
                    toggle(ui, setter, &bp.dynamic, "Dynamic", color);
                    ui.add_enabled_ui(s.dynamic, |ui| {
                        knob(ui, setter, &bp.thresh, "Threshold", color, 34.0, local);
                        knob(ui, setter, &bp.range, "Range", color, 34.0, local);
                        knob(ui, setter, &bp.attack, "Attack", color, 34.0, local);
                        knob(ui, setter, &bp.release, "Release", color, 34.0, local);
                        toggle(ui, setter, &bp.ext, "Ext SC", color);
                    });
                });
                // live gain change from the detector
                let (rect, _) = ui.allocate_exact_size(vec2(74.0, 40.0), Sense::hover());
                let bar = Rect::from_center_size(rect.center(), vec2(64.0, 8.0));
                ui.painter().rect_filled(bar, 3.0, Color32::from_rgb(0x1c, 0x1c, 0x20));
                let half = (dyn_now / 30.0).clamp(-1.0, 1.0) * 32.0;
                let fill = Rect::from_min_max(pos2(bar.center().x.min(bar.center().x + half), bar.top()), pos2(bar.center().x.max(bar.center().x + half), bar.bottom()));
                ui.painter().rect_filled(fill, 3.0, color);
                ui.painter().text(pos2(rect.center().x, rect.bottom() - 2.0), Align2::CENTER_BOTTOM, format!("{dyn_now:+.1} dB"), mono(10.0), MUTED);
            });
        });
    });
}

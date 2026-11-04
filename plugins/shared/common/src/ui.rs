//! The shared theme and widgets: palette, fonts, arc knobs (click a value to type, double-click resets, Shift-drag is
//! fine), toggles, segmented buttons, meter bars, S/M/L size buttons.
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, vec2, Color32, FontData, FontDefinitions, FontFamily, FontId, Id, Pos2, Rect, Sense, Stroke};
use nih_plug_egui::EguiState;
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x0a, 0x0a, 0x0b);
pub const PANEL: Color32 = Color32::from_rgb(0x14, 0x14, 0x16);
pub const INSET: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x20);
pub const GRID: Color32 = Color32::from_rgb(0x22, 0x22, 0x26);
pub const WIRE: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x40);
pub const TEXT: Color32 = Color32::from_rgb(0xe8, 0xe8, 0xea);
pub const MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x90);
pub const ACCENT: Color32 = Color32::from_rgb(0xd4, 0xff, 0x3a);
pub const RED: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);
pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x9f, 0x43);
pub const YELLOW: Color32 = Color32::from_rgb(0xf5, 0xd5, 0x3f);
pub const GREEN: Color32 = Color32::from_rgb(0x5a, 0xe0, 0x7a);
pub const CYAN: Color32 = Color32::from_rgb(0x3e, 0xe6, 0xd4);

pub fn sora(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("sora".into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono".into()))
}

/// Fonts and the dark theme. Call once when the window opens.
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
    v.widgets.inactive.bg_fill = INSET;
    v.widgets.inactive.weak_bg_fill = INSET;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x26, 0x26, 0x2c);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.window_corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Scale the whole UI so the fixed design width fills the window (S/M/L sizes).
pub fn apply_zoom(ctx: &egui::Context, state: &EguiState, design_w: f32) {
    let (w, _) = state.size();
    ctx.set_zoom_factor((w as f32 / design_w).clamp(0.5, 2.5));
}

// ---- parameter gestures (each is one host gesture) ----
pub fn set_float(setter: &ParamSetter, p: &FloatParam, v: f32) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
pub fn set_bool(setter: &ParamSetter, p: &BoolParam, v: bool) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
pub fn set_enum<E: Enum + PartialEq>(setter: &ParamSetter, p: &EnumParam<E>, v: E) {
    setter.begin_set_parameter(p);
    setter.set_parameter(p, v);
    setter.end_set_parameter(p);
}
pub fn reset_param<P: Param>(setter: &ParamSetter, p: &P) {
    setter.begin_set_parameter(p);
    setter.set_parameter_normalized(p, p.default_normalized_value());
    setter.end_set_parameter(p);
}

/// The value field being typed into, if any.
#[derive(Default)]
pub struct Editing(pub Option<(Id, String)>);

fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = 28;
    (0..=n).map(|i| center + vec2((a0 + (a1 - a0) * i as f32 / n as f32).cos(), (a0 + (a1 - a0) * i as f32 / n as f32).sin()) * r).collect()
}

/// The dial itself. `bipolar` fills from 12 o'clock: left in `neg` colour, right in `color`.
pub fn draw_dial(painter: &egui::Painter, rect: Rect, norm: f32, hot: bool, color: Color32, bipolar: Option<Color32>) {
    let c = rect.center();
    let r = rect.width() * 0.5 - 3.0;
    let (a0, a1) = (std::f32::consts::PI * 0.75, std::f32::consts::PI * 2.25);
    painter.add(epaint::Shape::line(arc_points(c, r, a0, a1), Stroke::new(3.0_f32, Color32::from_rgb(0x2a, 0x2a, 0x30))));
    let amid = (a0 + a1) * 0.5;
    let ang = a0 + (a1 - a0) * norm;
    let dim = |col: Color32| if hot { col } else { col.gamma_multiply(0.85) };
    match bipolar {
        Some(neg) => {
            let (from, to, col) = if norm >= 0.5 { (amid, ang, color) } else { (ang, amid, neg) };
            if (to - from).abs() > 0.002 {
                painter.add(epaint::Shape::line(arc_points(c, r, from, to), Stroke::new(3.0_f32, dim(col))));
            }
            painter.line_segment([c + vec2(amid.cos(), amid.sin()) * (r + 2.0), c + vec2(amid.cos(), amid.sin()) * (r - 3.0)], Stroke::new(1.0_f32, MUTED));
        }
        None => {
            painter.add(epaint::Shape::line(arc_points(c, r, a0, a0 + (a1 - a0) * norm.max(0.001)), Stroke::new(3.0_f32, dim(color))));
        }
    }
    painter.circle_filled(c + vec2(ang.cos(), ang.sin()) * (r - 6.0), 2.5, TEXT);
}

/// A knob with the value under it and the label below that.
pub fn knob(ui: &mut egui::Ui, setter: &ParamSetter, p: &FloatParam, label: &str, color: Color32, size: f32, ed: &mut Editing) {
    knob_ex(ui, setter, p, label, color, size, ed, None)
}

pub fn knob_ex(ui: &mut egui::Ui, setter: &ParamSetter, p: &FloatParam, label: &str, color: Color32, size: f32, ed: &mut Editing, bipolar: Option<Color32>) {
    let id = Id::new(("knob", p.name()));
    let w = (size + 30.0).max(62.0);
    ui.allocate_ui_with_layout(vec2(w, size + 30.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
        let norm = p.unmodulated_normalized_value();
        draw_dial(ui.painter(), rect, norm, resp.hovered() || resp.dragged(), color, bipolar);
        if resp.drag_started() {
            setter.begin_set_parameter(p);
        }
        if resp.dragged() {
            let fine = ui.input(|i| i.modifiers.shift);
            let n = (p.unmodulated_normalized_value() - resp.drag_delta().y / 180.0 * if fine { 0.1 } else { 1.0 }).clamp(0.0, 1.0);
            setter.set_parameter_normalized(p, n);
        }
        if resp.drag_stopped() {
            setter.end_set_parameter(p);
        }
        if resp.double_clicked() {
            reset_param(setter, p);
        }
        if ed.0.as_ref().map_or(false, |(i, _)| *i == id) {
            let (_, text) = ed.0.as_mut().unwrap();
            let r = ui.add(egui::TextEdit::singleline(text).font(mono(11.0)).desired_width(w - 6.0).horizontal_align(egui::Align::Center));
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
                ed.0 = None;
            }
        } else {
            let val = p.normalized_value_to_string(norm, true);
            if ui.add(egui::Label::new(egui::RichText::new(val).font(mono(11.0)).color(TEXT)).sense(Sense::click())).clicked() {
                ed.0 = Some((id, p.normalized_value_to_string(norm, false)));
            }
        }
        ui.label(egui::RichText::new(label).font(sora(10.0)).color(MUTED));
    });
}

/// A pill toggle.
pub fn toggle(ui: &mut egui::Ui, setter: &ParamSetter, p: &BoolParam, label: &str, color: Color32) {
    let on = p.value();
    let text = egui::RichText::new(label).font(sora(11.0)).color(if on { Color32::BLACK } else { TEXT });
    let b = egui::Button::new(text).fill(if on { color } else { INSET }).corner_radius(999.0).min_size(vec2(54.0, 22.0));
    if ui.add(b).clicked() {
        set_bool(setter, p, !on);
    }
}

/// Segmented buttons for an enum; the active one is filled with the accent.
pub fn segmented<E: Enum + PartialEq + Copy + 'static>(ui: &mut egui::Ui, setter: &ParamSetter, p: &EnumParam<E>, all: &[(E, &str)], accent: Color32) {
    let cur = p.value();
    for (e, name) in all {
        let on = cur == *e;
        let b = egui::Button::new(egui::RichText::new(*name).font(sora(11.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { accent } else { INSET }).min_size(vec2(52.0, 22.0));
        if ui.add(b).clicked() && !on {
            set_enum(setter, p, *e);
        }
    }
}

pub fn combo_enum<E: Enum + PartialEq + Copy + 'static>(ui: &mut egui::Ui, setter: &ParamSetter, p: &EnumParam<E>, id: &str, all: &[(E, &str)], width: f32) {
    let cur = all.iter().find(|(e, _)| *e == p.value()).map_or("", |(_, n)| *n);
    egui::ComboBox::from_id_salt(id).selected_text(cur).width(width).show_ui(ui, |ui| {
        for (e, name) in all {
            if ui.selectable_label(p.value() == *e, *name).clicked() {
                set_enum(setter, p, *e);
            }
        }
    });
}

/// S / M / L buttons (right-to-left layout expected).
pub fn size_buttons(ui: &mut egui::Ui, state: &EguiState, sizes: &[(u32, u32); 3]) {
    let cur = state.size();
    for (i, name) in ["L", "M", "S"].iter().enumerate() {
        let idx = 2 - i;
        let on = cur == sizes[idx];
        let b = egui::Button::new(egui::RichText::new(*name).font(sora(12.0)).color(if on { Color32::BLACK } else { TEXT })).fill(if on { ACCENT } else { INSET }).min_size(vec2(28.0, 24.0));
        if ui.add(b).on_hover_text("Window size").clicked() && !on {
            state.set_requested_size(sizes[idx]);
        }
    }
}

/// Peak meter with hold: `level`/`hold` are 0..1 fractions of the bar. `from_top` hangs the fill from the top (gain
/// reduction).
pub fn meter_bar(painter: &egui::Painter, bar: Rect, level: f32, hold: f32, color: Color32, from_top: bool) {
    painter.rect_filled(bar, 3.0, INSET);
    let h = bar.height();
    let (l, hd) = (level.clamp(0.0, 1.0), hold.clamp(0.0, 1.0));
    if from_top {
        painter.rect_filled(Rect::from_min_max(bar.min, Pos2::new(bar.right(), bar.top() + h * l)), 3.0, color);
        let y = bar.top() + h * hd;
        painter.line_segment([Pos2::new(bar.left(), y), Pos2::new(bar.right(), y)], Stroke::new(1.5_f32, TEXT));
    } else {
        painter.rect_filled(Rect::from_min_max(Pos2::new(bar.left(), bar.bottom() - h * l), bar.max), 3.0, color);
        let y = bar.bottom() - h * hd;
        painter.line_segment([Pos2::new(bar.left(), y), Pos2::new(bar.right(), y)], Stroke::new(1.5_f32, TEXT));
    }
}

/// Meter ballistics: instant rise, fall at `fall` units per second, peak hold for 1 s then the same fall.
#[derive(Clone, Copy, Default)]
pub struct Ballistics {
    pub level: f32,
    pub hold: f32,
    hold_t: f32,
}
impl Ballistics {
    /// `v` is the new peak in the bar's units (e.g. dB above the floor).
    pub fn update(&mut self, v: f32, dt: f32, fall: f32) {
        self.level = if v > self.level { v } else { (self.level - fall * dt).max(v).max(0.0) };
        if v >= self.hold {
            self.hold = v;
            self.hold_t = 0.0;
        } else {
            self.hold_t += dt;
            if self.hold_t > 1.0 {
                self.hold = (self.hold - fall * dt).max(self.level);
            }
        }
    }
}

pub fn db_frac(db: f32, floor: f32) -> f32 {
    ((db - floor) / -floor).clamp(0.0, 1.0)
}

/// Rail label + value under a meter.
pub fn meter_label(painter: &egui::Painter, x: f32, y: f32, name: &str, value: &str) {
    painter.text(Pos2::new(x, y), egui::Align2::CENTER_TOP, name, sora(10.0), MUTED);
    painter.text(Pos2::new(x, y + 11.0), egui::Align2::CENTER_TOP, value, mono(9.0), MUTED);
}

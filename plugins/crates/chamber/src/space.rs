//! The animated "space": a pseudo-3D wireframe (no 3D engine) whose shape follows the mode, with rings that spread from
//! each transient, glow that follows the wet level, a tone tint, duck dimming and a freeze tint.
use nih_plug_egui::egui::{self, pos2, Color32, Painter, Pos2, Rect, Stroke};

const ACCENT: [f32; 3] = [212.0, 255.0, 58.0];
const WARM: [f32; 3] = [255.0, 176.0, 84.0];
const FREEZE: [f32; 3] = [191.0, 231.0, 255.0];
const WIRE: [f32; 3] = [58.0, 58.0, 64.0];
const MAX_RINGS: usize = 24;

#[derive(Clone, Copy)]
pub struct Inputs {
    pub mode: usize,
    pub size: f32,
    pub decay_s: f32,
    pub width: f32,
    pub damping_hz: f32,
    pub high_cut_hz: f32,
    pub mod_rate: f32,
    pub mod_depth: f32,
    pub duck_gr_db: f32,
    pub freeze: bool,
    pub wet_level: f32,
    /// linear input peak since the last frame
    pub in_peak: f32,
}

impl Default for Inputs {
    fn default() -> Self {
        Self { mode: 0, size: 0.5, decay_s: 2.0, width: 1.0, damping_hz: 7000.0, high_cut_hz: 12_000.0, mod_rate: 0.6, mod_depth: 0.25, duck_gr_db: 0.0, freeze: false, wet_level: 0.0, in_peak: 0.0 }
    }
}

struct Ring {
    age: f32,
    /// source position in shape coordinates (u across, v along the depth), -1..1
    u: f32,
    v: f32,
    strength: f32,
}

pub struct SpaceState {
    rings: Vec<Ring>,
    scale: f32,
    glow: f32,
    freeze_amt: f32,
    duck_dim: f32,
    in_avg: f32,
    cooldown: f32,
    mode: usize,
    mode_fade: f32,
    t: f32,
    seed: u32,
}

impl Default for SpaceState {
    fn default() -> Self {
        Self { rings: Vec::new(), scale: 0.5, glow: 0.0, freeze_amt: 0.0, duck_dim: 0.0, in_avg: 0.0, cooldown: 0.0, mode: 0, mode_fade: 1.0, t: 0.0, seed: 7 }
    }
}

impl SpaceState {
    pub fn ring_count(&self) -> usize {
        self.rings.len()
    }

    fn rand(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Advance the animation by `dt` seconds.
    pub fn step(&mut self, dt: f32, i: &Inputs) {
        let dt = dt.clamp(0.0, 0.1);
        self.t += dt;
        let ease = |cur: f32, target: f32, tau: f32| cur + (target - cur) * (1.0 - (-dt / tau).exp());
        self.scale = ease(self.scale, i.size, 0.2 / 2.2); // ~200 ms to settle
        self.glow = ease(self.glow, (i.wet_level * 3.0).sqrt().min(1.0), 0.1);
        self.freeze_amt = ease(self.freeze_amt, if i.freeze { 1.0 } else { 0.0 }, 0.12);
        self.duck_dim = ease(self.duck_dim, (i.duck_gr_db / 30.0).clamp(0.0, 1.0), 0.05);
        if i.mode != self.mode {
            self.mode = i.mode;
            self.mode_fade = 0.0;
            self.rings.clear();
        }
        self.mode_fade = (self.mode_fade + dt / 0.25).min(1.0);

        // transients spawn rings (not while frozen)
        self.cooldown -= dt;
        let lvl = i.in_peak;
        if !i.freeze && lvl > 0.003 && lvl > self.in_avg * 2.0 && self.cooldown <= 0.0 {
            if self.rings.len() >= MAX_RINGS {
                self.rings.remove(0);
            }
            let (u, v) = (self.rand() * 0.5 - 0.25, self.rand() * 0.5 - 0.25);
            self.rings.push(Ring { age: 0.0, u, v, strength: lvl.sqrt().clamp(0.3, 1.0) });
            self.cooldown = 0.07;
        }
        self.in_avg = ease(self.in_avg, lvl, 0.15);

        // rings travel with Size, fade with Decay; frozen rings hold in place
        if !i.freeze {
            let life = life_s(i.decay_s);
            for r in self.rings.iter_mut() {
                r.age += dt;
            }
            self.rings.retain(|r| r.age < life);
        }
    }
}

fn life_s(decay: f32) -> f32 {
    (decay * 0.6).clamp(0.5, 8.0)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn col(c: [f32; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0] as u8, c[1] as u8, c[2] as u8, (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// World to screen. x across (-1..1), y up (-1..1), z depth (0 near .. 1 far).
struct Cam {
    centre: Pos2,
    sx: f32,
    sy: f32,
    wobble: f32,
    t: f32,
    rate: f32,
}

impl Cam {
    fn p(&self, x: f32, y: f32, z: f32) -> Pos2 {
        let k = 1.0 / (1.0 + z * 0.9);
        let w = if self.wobble > 0.0 { (self.t * self.rate * std::f32::consts::TAU + x * 4.0 + z * 6.0).sin() * self.wobble } else { 0.0 };
        pos2(self.centre.x + x * self.sx * k, self.centre.y - y * self.sy * k + w)
    }
}

type Line = Vec<[f32; 3]>;

fn rect_xy(w: f32, h: f32, z: f32, y0: f32) -> Line {
    vec![[-w, y0 - h, z], [w, y0 - h, z], [w, y0 + h, z], [-w, y0 + h, z], [-w, y0 - h, z]]
}

/// Wireframe of the current shape (world lines) plus a function turning ring (u, v, radius) into world points.
fn shape(mode: usize, scale: f32, width: f32) -> (Vec<Line>, [f32; 4]) {
    let s = 0.55 + 0.6 * scale;
    let spread = 1.0 + 0.25 * (width - 1.0);
    match mode {
        0 => {
            // plate: a sheet leaning back inside a frame, with four corner springs
            let (w, h) = (0.62 * s * spread, 0.5 * s);
            let sheet = vec![[-w, -h, 0.25], [w, -h, 0.25], [w, h, 0.65], [-w, h, 0.65], [-w, -h, 0.25]];
            let (fw, fh) = (0.9 * spread, 0.85);
            let frame = vec![[-fw, -fh, 0.45], [fw, -fh, 0.45], [fw, fh, 0.45], [-fw, fh, 0.45], [-fw, -fh, 0.45]];
            let mut lines = vec![frame, sheet.clone()];
            let corners = [([-w, -h, 0.25], [-fw, -fh, 0.45]), ([w, -h, 0.25], [fw, -fh, 0.45]), ([w, h, 0.65], [fw, fh, 0.45]), ([-w, h, 0.65], [-fw, fh, 0.45])];
            for (a, b) in corners {
                let mut spring = vec![a];
                for k in 1..8 {
                    let t = k as f32 / 8.0;
                    let off = if k % 2 == 0 { 0.035 } else { -0.035 };
                    spring.push([a[0] + (b[0] - a[0]) * t + off, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]);
                }
                spring.push(b);
                lines.push(spring);
            }
            for k in 1..4 {
                let t = k as f32 / 4.0;
                lines.push(vec![[-w, -h + 2.0 * h * t, 0.25 + 0.4 * t], [w, -h + 2.0 * h * t, 0.25 + 0.4 * t]]);
            }
            (lines, [w, h, 0.0, 0.0])
        }
        1 => {
            // hall: long room in perspective with an arched ceiling
            let (w, h, d) = (0.55 * s * spread, 0.36 * s, 1.5);
            let arch = |z: f32| -> Line { (0..=12).map(|k| { let a = std::f32::consts::PI * k as f32 / 12.0; [w * a.cos(), h * 0.4 + h * 0.75 * a.sin(), z] }).collect() };
            let mut lines = vec![rect_xy(w, h * 0.0 + 0.0, 0.0, 0.0)];
            lines.clear();
            lines.push(vec![[-w, -h, 0.0], [w, -h, 0.0]]);
            lines.push(vec![[-w, -h, d], [w, -h, d]]);
            lines.push(arch(0.0));
            lines.push(arch(d));
            lines.push(arch(d * 0.5));
            for x in [-w, w] {
                lines.push(vec![[x, -h, 0.0], [x, h * 0.4, 0.0]]);
                lines.push(vec![[x, -h, d], [x, h * 0.4, d]]);
                lines.push(vec![[x, -h, 0.0], [x, -h, d]]);
                lines.push(vec![[x, h * 0.4, 0.0], [x, h * 0.4, d]]);
            }
            lines.push(vec![[0.0, h * 1.15, 0.0], [0.0, h * 1.15, d]]);
            lines.push(vec![[0.0, -h, 0.0], [0.0, -h, d]]);
            (lines, [w, h, d, -h])
        }
        _ => {
            // room: a small box
            let (w, h, d) = (0.5 * s * spread, 0.38 * s, 0.7);
            let mut lines = vec![rect_xy(w, h, 0.0, 0.0), rect_xy(w, h, d, 0.0)];
            for (x, y) in [(-w, -h), (w, -h), (w, h), (-w, h)] {
                lines.push(vec![[x, y, 0.0], [x, y, d]]);
            }
            (lines, [w, h, d, -h])
        }
    }
}

pub fn draw(painter: &Painter, rect: Rect, st: &SpaceState, i: &Inputs) {
    let fz = st.freeze_amt;
    let brightness = {
        let d = ((i.damping_hz / 1000.0).ln() / 20f32.ln()).clamp(0.0, 1.0);
        let h = ((i.high_cut_hz / 1000.0).ln() / 20f32.ln()).clamp(0.0, 1.0);
        0.5 * (d + h)
    };
    let tone = mix(WARM, ACCENT, 0.35 + 0.65 * brightness); // dark settings run warmer
    let live = mix(tone, FREEZE, fz);
    let dim = 1.0 - 0.7 * st.duck_dim;
    let glow = (0.18 + 0.82 * st.glow) * (0.55 + 0.45 * brightness) * dim;
    let pulse = if fz > 0.01 { 1.0 + 0.2 * fz * (st.t * 2.2).sin() } else { 1.0 };
    let wob = i.mod_depth * 2.2;
    let cam = Cam { centre: rect.center() + egui::vec2(0.0, rect.height() * 0.03), sx: rect.width() * 0.66, sy: rect.height() * 0.55, wobble: wob, t: st.t, rate: i.mod_rate.min(2.0) * 0.5 };
    let fade = st.mode_fade;

    let (lines, ext) = shape(i.mode, st.scale, i.width);
    let wire_c = mix(WIRE, live, 0.25 * glow);
    for l in &lines {
        let pts: Vec<Pos2> = l.iter().map(|v| cam.p(v[0], v[1], v[2])).collect();
        painter.add(egui::Shape::line(pts.clone(), Stroke::new(1.0_f32, col(wire_c, (0.55 + 0.45 * glow) * fade))));
        if glow > 0.3 {
            painter.add(egui::Shape::line(pts, Stroke::new(2.5_f32, col(live, 0.10 * glow * pulse * fade))));
        }
    }

    // rings: ellipses on the floor (hall, room) or on the sheet (plate), squashed against the walls
    let life = life_s(i.decay_s);
    for r in &st.rings {
        let speed = 0.55 + 1.5 * st.scale; // world units per second
        let radius = r.age * speed * if i.mode == 2 { 0.7 } else { 1.0 };
        let a = ((1.0 - r.age / life).max(0.0)).powf(1.4) * r.strength * (0.35 + 0.65 * glow);
        let a = if fz > 0.5 { 0.45 * pulse * r.strength } else { a };
        if a < 0.01 {
            continue;
        }
        // points outside the walls are dropped, so a ring visibly runs into the wall and ends there
        let mut runs: Vec<Vec<Pos2>> = vec![Vec::new()];
        for k in 0..=48 {
            let th = std::f32::consts::TAU * k as f32 / 48.0;
            let (c, s) = (th.cos(), th.sin());
            let (inside, p) = match i.mode {
                0 => {
                    let (w, h) = (ext[0], ext[1]);
                    let u = r.u * w + radius * c * 0.9;
                    let v = r.v * h + radius * s * 0.8;
                    let t = ((v / h).clamp(-1.0, 1.0) + 1.0) * 0.5;
                    (u.abs() <= w && v.abs() <= h, cam.p(u.clamp(-w, w), v.clamp(-h, h), 0.25 + 0.4 * t))
                }
                _ => {
                    let (w, d, y) = (ext[0], ext[2], ext[3]);
                    let x = r.u * w + radius * c;
                    let z = d * 0.5 + r.v * d + radius * s * d * 0.8;
                    (x.abs() <= w && (0.0..=d).contains(&z), cam.p(x.clamp(-w, w), y, z.clamp(0.0, d)))
                }
            };
            if inside {
                runs.last_mut().unwrap().push(p);
            } else if !runs.last().unwrap().is_empty() {
                runs.push(Vec::new());
            }
        }
        for run in runs.into_iter().filter(|r| r.len() > 1) {
            painter.add(egui::Shape::line(run, Stroke::new(1.5_f32, col(live, a * dim * fade))));
        }
    }

    // the source point
    if i.mode != 0 {
        let p = cam.p(0.0, ext[3], ext[2] * 0.5);
        painter.circle_filled(p, 2.5, col(live, 0.5 * dim));
    }
    if fz > 0.05 {
        painter.text(rect.left_top() + egui::vec2(10.0, 8.0), egui::Align2::LEFT_TOP, "FROZEN", egui::FontId::new(11.0, egui::FontFamily::Name("mono".into())), col(FREEZE, fz));
    }
}

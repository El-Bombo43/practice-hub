//! The three reverb tanks. All are feedback delay networks (an orthogonal Hadamard matrix, so the loop is lossless until
//! the per-line decay filters take energy out), with the character coming from the delay lengths, the input diffusers,
//! the early-reflection taps and a direct diffused path (Plate). Per-line gain is derived from the *effective* line time
//! (delay plus the mean group delay of its allpass), so Decay is the true RT60 at 1 kHz in every mode.
use super::util::{Allpass, Delay};
use std::f64::consts::PI;

pub const LINES: usize = 8;
const BLOCK: f32 = 32.0;
/// Frozen loop gain: lossless, with a hair of margin so f32 rounding can never make it grow.
const FREEZE_GAIN: f32 = 0.999_999_5;
const BASS_CORNER_HZ: f64 = 400.0;
const DC_R: f32 = 0.9999;
const DC_G: f32 = (1.0 + DC_R) * 0.5;
const IN_SIGN_L: [f32; 8] = [1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0];
const IN_SIGN_R: [f32; 8] = [1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0];
const OUT_SIGN_L: [f32; 8] = [1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0];
const OUT_SIGN_R: [f32; 8] = [1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, -1.0];

pub struct Cfg {
    pub delays_ms: [f32; LINES],
    pub ap_ms: [f32; LINES],
    pub in_ap_ms: [f32; 4],
    /// Plate: stereo input is summed to mono before the tank.
    pub mono_in: bool,
    pub er_taps: usize,
    pub er_span_ms: (f32, f32),
    pub er_tau_ms: f32,
    pub in_gain: f32,
    pub out_gain: f32,
    pub direct_gain: f32,
    pub mod_ms: f32,
}

pub const PLATE: Cfg = Cfg {
    delays_ms: [19.3, 23.9, 29.3, 34.7, 41.3, 47.9, 55.1, 62.9],
    ap_ms: [1.9, 2.3, 2.9, 3.4, 4.1, 4.7, 5.3, 5.9],
    in_ap_ms: [4.77, 3.59, 12.73, 9.31],
    mono_in: true,
    er_taps: 0,
    er_span_ms: (0.0, 0.0),
    er_tau_ms: 1.0,
    in_gain: 0.5,
    out_gain: 0.31,
    direct_gain: 0.5,
    mod_ms: 3.0,
};
pub const HALL: Cfg = Cfg {
    delays_ms: [37.1, 47.9, 61.3, 74.9, 91.7, 109.1, 127.3, 149.9],
    ap_ms: [2.3, 2.9, 3.4, 4.1, 4.7, 5.3, 5.9, 6.7],
    in_ap_ms: [4.77, 3.59, 12.73, 9.31],
    mono_in: false,
    er_taps: 12,
    er_span_ms: (11.0, 110.0),
    er_tau_ms: 90.0,
    in_gain: 0.5,
    out_gain: 0.35,
    direct_gain: 0.0,
    mod_ms: 3.0,
};
pub const ROOM: Cfg = Cfg {
    delays_ms: [6.7, 8.9, 11.3, 13.7, 16.9, 19.9, 23.3, 27.7],
    ap_ms: [0.9, 1.1, 1.3, 1.7, 1.9, 2.3, 2.7, 3.1],
    in_ap_ms: [2.9, 2.1, 6.1, 4.3],
    mono_in: false,
    er_taps: 20,
    er_span_ms: (2.5, 48.0),
    er_tau_ms: 28.0,
    in_gain: 0.5,
    out_gain: 0.2,
    direct_gain: 0.0,
    mod_ms: 1.5,
};

pub fn cfg(mode: usize) -> &'static Cfg {
    match mode {
        0 => &PLATE,
        1 => &HALL,
        _ => &ROOM,
    }
}

/// Everything the tank needs, already smoothed by the engine. `scale` multiplies all delay times.
#[derive(Clone, Copy, Debug)]
pub struct TankParams {
    pub scale: f32,
    pub rt60: f32,
    pub diffusion: f32,
    pub damping_hz: f32,
    pub bass: f32,
    pub mod_rate: f32,
    pub mod_depth: f32,
    pub early_gain: f32,
}

struct Line {
    d: Delay,
    ap: Allpass,
    d_cur: f32,
    d_step: f32,
    interp_y: f32,
    lo: f32,
    z1: f32,
    z2: f32,
    dcx: f32,
    dcy: f32,
    g_n: f32,
    m: f32,
    ph1: f64,
    ph2: f64,
    f1: f64,
    f2: f64,
}

struct Er {
    l: Delay,
    r: Delay,
    /// (delay ms at scale 1, source channel, gain to L, gain to R)
    taps: Vec<(f32, usize, f32, f32)>,
}

pub struct Tank {
    cfg: &'static Cfg,
    fs: f32,
    lines: Vec<Line>,
    in_ap: [[Allpass; 4]; 2],
    er: Er,
    g_ap: f32,
    g_in: f32,
    a_lo: f32,
    dp: [f32; 5],
    er_gain: f32,
    er_step: f32,
    scale: f32,
    first: bool,
}

struct Lcg(u32);
impl Lcg {
    fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

fn ms_to_n(ms: f32, fs: f32) -> usize {
    ((ms * 0.001 * fs).round() as usize).max(1)
}

impl Tank {
    pub fn new(fs: f32, mode: usize) -> Self {
        let cfg = cfg(mode);
        let max_scale = 1.7;
        let lines = (0..LINES)
            .map(|i| {
                let cap = ms_to_n(cfg.delays_ms[i] * max_scale + cfg.mod_ms + 2.0, fs) + 8;
                let r = |k: f64| 0.31 + 0.11 * i as f64 * k;
                Line { d: Delay::new(cap), ap: Allpass::new(ms_to_n(cfg.ap_ms[i], fs)), d_cur: 0.0, d_step: 0.0, interp_y: 0.0, lo: 0.0, z1: 0.0, z2: 0.0, dcx: 0.0, dcy: 0.0, g_n: 0.5, m: 0.0, ph1: i as f64 * 0.9, ph2: i as f64 * 2.1 + 1.0, f1: r(1.0), f2: r(0.77) * 0.61 + 0.17 }
            })
            .collect();
        let mk = |k: usize| [0, 1, 2, 3].map(|j| Allpass::new(ms_to_n(cfg.in_ap_ms[j] * (1.0 + 0.07 * k as f32), fs)));
        let mut rng = Lcg(0x1234_5678 ^ mode as u32);
        let mut taps = Vec::new();
        for k in 0..cfg.er_taps {
            let (t0, t1) = cfg.er_span_ms;
            let t = t0 * (t1 / t0).powf((k as f32 + 0.6 * rng.f()) / cfg.er_taps as f32);
            let amp = 0.9 * (-t / cfg.er_tau_ms).exp() * if rng.f() < 0.5 { -1.0 } else { 1.0 };
            let pan = rng.f();
            taps.push((t, k % 2, amp * (1.0 - 0.7 * pan), amp * (0.3 + 0.7 * pan)));
        }
        let er_cap = ms_to_n(cfg.er_span_ms.1 * max_scale + 2.0, fs) + 4;
        Tank { cfg, fs, lines, in_ap: [mk(0), mk(1)], er: Er { l: Delay::new(er_cap), r: Delay::new(er_cap), taps }, g_ap: 0.0, g_in: 0.0, a_lo: 0.0, dp: [1.0, 0.0, 0.0, 0.0, 0.0], er_gain: 0.0, er_step: 0.0, scale: 1.0, first: true }
    }

    pub fn clear(&mut self) {
        for l in self.lines.iter_mut() {
            l.d.clear();
            l.ap.clear();
            l.interp_y = 0.0;
            l.lo = 0.0;
            l.z1 = 0.0;
            l.z2 = 0.0;
            l.dcx = 0.0;
            l.dcy = 0.0;
        }
        for c in self.in_ap.iter_mut() {
            c.iter_mut().for_each(|a| a.clear());
        }
        self.er.l.clear();
        self.er.r.clear();
    }

    /// Recompute gains, filters and delay targets. Called every 32 samples.
    pub fn control(&mut self, p: &TankParams) {
        let fs = self.fs as f64;
        self.g_ap = 0.62 * p.diffusion.clamp(0.0, 1.0);
        self.g_in = 0.75 * p.diffusion.clamp(0.0, 1.0);
        let dpc = butter_lp(p.damping_hz as f64, fs);
        self.dp = dpc.map(|v| v as f32);
        self.a_lo = (-2.0 * PI * BASS_CORNER_HZ / fs).exp() as f32;
        let target_er = p.early_gain;
        self.er_step = if self.first { 0.0 } else { (target_er - self.er_gain) / BLOCK };
        if self.first {
            self.er_gain = target_er;
        }
        self.scale = p.scale;
        for i in 0..LINES {
            let nom = self.cfg.delays_ms[i] * p.scale * 0.001 * self.fs;
            let excursion = (p.mod_depth * self.cfg.mod_ms * 0.001 * self.fs).min(nom * 0.3);
            let ln = &mut self.lines[i];
            let dt = BLOCK as f64 / fs;
            ln.ph1 = (ln.ph1 + 2.0 * PI * ln.f1 * p.mod_rate as f64 * dt) % (2.0 * PI);
            ln.ph2 = (ln.ph2 + 2.0 * PI * ln.f2 * p.mod_rate as f64 * dt) % (2.0 * PI);
            let m = 0.6 * ln.ph1.sin() + 0.4 * ln.ph2.sin();
            let target = (nom + excursion * m as f32).max(8.0);
            if self.first {
                ln.d_cur = target;
                ln.d_step = 0.0;
            } else {
                ln.d_step = (target - ln.d_cur) / BLOCK;
            }
            // mean group delay of the line's allpass over the octave around 1 kHz (it peaks near multiples of fs/N)
            let (g, nn) = (self.g_ap as f64, ln.ap.len() as f64);
            let mut tau_ap = 0.0;
            for k in 0..12 {
                let f = 707.0 * 2f64.powf((k as f64 + 0.5) / 12.0);
                tau_ap += nn * (1.0 - g * g) / (1.0 + g * g - 2.0 * g * (2.0 * PI * f * nn / fs).cos());
            }
            tau_ap /= 12.0;
            let d_eff = nom as f64 / fs + tau_ap / fs;
            let (g_n, m) = design_loss(d_eff, p.rt60 as f64, p.bass as f64, self.a_lo as f64, &dpc, fs);
            ln.g_n = g_n as f32;
            ln.m = m as f32;
        }
        self.first = false;
    }

    /// One stereo sample in, one stereo wet sample out. `fz` is the freeze amount, 0..1.
    #[inline]
    pub fn process(&mut self, inl: f32, inr: f32, fz: f32) -> (f32, f32) {
        let cfg = self.cfg;
        let (xl, xr) = if cfg.mono_in { let m = 0.5 * (inl + inr); (m, m) } else { (inl, inr) };
        let g_in = self.g_in;
        let mut dl = xl;
        for a in self.in_ap[0].iter_mut() {
            dl = a.process(dl, g_in);
        }
        let dr = {
            let mut v = xr;
            for a in self.in_ap[1].iter_mut() {
                v = a.process(v, g_in);
            }
            v
        };

        let mut ys = [0.0f32; LINES];
        let (a_lo, dp, g_ap) = (self.a_lo, self.dp, self.g_ap);
        for (i, ln) in self.lines.iter_mut().enumerate() {
            ln.d_cur += ln.d_step;
            let d = ln.d_cur;
            let ii = ((d - 0.5).floor() as usize).max(1);
            let eta = (d - ii as f32).clamp(0.1, 2.0);
            let c = (1.0 - eta) / (1.0 + eta);
            let v = c * ln.d.read(ii) + ln.d.read(ii + 1) - c * ln.interp_y;
            ln.interp_y = v;
            // decay filter: bass shelf then damping, scaled so the gain at 1 kHz is exactly the RT60 gain
            ln.lo = a_lo * ln.lo + (1.0 - a_lo) * v;
            let t = v + ln.m * ln.lo;
            let dv = dp[0] * t + ln.z1;
            ln.z1 = dp[1] * t - dp[3] * dv + ln.z2;
            ln.z2 = dp[2] * t - dp[4] * dv;
            let mut y = ln.g_n * dv;
            y += (v * FREEZE_GAIN - y) * fz;
            // DC blocker, then the in-loop allpass
            let dc = DC_G * (y - ln.dcx) + DC_R * ln.dcy; // gain (1+R)/2 keeps |H| <= 1 at every frequency
            ln.dcx = y;
            ln.dcy = dc;
            ys[i] = ln.ap.process(dc, g_ap);
        }
        let (mut ol, mut or) = (0.0f32, 0.0f32);
        for i in 0..LINES {
            ol += OUT_SIGN_L[i] * ys[i];
            or += OUT_SIGN_R[i] * ys[i];
        }
        // Hadamard matrix (orthogonal), then feed back with the injected input
        let mut h = ys;
        let mut s = 1;
        while s < LINES {
            let mut j = 0;
            while j < LINES {
                for k in j..j + s {
                    let (a, b) = (h[k], h[k + s]);
                    h[k] = a + b;
                    h[k + s] = a - b;
                }
                j += 2 * s;
            }
            s *= 2;
        }
        let norm = 0.353_553_4;
        let gi = cfg.in_gain;
        for (i, ln) in self.lines.iter_mut().enumerate() {
            ln.d.push(h[i] * norm + gi * (IN_SIGN_L[i] * dl + IN_SIGN_R[i] * dr));
        }
        let og = cfg.out_gain;
        let mut wl = og * ol + cfg.direct_gain * dl;
        let mut wr = og * or + cfg.direct_gain * dr;
        if cfg.er_taps > 0 {
            self.er_gain += self.er_step;
            self.er.l.push(inl);
            self.er.r.push(inr);
            let (mut el, mut er_) = (0.0f32, 0.0f32);
            for &(ms, ch, gl, gr) in &self.er.taps {
                let d = ms * self.scale * 0.001 * self.fs;
                let v = if ch == 0 { self.er.l.read_after_push(d) } else { self.er.r.read_after_push(d) };
                el += gl * v;
                er_ += gr * v;
            }
            wl += self.er_gain * 0.45 * el;
            wr += self.er_gain * 0.45 * er_;
        }
        (wl, wr)
    }
}

/// Complex response of the one-pole low-pass y = (1-a) x + a y1 at angular frequency `w`.
fn one_pole(a: f64, w: f64) -> (f64, f64) {
    // (1-a) / (1 - a e^{-jw})
    let (re, im) = (1.0 - a * w.cos(), a * w.sin());
    let k = (1.0 - a) / (re * re + im * im);
    (k * re, -k * im)
}

fn mag_t(kb: f64, l: (f64, f64)) -> f64 {
    let (re, im) = (1.0 + (kb - 1.0) * l.0, (kb - 1.0) * l.1);
    (re * re + im * im).sqrt()
}

/// Per-line loss filter: returns (overall gain, shelf amount m) such that the gain at 1 kHz is the RT60 gain
/// `10^(-3 d / rt60)` and the gain at 125 Hz is that of RT60 * bass. Never exceeds unity at DC.
pub fn design_loss(d: f64, rt60: f64, bass: f64, a_lo: f64, dp: &[f64; 5], fs: f64) -> (f64, f64) {
    let g_mid = 10f64.powf(-3.0 * d / rt60);
    let g_low = g_mid.powf(1.0 / bass).min(0.999_999);
    let (w1, w2) = (2.0 * PI * 1000.0 / fs, 2.0 * PI * 125.0 / fs);
    let (l1, l2) = (one_pole(a_lo, w1), one_pole(a_lo, w2));
    let (dm1, dm2) = (biquad_mag(dp, w1), biquad_mag(dp, w2));
    let ratio = |kb: f64| mag_t(kb, l2) * dm2 / (mag_t(kb, l1) * dm1);
    let kb = if (bass - 1.0).abs() < 1e-3 {
        1.0
    } else {
        let target = g_low / g_mid;
        let (mut lo, mut hi) = (0.02f64.ln(), 300f64.ln());
        for _ in 0..28 {
            let mid = 0.5 * (lo + hi);
            if ratio(mid.exp()) < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (0.5 * (lo + hi)).exp()
    };
    let mut g_n = g_mid / (mag_t(kb, l1) * dm1);
    if g_n * kb > 0.999_999 {
        g_n = 0.999_999 / kb;
    }
    (g_n, kb - 1.0)
}

/// Second-order Butterworth low-pass (flat below the corner, so Damping leaves the 1 kHz decay alone): [b0, b1, b2, a1, a2].
pub fn butter_lp(fc: f64, fs: f64) -> [f64; 5] {
    let fc = fc.clamp(200.0, fs * 0.45);
    let w = 2.0 * PI * fc / fs;
    let (s, c) = w.sin_cos();
    let alpha = s * std::f64::consts::FRAC_1_SQRT_2;
    let a0 = 1.0 + alpha;
    [(1.0 - c) / 2.0 / a0, (1.0 - c) / a0, (1.0 - c) / 2.0 / a0, -2.0 * c / a0, (1.0 - alpha) / a0]
}

fn biquad_mag(c: &[f64; 5], w: f64) -> f64 {
    let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2.0 * w).cos(), (2.0 * w).sin());
    let (nr, ni) = (c[0] + c[1] * c1 + c[2] * c2, -(c[1] * s1 + c[2] * s2));
    let (dr, di) = (1.0 + c[3] * c1 + c[4] * c2, -(c[3] * s1 + c[4] * s2));
    ((nr * nr + ni * ni) / (dr * dr + di * di)).sqrt()
}

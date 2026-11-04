//! Punch: a level-independent transient shaper. Everything is decided from *ratios* of envelope followers, so a quiet
//! and a loud hit get the same treatment. Zero latency.
use phub_common::dsp::{db_to_gain, tau_coef, Biquad, Ramp};

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// -1..1
    pub attack: f32,
    /// -1..1
    pub sustain: f32,
    pub attack_len_ms: f32,
    pub sustain_len_ms: f32,
    /// Hz; 20 or less is off
    pub focus: f32,
    pub link: bool,
    pub clip: bool,
    pub out_gain_db: f32,
    /// 0..1
    pub mix: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { attack: 0.0, sustain: 0.0, attack_len_ms: 10.0, sustain_len_ms: 150.0, focus: 20.0, link: true, clip: false, out_gain_db: 0.0, mix: 1.0 }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Meters {
    pub in_peak: f32,
    pub out_peak: f32,
    /// the largest gain change (dB, signed) seen since the last read
    pub gain_db: f32,
}

#[derive(Clone, Copy, Default)]
struct Detector {
    fast: f32,
    slow: f32,
    peak: f32,
    full: f32,
    det_fast: f32,
    weight: f32,
}

pub struct Engine {
    fs: f32,
    s: Settings,
    initialised: bool,
    attack: Ramp,
    sustain: Ramp,
    out_g: Ramp,
    mix: Ramp,
    focus: [Biquad; 2],
    focus_on: bool,
    det: [Detector; 2],
    gain_db: [f32; 2],
    pub meters: Meters,
}

const FLOOR_DB: f32 = -80.0;

pub fn smooth01(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Engine {
    pub fn new(fs: f64) -> Self {
        Self { fs: fs as f32, s: Settings::default(), initialised: false, attack: Ramp::new(0.0), sustain: Ramp::new(0.0), out_g: Ramp::new(1.0), mix: Ramp::new(1.0), focus: [Biquad::identity(); 2], focus_on: false, det: [Detector::default(); 2], gain_db: [0.0; 2], meters: Meters::default() }
    }
    pub fn sample_rate(&self) -> f32 {
        self.fs
    }
    pub fn reset(&mut self, fs: f64) {
        let (s, init) = (self.s, self.initialised);
        *self = Engine::new(fs);
        if init {
            self.set_settings(&s);
        }
    }
    pub fn gain_db(&self) -> f32 {
        self.gain_db[0]
    }

    pub fn set_settings(&mut self, s: &Settings) {
        let n20 = 0.02 * self.fs;
        if !self.initialised {
            self.attack.snap(s.attack);
            self.sustain.snap(s.sustain);
            self.out_g.snap(db_to_gain(s.out_gain_db));
            self.mix.snap(s.mix);
            self.initialised = true;
        }
        self.attack.set(s.attack, n20);
        self.sustain.set(s.sustain, n20);
        self.out_g.set(db_to_gain(s.out_gain_db), n20);
        self.mix.set(s.mix, n20);
        if s.focus != self.s.focus || self.s.focus == 0.0 {
            self.focus_on = s.focus > 20.5;
            if self.focus_on {
                for f in self.focus.iter_mut() {
                    f.set_coeffs(&Biquad::highpass(self.fs as f64, s.focus as f64, std::f64::consts::FRAC_1_SQRT_2));
                }
            }
        }
        self.s = *s;
    }

    pub fn process(&mut self, l: &mut [f32], mut r: Option<&mut [f32]>) {
        let fs = self.fs;
        let s = self.s;
        let nch = if r.is_some() { 2 } else { 1 };
        let (fa, fr) = (tau_coef(0.0005, fs), tau_coef(0.015, fs));
        let slow_rel = tau_coef(0.002, fs);
        let w_rise = tau_coef(0.0015, fs);
        let (gsm_up, gsm) = (tau_coef(0.0002, fs), tau_coef(0.003, fs));
        let sa = tau_coef(s.attack_len_ms * 0.001, fs);
        let hp_rel = tau_coef(6.0 * s.sustain_len_ms * 0.001, fs);
        let (mut in_pk, mut out_pk, mut gmax) = (0.0f32, 0.0f32, 0.0f32);
        let eps = 1e-5f32;
        for i in 0..l.len() {
            let raw = [l[i], r.as_ref().map_or(0.0, |r| r[i])];
            in_pk = in_pk.max(raw[0].abs()).max(raw[1].abs());
            let att = self.attack.tick();
            let sus = self.sustain.tick();
            let mut y = raw;
            if att != 0.0 || sus != 0.0 || self.gain_db != [0.0; 2] {
                // detector input: Focus high-passes the detector only
                let mut a = [0.0f32; 2];
                let mut full = [0.0f32; 2];
                for ch in 0..nch {
                    full[ch] = raw[ch].abs();
                    a[ch] = if self.focus_on { self.focus[ch].process(raw[ch] as f64).abs() as f32 } else { full[ch] };
                }
                let groups: [usize; 2] = if s.link || nch == 1 { [0, 0] } else { [0, 1] };
                if s.link && nch == 2 {
                    a[0] = a[0].max(a[1]);
                    full[0] = full[0].max(full[1]);
                }
                for ch in 0..nch {
                    let g = groups[ch];
                    if ch > 0 && g == 0 {
                        self.gain_db[1] = self.gain_db[0];
                        continue;
                    }
                    let d = &mut self.det[ch];
                    // amplitude envelope (instant attack, 15 ms release: little ripple on a steady tone)
                    d.det_fast = a[ch].max(d.det_fast * fr);
                    d.fast += (d.det_fast - d.fast) * (1.0 - fa);
                    d.slow += (d.fast - d.slow) * (1.0 - if d.fast > d.slow { sa } else { slow_rel });
                    d.peak = d.fast.max(d.peak * hp_rel);
                    d.full = full[ch].max(d.full * fr);
                    let lvl = 20.0 * (d.fast.max(d.full) + 1e-9).log10();
                    let lev = smooth01((lvl - FLOOR_DB) / 10.0);
                    let w_target = if self.focus_on { (4.0 * d.fast / (d.full + eps)).min(1.0) } else { 1.0 };
                    // with Focus on, trust the band only after ~1.5 ms of evidence (a high-pass passes the first
                    // samples of any onset, whatever the tone is)
                    d.weight = if w_target > d.weight { d.weight + (w_target - d.weight) * (1.0 - w_rise) } else { w_target };
                    let weight = d.weight;
                    let diff_a = 20.0 * ((d.fast + eps) / (d.slow + eps)).log10();
                    let diff_s = 20.0 * ((d.peak + eps) / (d.fast + eps)).log10();
                    let ga = att * 12.0 * smooth01(diff_a / 3.0);
                    let gs = sus * 18.0 * smooth01((diff_s - 0.6) / 1.6);
                    let target = (ga + gs) * lev * weight;
                    let c = if target.abs() > self.gain_db[ch].abs() { gsm_up } else { gsm };
                    self.gain_db[ch] = target + (self.gain_db[ch] - target) * c;
                    if self.gain_db[ch].abs() < 1e-4 && target == 0.0 {
                        self.gain_db[ch] = 0.0;
                    }
                }
                if nch == 1 || (s.link && nch == 2) {
                    self.gain_db[1] = self.gain_db[0];
                }
                for ch in 0..nch {
                    let g = if self.gain_db[ch] == 0.0 { 1.0 } else { db_to_gain(self.gain_db[ch]) };
                    y[ch] = raw[ch] * g;
                }
                let gd = self.gain_db[0];
                if gd.abs() > gmax.abs() {
                    gmax = gd;
                }
            }
            if s.clip {
                const T: f32 = 0.891; // -1 dBFS
                for v in y.iter_mut() {
                    let a = v.abs();
                    if a > T {
                        *v = v.signum() * (T + (1.0 - T) * ((a - T) / (1.0 - T)).tanh());
                    }
                }
            }
            if self.out_g.moving() || self.out_g.cur != 1.0 {
                let g = self.out_g.tick();
                y = [y[0] * g, y[1] * g];
            }
            let m = self.mix.tick();
            let out = if m >= 1.0 { y } else { [raw[0] + (y[0] - raw[0]) * m, raw[1] + (y[1] - raw[1]) * m] };
            out_pk = out_pk.max(out[0].abs()).max(out[1].abs());
            l[i] = out[0];
            if let Some(r) = r.as_mut() {
                r[i] = out[1];
            }
        }
        self.meters.in_peak = self.meters.in_peak.max(in_pk);
        self.meters.out_peak = self.meters.out_peak.max(out_pk);
        if gmax.abs() > self.meters.gain_db.abs() {
            self.meters.gain_db = gmax;
        }
    }

    pub fn take_meters(&mut self) -> Meters {
        std::mem::take(&mut self.meters)
    }
}

//! One EQ band at run time: smoothing, coefficient updates, structure crossfades and the dynamic detector.

use super::design::{band_pass, design, Biquad, DesignParams, FilterType, Sections, MAX_SECTIONS};
use super::engine::BandSettings;

/// Samples per control block: parameters and coefficients update this often.
pub const BLOCK: usize = 32;
const FADE_SECONDS: f64 = 0.010;
const SMOOTH_TAU: f64 = 0.007;
/// Soft knee width and the span (above Threshold) over which the full Range is reached.
const KNEE_DB: f64 = 6.0;
const FULL_RANGE_DB: f64 = 12.0;

type State = [[[f64; 2]; MAX_SECTIONS]; 2];

#[derive(Clone)]
struct Chain {
    sections: Sections,
    st: State,
}

impl Chain {
    fn new(sections: Sections) -> Self {
        Self { sections, st: [[[0.0; 2]; MAX_SECTIONS]; 2] }
    }

    fn run(&mut self, nch: usize, input: &[[f64; BLOCK]; 2], out: &mut [[f64; BLOCK]; 2], n: usize) {
        for ch in 0..nch {
            for i in 0..n {
                let mut y = input[ch][i];
                for k in 0..self.sections.n {
                    let b = &self.sections.s[k];
                    let z = &mut self.st[ch][k];
                    let x = y;
                    y = b.b0 * x + z[0];
                    z[0] = b.b1 * x - b.a1 * y + z[1];
                    z[1] = b.b2 * x - b.a2 * y;
                }
                out[ch][i] = y;
            }
        }
    }

    fn flush(&mut self) {
        for ch in self.st.iter_mut() {
            for s in ch.iter_mut() {
                for z in s.iter_mut() {
                    if z.abs() < 1e-30 {
                        *z = 0.0;
                    }
                }
            }
        }
    }
}

/// What the detector gets from the soft knee: 0 below Threshold-3 dB, 1 from Threshold+9 dB (12 dB span).
pub fn dynamic_fraction(level_db: f64, thresh_db: f64) -> f64 {
    let x = level_db - thresh_db;
    let half = KNEE_DB / 2.0;
    let g = if x <= -half {
        0.0
    } else if x < half {
        (x + half).powi(2) / (2.0 * KNEE_DB)
    } else {
        x
    };
    (g / FULL_RANGE_DB).clamp(0.0, 1.0)
}

pub struct BandRt {
    pub s: BandSettings,
    active: bool,
    // smoothed values
    ln_f: f64,
    gain: f64,
    q: f64,
    thresh: f64,
    range: f64,
    mix: f64,
    dyn_amt: f64,
    // filter
    chain: Chain,
    cur: (FilterType, u32),
    designed: (f64, f64, f64),
    old: Option<Chain>,
    fade: f64,
    // detector
    det: Biquad,
    det_st: [[f64; 2]; 2],
    det_designed: (f64, f64),
    env: f64,
    dyn_f: f64,
    /// Live dynamic movement in dB (already scaled), for the editor.
    pub dyn_db: f64,
}

impl BandRt {
    pub fn new() -> Self {
        Self {
            s: BandSettings::default(),
            active: false,
            ln_f: 1000f64.ln(),
            gain: 0.0,
            q: 1.0,
            thresh: -20.0,
            range: -6.0,
            mix: 0.0,
            dyn_amt: 0.0,
            chain: Chain::new(Sections { n: 0, s: [Biquad::IDENTITY; MAX_SECTIONS] }),
            cur: (FilterType::Bell, 12),
            designed: (f64::NAN, f64::NAN, f64::NAN),
            old: None,
            fade: 0.0,
            det: Biquad::IDENTITY,
            det_st: [[0.0; 2]; 2],
            det_designed: (f64::NAN, f64::NAN),
            env: 0.0,
            dyn_f: 0.0,
            dyn_db: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.active = false;
        self.mix = 0.0;
        self.old = None;
        self.env = 0.0;
        self.dyn_f = 0.0;
        self.dyn_db = 0.0;
        self.chain = Chain::new(self.chain.sections);
        self.det_st = [[0.0; 2]; 2];
        self.designed = (f64::NAN, f64::NAN, f64::NAN);
        self.det_designed = (f64::NAN, f64::NAN);
    }

    /// True if this band changes the signal at all right now (used by the engine to skip idle bands).
    pub fn is_running(&self) -> bool {
        self.active
    }

    fn structure(&self) -> (FilterType, u32) {
        (self.s.ftype, if self.s.ftype.is_cut() { self.s.slope } else { 0 })
    }

    /// Process `n` samples of every channel in place. `sc` is the external sidechain for this block, if any.
    pub fn process(&mut self, nch: usize, buf: &mut [[f64; BLOCK]; 2], n: usize, sc: Option<&[[f64; BLOCK]; 2]>, fs: f64, scale: f64) {
        let want = self.s.used && (self.s.enabled || self.mix > 1e-6);
        if !want {
            if self.active {
                self.reset();
            }
            self.dyn_db = 0.0;
            return;
        }
        let nf = n as f64;
        let alpha = 1.0 - (-nf / (fs * SMOOTH_TAU)).exp();
        let fade_step = nf / (fs * FADE_SECONDS);

        let f_t = (self.s.freq as f64).clamp(10.0, 30_000.0).ln();
        let first = !self.active;
        if first {
            self.active = true;
            self.ln_f = f_t;
            self.gain = self.s.gain_db as f64;
            self.q = self.s.q as f64;
            self.thresh = self.s.thresh_db as f64;
            self.range = self.s.range_db as f64;
            self.mix = if self.s.enabled { 1.0 } else { 0.0 };
            self.dyn_amt = if self.dynamic_target() { 1.0 } else { 0.0 };
            self.cur = self.structure();
            self.chain = Chain::new(self.chain.sections);
            self.designed = (f64::NAN, f64::NAN, f64::NAN);
        } else {
            self.ln_f += (f_t - self.ln_f) * alpha;
            self.gain += (self.s.gain_db as f64 - self.gain) * alpha;
            self.q = (self.q.ln() + ((self.s.q as f64).ln() - self.q.ln()) * alpha).exp(); // smoothed in the log domain
            self.thresh += (self.s.thresh_db as f64 - self.thresh) * alpha;
            self.range += (self.s.range_db as f64 - self.range) * alpha;
            let mt = if self.s.enabled { 1.0 } else { 0.0 };
            self.mix += (mt - self.mix).clamp(-fade_step, fade_step);
            let dt = if self.dynamic_target() { 1.0 } else { 0.0 };
            self.dyn_amt += (dt - self.dyn_amt).clamp(-fade_step, fade_step);
        }

        // a change of filter type or cut slope: crossfade from the old chain to a fresh one
        let structure = self.structure();
        if structure != self.cur {
            if self.mix > 0.0 {
                self.old = Some(self.chain.clone());
                self.fade = 1.0;
            }
            self.chain = Chain::new(self.chain.sections);
            self.cur = structure;
            self.designed = (f64::NAN, f64::NAN, f64::NAN);
        }

        // coefficients: static gain plus the dynamic movement from the last block's detector, both scaled
        let uses_gain = self.s.ftype.uses_gain();
        let dyn_db = if uses_gain { self.dyn_amt * self.range * self.dyn_f } else { 0.0 };
        let total = if uses_gain { scale * (self.gain + dyn_db) } else { 0.0 };
        self.dyn_db = scale * dyn_db;
        let q_now = self.q;
        if self.designed.0.is_nan()
            || (self.ln_f - self.designed.0).abs() > 1e-5
            || (total - self.designed.1).abs() > 1e-4
            || (q_now - self.designed.2).abs() > 1e-5 * q_now.max(1.0)
        {
            let p = DesignParams { ftype: self.s.ftype, freq: self.ln_f.exp(), gain_db: total, q: q_now, slope: self.s.slope };
            self.chain.sections = design(&p, fs);
            self.designed = (self.ln_f, total, q_now);
        }

        let detecting = self.dynamic_target() || self.dyn_amt > 0.0;
        if detecting {
            if self.det_designed.0.is_nan() || (self.ln_f - self.det_designed.0).abs() > 1e-5 || (q_now - self.det_designed.1).abs() > 1e-5 * q_now.max(1.0) {
                let w0 = 2.0 * std::f64::consts::PI * super::design::clamp_freq(self.ln_f.exp(), fs) / fs;
                self.det = band_pass(w0, q_now);
                self.det_designed = (self.ln_f, q_now);
            }
            let src: &[[f64; BLOCK]; 2] = if self.s.ext { sc.unwrap_or(&[[0.0; BLOCK]; 2]) } else { buf };
            let a_att = 1.0 - (-1.0 / (self.s.attack_ms as f64 * 1e-3 * fs)).exp();
            let a_rel = 1.0 - (-1.0 / (self.s.release_ms as f64 * 1e-3 * fs)).exp();
            let d = self.det;
            let mut env = self.env;
            for i in 0..n {
                let mut m = 0.0f64;
                for ch in 0..nch {
                    let x = src[ch][i];
                    let z = &mut self.det_st[ch];
                    let y = d.b0 * x + z[0];
                    z[0] = d.b1 * x - d.a1 * y + z[1];
                    z[1] = d.b2 * x - d.a2 * y;
                    m = m.max(y.abs());
                }
                env += (m - env) * if m > env { a_att } else { a_rel };
            }
            self.env = env;
            for ch in self.det_st.iter_mut() {
                for z in ch.iter_mut() {
                    if z.abs() < 1e-30 {
                        *z = 0.0;
                    }
                }
            }
            let level_db = 20.0 * (env + 1e-9).log10();
            self.dyn_f = dynamic_fraction(level_db, self.thresh);
        } else {
            self.env = 0.0;
            self.dyn_f = 0.0;
        }

        // run the filter(s)
        let input = *buf;
        if self.chain.sections.n > 0 || self.old.is_some() {
            let mut new_out = input;
            self.chain.run(nch, &input, &mut new_out, n);
            let mut result = new_out;
            if let Some(old) = self.old.as_mut() {
                let mut old_out = input;
                old.run(nch, &input, &mut old_out, n);
                let (t0, t1) = (self.fade, (self.fade - fade_step).max(0.0));
                for ch in 0..nch {
                    for i in 0..n {
                        let t = t0 + (t1 - t0) * (i as f64 / nf);
                        result[ch][i] = new_out[ch][i] * (1.0 - t) + old_out[ch][i] * t;
                    }
                }
                self.fade = t1;
                old.flush();
                if t1 <= 0.0 {
                    self.old = None;
                }
            }
            if self.mix < 1.0 {
                let m = self.mix;
                for ch in 0..nch {
                    for i in 0..n {
                        result[ch][i] = input[ch][i] + m * (result[ch][i] - input[ch][i]);
                    }
                }
            }
            for ch in 0..nch {
                buf[ch][..n].copy_from_slice(&result[ch][..n]);
            }
        }
        self.chain.flush();

        if !self.s.enabled && self.mix <= 1e-6 {
            self.reset();
        }
    }

    fn dynamic_target(&self) -> bool {
        self.s.dynamic && self.s.ftype.allows_dynamic()
    }
}

impl Default for BandRt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knee_and_range_mapping() {
        assert_eq!(dynamic_fraction(-60.0, -20.0), 0.0);
        assert_eq!(dynamic_fraction(-23.0, -20.0), 0.0); // knee starts 3 dB below
        assert!((dynamic_fraction(-20.0, -20.0) - 0.75 / 12.0).abs() < 1e-9); // at the threshold the knee gives 0.75 dB of the 12
        assert!((dynamic_fraction(-14.0, -20.0) - 0.5).abs() < 1e-9); // 6 dB over -> half the range
        assert_eq!(dynamic_fraction(-8.0, -20.0), 1.0); // 12 dB over -> full range
        assert_eq!(dynamic_fraction(0.0, -20.0), 1.0);
    }
}

//! Small DSP building blocks shared by the plugins.
use std::f64::consts::PI;
use std::sync::atomic::{AtomicU32, Ordering};

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}
#[inline]
pub fn gain_to_db(g: f32) -> f32 {
    20.0 * g.max(1e-12).log10()
}

/// A linear ramp that lands exactly on its target.
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    pub cur: f32,
    pub target: f32,
    step: f32,
}
impl Ramp {
    pub fn new(v: f32) -> Self {
        Self { cur: v, target: v, step: 0.0 }
    }
    pub fn snap(&mut self, v: f32) {
        *self = Self::new(v);
    }
    pub fn set(&mut self, target: f32, samples: f32) {
        if target != self.target {
            self.target = target;
            self.step = (target - self.cur) / samples.max(1.0);
        }
    }
    pub fn moving(&self) -> bool {
        self.cur != self.target
    }
    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.cur != self.target {
            let next = self.cur + self.step;
            if (self.step > 0.0 && next >= self.target) || (self.step < 0.0 && next <= self.target) || self.step == 0.0 {
                self.cur = self.target;
            } else {
                self.cur = next;
            }
        }
        self.cur
    }
}

/// Second-order section (TDF2, f64 state).
#[derive(Clone, Copy, Default, Debug)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
    z1: f64,
    z2: f64,
}
impl Biquad {
    pub fn identity() -> Self {
        Self { b0: 1.0, ..Default::default() }
    }
    /// RBJ designs. `q` is the quality factor (Butterworth = 1/sqrt 2).
    pub fn lowpass(fs: f64, fc: f64, q: f64) -> Self {
        let (c, alpha) = Self::prep(fs, fc, q);
        Self::norm((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }
    pub fn highpass(fs: f64, fc: f64, q: f64) -> Self {
        let (c, alpha) = Self::prep(fs, fc, q);
        Self::norm((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }
    /// Constant 0 dB peak band-pass.
    pub fn bandpass(fs: f64, fc: f64, q: f64) -> Self {
        let (c, alpha) = Self::prep(fs, fc, q);
        Self::norm(alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }
    pub fn high_shelf(fs: f64, fc: f64, gain_db: f64) -> Self {
        let a = 10f64.powf(gain_db / 40.0);
        let w = 2.0 * PI * fc.clamp(10.0, fs * 0.49) / fs;
        let (s, c) = w.sin_cos();
        let alpha = s / 2.0 * 2f64.sqrt(); // S = 1
        let sq = 2.0 * a.sqrt() * alpha;
        Self::norm(a * ((a + 1.0) + (a - 1.0) * c + sq), -2.0 * a * ((a - 1.0) + (a + 1.0) * c), a * ((a + 1.0) + (a - 1.0) * c - sq), (a + 1.0) - (a - 1.0) * c + sq, 2.0 * ((a - 1.0) - (a + 1.0) * c), (a + 1.0) - (a - 1.0) * c - sq)
    }
    fn prep(fs: f64, fc: f64, q: f64) -> (f64, f64) {
        let w = 2.0 * PI * fc.clamp(1.0, fs * 0.49) / fs;
        let (s, c) = w.sin_cos();
        (c, s / (2.0 * q))
    }
    fn norm(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Self {
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, z1: 0.0, z2: 0.0 }
    }
    /// Replace the coefficients but keep the state.
    pub fn set_coeffs(&mut self, o: &Biquad) {
        self.b0 = o.b0;
        self.b1 = o.b1;
        self.b2 = o.b2;
        self.a1 = o.a1;
        self.a2 = o.a2;
    }
    #[inline]
    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
    pub fn clear(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// One-pole smoother coefficient for a time constant (seconds): y += (x - y) * (1 - coef).
pub fn tau_coef(tau_s: f32, fs: f32) -> f32 {
    (-1.0 / (tau_s.max(1e-6) * fs)).exp()
}

/// A lock-free f32 that keeps the largest value written since it was last taken (audio thread -> editor).
pub struct PeakCell(AtomicU32);
impl PeakCell {
    pub const fn new() -> Self {
        Self(AtomicU32::new(0))
    }
    /// Non-negative values only (their bit patterns order like the numbers).
    pub fn max(&self, v: f32) {
        self.0.fetch_max(v.max(0.0).to_bits(), Ordering::Relaxed);
    }
    pub fn store(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
    pub fn take(&self) -> f32 {
        f32::from_bits(self.0.swap(0, Ordering::Relaxed))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}
impl Default for PeakCell {
    fn default() -> Self {
        Self::new()
    }
}

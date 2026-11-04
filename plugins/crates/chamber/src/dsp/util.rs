//! Small building blocks: smoothing ramps, delay lines, allpasses, filters.

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
        self.cur = v;
        self.target = v;
        self.step = 0.0;
    }
    /// Head for `target`, arriving after `samples` samples.
    pub fn set(&mut self, target: f32, samples: f32) {
        if target != self.target {
            self.target = target;
            self.step = (target - self.cur) / samples.max(1.0);
        }
    }
    pub fn moving(&self) -> bool {
        self.cur != self.target
    }
    /// Advance `n` samples and return the new value.
    pub fn tick(&mut self, n: f32) -> f32 {
        if self.cur != self.target {
            let next = self.cur + self.step * n;
            if (self.step > 0.0 && next >= self.target) || (self.step < 0.0 && next <= self.target) || self.step == 0.0 {
                self.cur = self.target;
            } else {
                self.cur = next;
            }
        }
        self.cur
    }
}

/// Power-of-two circular buffer. `read(n)` is the sample written `n` pushes ago *before* the next push (n >= 1).
#[derive(Clone)]
pub struct Delay {
    buf: Vec<f32>,
    mask: usize,
    w: usize,
}

impl Delay {
    pub fn new(min_len: usize) -> Self {
        let len = (min_len + 2).next_power_of_two().max(16);
        Self { buf: vec![0.0; len], mask: len - 1, w: 0 }
    }
    pub fn capacity(&self) -> usize {
        self.buf.len() - 2
    }
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.buf[self.w] = x;
        self.w = (self.w + 1) & self.mask;
    }
    #[inline]
    pub fn read(&self, n: usize) -> f32 {
        self.buf[(self.w.wrapping_sub(n)) & self.mask]
    }
    /// Linear-interpolated read of a fractional delay `d >= 0` *after* pushing the current sample (d = 0 is the input).
    #[inline]
    pub fn read_after_push(&self, d: f32) -> f32 {
        let i = d as usize;
        let f = d - i as f32;
        let a = self.read(i + 1);
        if f == 0.0 {
            return a;
        }
        let b = self.read(i + 2);
        a + (b - a) * f
    }
    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
    }
}

/// Schroeder allpass with a fixed integer delay. Unity magnitude at every frequency; mean group delay = its delay.
#[derive(Clone)]
pub struct Allpass {
    d: Delay,
    n: usize,
}

impl Allpass {
    pub fn new(n: usize) -> Self {
        let n = n.max(1);
        Self { d: Delay::new(n + 1), n }
    }
    pub fn len(&self) -> usize {
        self.n
    }
    #[inline]
    pub fn process(&mut self, x: f32, g: f32) -> f32 {
        let old = self.d.read(self.n);
        let w = x + g * old;
        self.d.push(w);
        old - g * w
    }
    pub fn clear(&mut self) {
        self.d.clear();
    }
}

/// Second-order section in transposed direct form II, f64 state.
#[derive(Clone, Copy, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    pub fn set_lowpass(&mut self, fs: f64, fc: f64) {
        self.set(fs, fc, false)
    }
    pub fn set_highpass(&mut self, fs: f64, fc: f64) {
        self.set(fs, fc, true)
    }
    fn set(&mut self, fs: f64, fc: f64, hp: bool) {
        let fc = fc.clamp(1.0, fs * 0.49);
        let w = 2.0 * std::f64::consts::PI * fc / fs;
        let (s, c) = w.sin_cos();
        let alpha = s / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = if hp { ((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0) } else { ((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0) };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let x = x as f64;
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y as f32
    }
    pub fn clear(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

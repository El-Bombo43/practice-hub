#![allow(dead_code)]
use chamber::dsp::{Engine, Settings};

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
    }
}

/// A fully wet, unfiltered setting: the impulse response of the tank alone.
pub fn wet(mode: usize, decay: f32) -> Settings {
    Settings { mode, mix: 1.0, predelay_ms: 0.0, decay_s: decay, early_db: -24.0, low_cut: 20.0, high_cut: 20_000.0, ..Default::default() }
}

/// Process `input` in blocks of `blk` samples; returns (L, R).
pub fn run(e: &mut Engine, s: &Settings, input: &[(f32, f32)], blk: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut ol, mut or) = (Vec::with_capacity(input.len()), Vec::with_capacity(input.len()));
    for ch in input.chunks(blk) {
        let mut l: Vec<f32> = ch.iter().map(|v| v.0).collect();
        let mut r: Vec<f32> = ch.iter().map(|v| v.1).collect();
        e.set_settings(s);
        e.process(&mut l, &mut r);
        ol.extend(l);
        or.extend(r);
    }
    (ol, or)
}

pub fn impulse(n: usize) -> Vec<(f32, f32)> {
    let mut v = vec![(0.0, 0.0); n];
    v[0] = (1.0, 1.0);
    v
}

pub fn ir(s: &Settings, fs: f64, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let mut e = Engine::new(fs);
    let n = (seconds * fs as f32) as usize;
    run(&mut e, s, &impulse(n), 512)
}

pub fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt() as f32
}

/// Octave band filter (4th order each side) around `fc`.
pub fn octave(x: &[f32], fs: f64, fc: f64) -> Vec<f32> {
    use chamber::dsp::util::Biquad;
    let (mut h1, mut h2, mut l1, mut l2) = (Biquad::default(), Biquad::default(), Biquad::default(), Biquad::default());
    let (lo, hi) = (fc / 2f64.sqrt(), (fc * 2f64.sqrt()).min(fs * 0.45));
    h1.set_highpass(fs, lo);
    h2.set_highpass(fs, lo);
    l1.set_lowpass(fs, hi);
    l2.set_lowpass(fs, hi);
    x.iter().map(|v| l2.process(l1.process(h2.process(h1.process(*v))))).collect()
}

/// RT60 from the Schroeder backward integral, fitted between -5 and -25 dB and extrapolated.
pub fn rt60(x: &[f32], fs: f64) -> f64 {
    let mut e = vec![0.0f64; x.len()];
    let mut acc = 0.0;
    for i in (0..x.len()).rev() {
        acc += (x[i] as f64).powi(2);
        e[i] = acc;
    }
    let total = e[0].max(1e-300);
    let db: Vec<f64> = e.iter().map(|v| 10.0 * (v / total).max(1e-300).log10()).collect();
    let i5 = db.iter().position(|v| *v <= -5.0).unwrap();
    let i25 = db.iter().position(|v| *v <= -25.0).unwrap();
    // least squares line through the points between
    let n = (i25 - i5 + 1) as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0);
    for i in i5..=i25 {
        let t = i as f64 / fs;
        sx += t;
        sy += db[i];
        sxx += t * t;
        sxy += t * db[i];
    }
    let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
    -60.0 / slope
}

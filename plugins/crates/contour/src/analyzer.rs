//! Spectrum analyser: the audio thread drops mono samples into a ring (only while the editor is open); the editor
//! thread windows and transforms them.

use parking_lot::Mutex;
use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub const RING: usize = 16_384;
pub const FFT_SIZE: usize = 8192;

pub struct Ring {
    pub pre: Vec<f32>,
    pub post: Vec<f32>,
    pub pos: usize,
}

impl Ring {
    pub fn new() -> Self {
        Self { pre: vec![0.0; RING], post: vec![0.0; RING], pos: 0 }
    }

    pub fn push(&mut self, pre: &[f32], post: &[f32]) {
        for (a, b) in pre.iter().zip(post.iter()) {
            self.pre[self.pos] = *a;
            self.post[self.pos] = *b;
            self.pos = (self.pos + 1) % RING;
        }
    }

    /// The most recent `FFT_SIZE` samples in time order.
    pub fn latest(&self, which_pre: bool, out: &mut [f32]) {
        let src = if which_pre { &self.pre } else { &self.post };
        let n = out.len();
        for (i, o) in out.iter_mut().enumerate() {
            *o = src[(self.pos + RING - n + i) % RING];
        }
    }
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared between the audio and editor threads.
pub struct Shared {
    pub ring: Mutex<Ring>,
    pub dyn_db: [std::sync::atomic::AtomicU32; 16],
    pub solo: std::sync::atomic::AtomicI32,
    pub sample_rate: std::sync::atomic::AtomicU32,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            ring: Mutex::new(Ring::new()),
            dyn_db: std::array::from_fn(|_| std::sync::atomic::AtomicU32::new(0)),
            solo: std::sync::atomic::AtomicI32::new(-1),
            sample_rate: std::sync::atomic::AtomicU32::new(48_000f32.to_bits()),
        }
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

/// Smoothed, tilted spectrum over log-spaced display points. One per trace (pre / post).
pub struct Trace {
    pub db: Vec<f32>,
}

pub struct Analyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    scratch_in: Vec<f32>,
    spectrum: Vec<realfft::num_complex::Complex<f32>>,
    pub points: usize,
    pub pre: Trace,
    pub post: Trace,
}

pub const DB_FLOOR: f32 = -90.0;
/// Display tilt: +4.5 dB per octave above 1 kHz (and the same below), so a typical mix looks roughly flat.
pub const TILT_DB_PER_OCT: f32 = 4.5;

impl Analyzer {
    pub fn new(points: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let window = (0..FFT_SIZE).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos()).collect();
        Self {
            spectrum: fft.make_output_vec(),
            fft,
            window,
            scratch_in: vec![0.0; FFT_SIZE],
            points,
            pre: Trace { db: vec![DB_FLOOR; points] },
            post: Trace { db: vec![DB_FLOOR; points] },
        }
    }

    /// Frequency (Hz) of display point `i` of `points`, 10 Hz to 30 kHz, log spaced.
    pub fn point_freq(&self, i: usize) -> f32 {
        10.0 * (3000.0f32).powf(i as f32 / (self.points - 1) as f32)
    }

    /// Update one trace from the ring. `decay` is the per-frame fall (dB) for a falling level: bigger is faster.
    pub fn update(&mut self, ring: &Ring, pre: bool, fs: f32, decay_db: f32) {
        ring.latest(pre, &mut self.scratch_in);
        for (s, w) in self.scratch_in.iter_mut().zip(self.window.iter()) {
            *s *= *w;
        }
        let _ = self.fft.process(&mut self.scratch_in, &mut self.spectrum);
        let bin_hz = fs / FFT_SIZE as f32;
        // amplitude scaling: a bin-centred full-scale sine has magnitude 1 (Hann coherent gain 0.5)
        let norm = 2.0 / (FFT_SIZE as f32 * 0.5);
        let mags: Vec<f32> = self.spectrum.iter().map(|c| c.norm() * norm).collect();
        let tr = if pre { &mut self.pre } else { &mut self.post };
        for i in 0..self.points {
            let f = 10.0 * (3000.0f32).powf(i as f32 / (self.points - 1) as f32);
            // smoothing about 1/6 octave: average power over [f / 2^(1/12), f * 2^(1/12)]
            let (lo, hi) = (f / 1.0595, f * 1.0595);
            if lo >= fs / 2.0 {
                tr.db[i] = DB_FLOOR; // above Nyquist: nothing to show
                continue;
            }
            let b0 = ((lo / bin_hz).floor() as usize).min(mags.len() - 1);
            let b1 = ((hi / bin_hz).ceil() as usize).clamp(b0 + 1, mags.len());
            let p = if b1 - b0 <= 2 {
                // narrower than a couple of FFT bins (low frequencies): interpolate instead of showing steps
                let pos = (f / bin_hz).clamp(0.0, (mags.len() - 2) as f32);
                let (i0, fr) = (pos.floor() as usize, pos.fract());
                mags[i0] * mags[i0] * (1.0 - fr) + mags[i0 + 1] * mags[i0 + 1] * fr
            } else {
                mags[b0..b1].iter().map(|m| m * m).sum::<f32>() / (b1 - b0) as f32
            };
            let mut db = 10.0 * p.max(1e-12).log10();
            db += TILT_DB_PER_OCT * (f / 1000.0).log2();
            let cur = tr.db[i];
            tr.db[i] = if db > cur { db } else { (cur - decay_db).max(db) };
        }
    }
}

/// The most prominent spectral peak near `cursor_hz`: a local maximum within about a third of an octave that stands at
/// least `min_prominence_db` above the valleys around it. `freq_of(i)` gives the frequency of display point `i`.
pub fn find_peak(db: &[f32], freq_of: impl Fn(usize) -> f32, cursor_hz: f32, min_prominence_db: f32) -> Option<usize> {
    let n = db.len();
    if n < 5 || cursor_hz <= 0.0 {
        return None;
    }
    let (lo, hi) = (cursor_hz / 1.26, cursor_hz * 1.26);
    let mut best: Option<(usize, f32)> = None;
    for i in 1..n - 1 {
        let f = freq_of(i);
        if f < lo || f > hi || db[i] < db[i - 1] || db[i] < db[i + 1] || db[i] <= DB_FLOOR + 3.0 {
            continue;
        }
        // valleys: the lowest level before a higher point (or an octave away) on each side
        let valley = |step: isize| {
            let mut m = db[i];
            let mut j = i as isize + step;
            while j >= 0 && (j as usize) < n && db[j as usize] <= db[i] && (freq_of(j as usize) / f).log2().abs() <= 1.0 {
                m = m.min(db[j as usize]);
                j += step;
            }
            m
        };
        let prominence = db[i] - valley(-1).max(valley(1));
        if prominence >= min_prominence_db && best.map_or(true, |(_, v)| db[i] > v) {
            best = Some((i, db[i]));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sine_peaks_at_its_frequency() {
        let fs = 48_000.0;
        let mut ring = Ring::new();
        let x: Vec<f32> = (0..RING).map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / fs).sin()).collect();
        ring.push(&x, &x);
        let mut a = Analyzer::new(400);
        for _ in 0..3 {
            a.update(&ring, false, fs, 100.0);
        }
        let (idx, peak) = a.post.db.iter().enumerate().fold((0, f32::MIN), |m, (i, v)| if *v > m.1 { (i, *v) } else { m });
        let f = a.point_freq(idx);
        assert!((f - 1000.0).abs() / 1000.0 < 0.05, "peak at {f} Hz");
        // the 1/6-octave power average spreads a pure tone over ~20 bins, so a full-scale sine reads about -8 to -11 dB
        assert!(peak > -14.0 && peak < -6.0, "peak {peak} dB (1 kHz has no tilt)");
    }

    #[test]
    fn falling_levels_decay_gradually() {
        let fs = 48_000.0;
        let mut ring = Ring::new();
        let tone: Vec<f32> = (0..RING).map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / fs).sin()).collect();
        ring.push(&tone, &tone);
        let mut a = Analyzer::new(200);
        a.update(&ring, false, fs, 1.5);
        let loud = a.post.db.iter().cloned().fold(f32::MIN, f32::max);
        ring.push(&vec![0.0; RING], &vec![0.0; RING]);
        a.update(&ring, false, fs, 1.5);
        let after = a.post.db.iter().cloned().fold(f32::MIN, f32::max);
        assert!((loud - after - 1.5).abs() < 0.1, "{loud} -> {after}");
    }

    fn log_pts(n: usize) -> impl Fn(usize) -> f32 {
        move |i| 10.0 * 3000.0f32.powf(i as f32 / (n - 1) as f32)
    }

    #[test]
    fn find_peak_picks_the_prominent_peak_near_the_cursor() {
        let n = 480;
        let f = log_pts(n);
        let mut db = vec![-70.0f32; n];
        // a peak at ~1 kHz, 20 dB over the floor, and a smaller bump at ~3 kHz
        for i in 0..n {
            let x = (f(i) / 1000.0).ln();
            db[i] += 20.0 * (-x * x / 0.002).exp();
            let y = (f(i) / 3000.0).ln();
            db[i] += 8.0 * (-y * y / 0.002).exp();
        }
        let i = find_peak(&db, &f, 950.0, 6.0).expect("peak");
        assert!((f(i) - 1000.0).abs() < 25.0, "{}", f(i));
        assert!(find_peak(&db, &f, 300.0, 6.0).is_none(), "nothing near 300 Hz");
        assert!(find_peak(&db, &f, 2900.0, 6.0).is_some(), "the 8 dB bump counts at 6 dB");
        assert!(find_peak(&db, &f, 2900.0, 12.0).is_none(), "but not at 12 dB");
    }

    #[test]
    fn find_peak_ignores_a_flat_or_sloping_spectrum() {
        let n = 300;
        let f = log_pts(n);
        let flat = vec![-50.0f32; n];
        assert!(find_peak(&flat, &f, 1000.0, 6.0).is_none());
        let slope: Vec<f32> = (0..n).map(|i| -80.0 + i as f32 * 0.1).collect();
        assert!(find_peak(&slope, &f, 1000.0, 6.0).is_none());
    }

    #[test]
    fn ring_returns_the_latest_samples_in_order() {
        let mut r = Ring::new();
        let x: Vec<f32> = (0..(RING + 100)).map(|i| i as f32).collect();
        r.push(&x, &x);
        let mut out = vec![0.0; 4];
        r.latest(false, &mut out);
        assert_eq!(out, vec![(RING + 96) as f32, (RING + 97) as f32, (RING + 98) as f32, (RING + 99) as f32]);
    }
}

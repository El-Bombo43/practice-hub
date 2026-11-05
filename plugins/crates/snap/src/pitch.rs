//! Monophonic pitch detection: YIN (de Cheveigné & Kawahara, 2002) on a 4x-decimated signal for the octave decision,
//! then a full-rate refinement around the candidate lag for cents-level accuracy.
use phub_common::dsp::Biquad;

/// Power-of-two ring buffer with absolute (ever-increasing) indices.
pub struct Ring {
    buf: Vec<f32>,
    mask: usize,
    pub n: u64,
}
impl Ring {
    pub fn new(min_len: usize) -> Self {
        let len = min_len.next_power_of_two().max(64);
        Self { buf: vec![0.0; len], mask: len - 1, n: 0 }
    }
    #[inline]
    pub fn push(&mut self, x: f32) {
        self.buf[(self.n as usize) & self.mask] = x;
        self.n += 1;
    }
    /// Sample at absolute index `i` (zero before the start; the caller must stay within the ring length of `n`).
    #[inline]
    pub fn at(&self, i: i64) -> f32 {
        if i < 0 || i as u64 >= self.n {
            0.0
        } else {
            self.buf[(i as usize) & self.mask]
        }
    }
    pub fn clear(&mut self) {
        self.buf.iter_mut().for_each(|v| *v = 0.0);
        self.n = 0;
    }
    /// Copy `out.len()` samples starting at absolute index `start` (zeros outside what has been written).
    pub fn copy_range(&self, start: i64, out: &mut [f32]) {
        for (k, o) in out.iter_mut().enumerate() {
            *o = self.at(start + k as i64);
        }
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    /// Hz, 0 when no period was found
    pub f0: f32,
    /// YIN's aperiodicity at the chosen lag (0 = perfectly periodic, ~1 = noise)
    pub aperiodicity: f32,
    pub rms: f32,
}

pub struct Detector {
    fs: f32,
    d: usize,
    lp: [Biquad; 2],
    dbuf: Ring,
    phase: usize,
    tau_min: usize,
    tau_max: usize,
    w: usize,
    seg: Vec<f32>,
    cmnd: Vec<f32>,
    /// samples (full rate) between the newest sample and the centre of the analysed window
    pub lag_samples: usize,
    /// lag (decimated samples) of the last good estimate: continuity prior against octave jumps
    prev_tau: f32,
    miss: u32,
    scratch_seg: Vec<f32>,
    scratch_vals: Vec<f32>,
}

impl Detector {
    /// `fmin`..`fmax` Hz is the search range.
    pub fn new(fs: f32, fmin: f32, fmax: f32) -> Self {
        let d = ((fs / 12_000.0).round() as usize).max(1);
        let fsd = fs / d as f32;
        let tau_min = ((fsd / fmax).floor() as usize).max(2);
        let tau_max = (fsd / fmin).ceil() as usize + 1;
        let w = ((tau_max as f32) * 0.8) as usize;
        let mut lp = [Biquad::identity(); 2];
        for b in lp.iter_mut() {
            b.set_coeffs(&Biquad::lowpass(fs as f64, 3000.0, std::f64::consts::FRAC_1_SQRT_2));
        }
        let lag = ((w + tau_max) / 2 + w / 2) * d;
        Self { fs, d, lp, dbuf: Ring::new(2 * (w + tau_max) + 16), phase: 0, tau_min, tau_max, w, seg: vec![0.0; w + tau_max + 2], cmnd: vec![0.0; tau_max + 2], lag_samples: lag, prev_tau: 0.0, miss: 0, scratch_seg: Vec::with_capacity(1600 + (fs / fmin) as usize * 2 + 256), scratch_vals: Vec::with_capacity(64) }
    }

    pub fn reset(&mut self) {
        self.lp.iter_mut().for_each(|b| b.clear());
        self.dbuf.clear();
        self.phase = 0;
        self.prev_tau = 0.0;
        self.miss = 0;
    }

    /// Feed a full-rate sample.
    pub fn push(&mut self, x: f32) {
        let a = self.lp[0].process(x as f64);
        let y = self.lp[1].process(a) as f32;
        self.phase += 1;
        if self.phase >= self.d {
            self.phase = 0;
            self.dbuf.push(y);
        }
    }

    pub fn ready(&self) -> bool {
        self.dbuf.n as usize >= self.w + self.tau_max + 2
    }

    /// Analyse the newest data. `mono` is the full-rate signal.
    pub fn analyse(&mut self, mono: &Ring) -> Frame {
        let need = self.w + self.tau_max;
        if !self.ready() {
            return Frame::default();
        }
        let end = self.dbuf.n as i64;
        for (k, s) in self.seg.iter_mut().enumerate().take(need) {
            *s = self.dbuf.at(end - need as i64 + k as i64);
        }
        let rms = (self.seg[..need].iter().map(|v| v * v).sum::<f32>() / need as f32).sqrt();
        if rms < 1.0e-4 {
            return Frame { f0: 0.0, aperiodicity: 1.0, rms };
        }
        // difference function and its cumulative-mean normalisation
        let (w, tmax) = (self.w, self.tau_max);
        let mut run = 0.0f32;
        self.cmnd[0] = 1.0;
        for tau in 1..=tmax {
            let d: f32 = self.seg[..w].iter().zip(&self.seg[tau..tau + w]).map(|(a, b)| (a - b) * (a - b)).sum();
            run += d;
            self.cmnd[tau] = if run > 0.0 { d * tau as f32 / run } else { 1.0 };
        }
        // the deepest dip sets the bar; the first dip nearly that deep is the period (smaller lags that only match a
        // formant ring are shallower, and multiples of the period are not "first")
        let mut gm = (f32::MAX, self.tau_min);
        for t in self.tau_min..tmax {
            if self.cmnd[t] < gm.0 {
                gm = (self.cmnd[t], t);
            }
        }
        let thr = gm.0 * 1.5 + 0.03;
        let mut tau = gm.1;
        let mut t = self.tau_min;
        while t < gm.1 {
            if self.cmnd[t] <= thr && self.cmnd[t] <= self.cmnd[t.saturating_sub(1)] && self.cmnd[t] <= self.cmnd[t + 1] {
                tau = t;
                break;
            }
            t += 1;
        }
        // continuity: a dip near the previous period beats a deeper dip an octave away
        if self.prev_tau > 0.0 {
            let (lo, hi) = (((self.prev_tau * 0.75) as usize).max(self.tau_min), ((self.prev_tau * 1.33) as usize + 1).min(tmax - 1));
            let mut best = (f32::MAX, 0usize);
            for t in lo..=hi {
                if self.cmnd[t] < best.0 && self.cmnd[t] <= self.cmnd[t - 1] && self.cmnd[t] <= self.cmnd[t + 1] {
                    best = (self.cmnd[t], t);
                }
            }
            if best.1 > 0 && best.0 < 0.3 && best.0 <= gm.0 + 0.15 {
                tau = best.1;
                // never settle on a multiple of the period when the shorter dip is nearly as good
                for k in [3usize, 2] {
                    let c = (tau as f32 / k as f32).round() as usize;
                    let (a, b) = (((c as f32 * 0.94) as usize).max(self.tau_min), ((c as f32 * 1.06) as usize + 1).min(tmax - 1));
                    if c < self.tau_min || a + 1 >= b {
                        continue;
                    }
                    let mut sub = (f32::MAX, 0usize);
                    for t in a.max(1)..=b {
                        if self.cmnd[t] < sub.0 && self.cmnd[t] <= self.cmnd[t - 1] && self.cmnd[t] <= self.cmnd[t + 1] {
                            sub = (self.cmnd[t], t);
                        }
                    }
                    if sub.1 > 0 && sub.0 <= self.cmnd[tau] + 0.12 {
                        tau = sub.1;
                        break;
                    }
                }
            }
        }
        let ap = self.cmnd[tau];
        if ap < 0.25 {
            self.prev_tau = tau as f32;
            self.miss = 0;
        } else {
            self.miss += 1;
            if self.miss > 3 {
                self.prev_tau = 0.0;
            }
        }
        // parabolic interpolation on the decimated grid
        let frac = parabolic(self.cmnd[tau - 1], self.cmnd[tau], self.cmnd[tau + 1]);
        let tau0 = (tau as f32 + frac) * self.d as f32; // full-rate lag
        let f0 = self.refine(mono, tau0);
        Frame { f0, aperiodicity: ap, rms }
    }

    /// Full-rate refinement: minimise the difference function over lags near `tau0`.
    fn refine(&mut self, mono: &Ring, tau0: f32) -> f32 {
        let e = mono.n as i64 - 1;
        let wf = ((2.5 * tau0) as usize).clamp(64, 1600);
        let span = self.d as i64 + 2;
        let lo = ((tau0 as i64) - span).max(2);
        let hi = (tau0 as i64) + span;
        if e < wf as i64 + hi + 2 {
            return self.fs / tau0;
        }
        let len = wf + hi as usize + 1;
        let mut seg = std::mem::take(&mut self.scratch_seg);
        seg.clear();
        seg.resize(len, 0.0); // within the capacity reserved at construction: no allocation
        mono.copy_range(e - len as i64 + 1, &mut seg);
        let newest = &seg[len - wf..];
        let mut vals = std::mem::take(&mut self.scratch_vals);
        vals.clear();
        let mut best = (f32::MAX, lo);
        for tau in lo..=hi {
            let past = &seg[len - wf - tau as usize..len - tau as usize];
            let d: f32 = newest.iter().zip(past).map(|(a, b)| (a - b) * (a - b)).sum();
            vals.push(d);
            if d < best.0 {
                best = (d, tau);
            }
        }
        let k = (best.1 - lo) as usize;
        let frac = if k > 0 && k + 1 < vals.len() { parabolic(vals[k - 1], vals[k], vals[k + 1]) } else { 0.0 };
        let f0 = self.fs / (best.1 as f32 + frac);
        self.scratch_seg = seg;
        self.scratch_vals = vals;
        f0
    }
}

/// Offset of the minimum of a parabola through three equally spaced points (-1, 0, +1).
fn parabolic(a: f32, b: f32, c: f32) -> f32 {
    let den = a - 2.0 * b + c;
    if den.abs() < 1e-12 {
        0.0
    } else {
        (0.5 * (a - c) / den).clamp(-1.0, 1.0)
    }
}

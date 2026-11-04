//! Loudness and peak measurement after ITU-R BS.1770-4 / EBU R128 (K-weighting, gating) and EBU Tech 3342 (LRA).
//! Audio is only read, never changed.
use phub_common::dsp::Biquad;
use std::collections::VecDeque;
use std::f64::consts::PI;

pub const BAR_SECONDS: f32 = 0.02;
pub const MAX_BARS: usize = 60_000; // 20 minutes of 20 ms bars
const MAX_BLOCKS: usize = 400_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct Bar {
    /// true-peak envelope over the bar (linear)
    pub tp: f32,
    /// short-term loudness at the time (LUFS, -inf before 3 s)
    pub st: f32,
    pub over: bool,
}

#[derive(Clone, Debug)]
pub struct OverEvent {
    pub beats: Option<f64>,
    pub seconds: f64,
    pub peak_db: f32,
    /// index of the 20 ms bar (counted since the last reset) the event started in
    pub bar_index: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Transport {
    pub playing: bool,
    pub beats: Option<f64>,
    pub seconds: f64,
    pub tempo: f64,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub momentary: f32,
    pub short_term: f32,
    pub integrated: f32,
    pub lra: f32,
    pub tp_max: [f32; 2],
    pub sample_peak: f32,
    pub max_momentary: f32,
    pub max_short: f32,
    pub overs: u32,
    pub clips: u32,
    pub correlation: f32,
    pub tp_now: f32,
    pub over_list: Vec<OverEvent>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self { momentary: f32::NEG_INFINITY, short_term: f32::NEG_INFINITY, integrated: f32::NEG_INFINITY, lra: 0.0, tp_max: [f32::NEG_INFINITY; 2], sample_peak: f32::NEG_INFINITY, max_momentary: f32::NEG_INFINITY, max_short: f32::NEG_INFINITY, overs: 0, clips: 0, correlation: 0.0, tp_now: f32::NEG_INFINITY, over_list: Vec::new() }
    }
}

impl Snapshot {
    /// PLR: highest true peak minus integrated loudness.
    pub fn plr(&self) -> f32 {
        self.tp_max[0].max(self.tp_max[1]) - self.integrated
    }
}

fn lufs(power: f64) -> f32 {
    if power <= 0.0 {
        f32::NEG_INFINITY
    } else {
        (-0.691 + 10.0 * power.log10()) as f32
    }
}
fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

/// K-weighting: the BS.1770 high shelf then the RLB high-pass, for any sample rate.
fn k_weighting(fs: f64) -> [Biquad; 2] {
    let (f0, g, q) = (1681.974450955533, 3.999843853973347, 0.7071752369554196);
    let k = (PI * f0 / fs).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let a0 = 1.0 + k / q + k * k;
    let mut shelf = Biquad::identity();
    shelf.b0 = (vh + vb * k / q + k * k) / a0;
    shelf.b1 = 2.0 * (k * k - vh) / a0;
    shelf.b2 = (vh - vb * k / q + k * k) / a0;
    shelf.a1 = 2.0 * (k * k - 1.0) / a0;
    shelf.a2 = (1.0 - k / q + k * k) / a0;
    let (f0, q) = (38.13547087613982, 0.5003270373253953);
    let k = (PI * f0 / fs).tan();
    let a0 = 1.0 + k / q + k * k;
    let mut rlb = Biquad::identity();
    rlb.b0 = 1.0;
    rlb.b1 = -2.0;
    rlb.b2 = 1.0;
    rlb.a1 = 2.0 * (k * k - 1.0) / a0;
    rlb.a2 = (1.0 - k / q + k * k) / a0;
    [shelf, rlb]
}

const TP_TAPS: usize = 48;

/// 4x polyphase interpolator (Kaiser-windowed sinc, 12 taps per phase) for true-peak detection.
struct Oversampler {
    phases: [[f32; TP_TAPS / 4]; 4],
    hist: [f32; TP_TAPS / 4],
}
impl Oversampler {
    fn new() -> Self {
        let n = TP_TAPS;
        let beta = 8.0f64;
        let i0 = |x: f64| { let (mut s, mut t) = (1.0, 1.0); for k in 1..30 { t *= (x / (2.0 * k as f64)).powi(2); s += t; } s };
        let mut h = vec![0.0f64; n];
        for (i, v) in h.iter_mut().enumerate() {
            let x = (i as f64 - (n as f64 - 1.0) / 2.0) / 4.0;
            let sinc = if x == 0.0 { 1.0 } else { (PI * x).sin() / (PI * x) };
            let r = 2.0 * i as f64 / (n as f64 - 1.0) - 1.0;
            *v = sinc * i0(beta * (1.0 - r * r).max(0.0).sqrt()) / i0(beta);
        }
        let mut phases = [[0.0f32; TP_TAPS / 4]; 4];
        for p in 0..4 {
            let sum: f64 = (0..n / 4).map(|k| h[p + 4 * k]).sum();
            for k in 0..n / 4 {
                phases[p][k] = (h[p + 4 * k] / sum) as f32; // unity DC gain per phase
            }
        }
        Self { phases, hist: [0.0; TP_TAPS / 4] }
    }
    /// Push a sample, return the largest |value| among the 4 interpolated outputs.
    #[inline]
    fn push(&mut self, x: f32) -> f32 {
        self.hist.copy_within(0..TP_TAPS / 4 - 1, 1);
        self.hist[0] = x;
        let mut best = 0.0f32;
        for p in 0..4 {
            let v: f32 = self.phases[p].iter().zip(&self.hist).map(|(a, b)| a * b).sum();
            best = best.max(v.abs());
        }
        best.max(x.abs())
    }
}

pub struct Meter {
    fs: f64,
    nch: usize,
    kw: [[Biquad; 2]; 2],
    os: [Oversampler; 2],
    hop_len: usize,
    hop_fill: usize,
    hop_pow: f64,
    subs: VecDeque<f64>, // mean power of the last <=30 sub-blocks (100 ms each), channels summed
    blocks: Vec<f64>,    // 400 ms block powers for the integrated gate
    short_terms: Vec<f64>, // 3 s window powers for LRA
    bar_len: usize,
    bar_fill: usize,
    bar_tp: f32,
    bar_over: bool,
    st_now: f32,
    pub bars: Vec<Bar>,
    pub dropped_bars: usize,
    pub bars_total: u64,
    pub reset_pending: bool,
    snap: Snapshot,
    // events
    over_open: Option<(OverEvent, usize)>, // event, samples since last over
    clip_quiet: Option<usize>,
    merge: usize,
    corr: [f64; 3],
    corr_k: f64,
    samples_in_session: u64,
}

impl Meter {
    pub fn new(fs: f64) -> Self {
        let mut m = Self {
            fs,
            nch: 2,
            kw: [k_weighting(fs), k_weighting(fs)],
            os: [Oversampler::new(), Oversampler::new()],
            hop_len: (fs * 0.1).round() as usize,
            hop_fill: 0,
            hop_pow: 0.0,
            subs: VecDeque::new(),
            blocks: Vec::new(),
            short_terms: Vec::new(),
            bar_len: (fs * BAR_SECONDS as f64).round() as usize,
            bar_fill: 0,
            bar_tp: 0.0,
            bar_over: false,
            st_now: f32::NEG_INFINITY,
            bars: Vec::new(),
            dropped_bars: 0,
            bars_total: 0,
            reset_pending: false,
            snap: Snapshot::default(),
            over_open: None,
            clip_quiet: None,
            merge: (fs * 0.05) as usize,
            corr: [0.0; 3],
            corr_k: (-1.0 / fs).exp(),
            samples_in_session: 0,
        };
        m.reset();
        m
    }

    pub fn reset(&mut self) {
        self.kw = [k_weighting(self.fs), k_weighting(self.fs)];
        self.os = [Oversampler::new(), Oversampler::new()];
        self.hop_fill = 0;
        self.hop_pow = 0.0;
        self.subs.clear();
        self.blocks.clear();
        self.short_terms.clear();
        self.bar_fill = 0;
        self.bar_tp = 0.0;
        self.bar_over = false;
        self.st_now = f32::NEG_INFINITY;
        self.bars.clear();
        self.dropped_bars = 0;
        self.bars_total = 0;
        self.reset_pending = true;
        self.snap = Snapshot::default();
        self.over_open = None;
        self.clip_quiet = None;
        self.corr = [0.0; 3];
        self.samples_in_session = 0;
    }

    pub fn sample_rate(&self) -> f64 {
        self.fs
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snap
    }

    fn integrated(&self) -> f32 {
        let above: Vec<f64> = self.blocks.iter().copied().filter(|p| lufs(*p) > -70.0).collect();
        if above.is_empty() {
            return f32::NEG_INFINITY;
        }
        let rel = lufs(above.iter().sum::<f64>() / above.len() as f64) - 10.0;
        let kept: Vec<f64> = above.into_iter().filter(|p| lufs(*p) > rel).collect();
        if kept.is_empty() {
            return f32::NEG_INFINITY;
        }
        lufs(kept.iter().sum::<f64>() / kept.len() as f64)
    }

    /// EBU Tech 3342: 3 s loudness values, absolute gate -70, relative gate -20 LU, 10th to 95th percentile.
    fn lra(&self) -> f32 {
        let above: Vec<f64> = self.short_terms.iter().copied().filter(|p| lufs(*p) > -70.0).collect();
        if above.is_empty() {
            return 0.0;
        }
        let rel = lufs(above.iter().sum::<f64>() / above.len() as f64) - 20.0;
        let mut l: Vec<f32> = above.iter().map(|p| lufs(*p)).filter(|v| *v > rel).collect();
        if l.len() < 2 {
            return 0.0;
        }
        l.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let at = |q: f32| l[((l.len() - 1) as f32 * q).round() as usize];
        at(0.95) - at(0.10)
    }

    fn finish_hop(&mut self) {
        let p = self.hop_pow / self.hop_len as f64;
        self.hop_pow = 0.0;
        self.subs.push_back(p);
        if self.subs.len() > 30 {
            self.subs.pop_front();
        }
        let n = self.subs.len();
        if n >= 4 {
            let m: f64 = self.subs.iter().rev().take(4).sum::<f64>() / 4.0;
            self.snap.momentary = lufs(m);
            self.snap.max_momentary = self.snap.max_momentary.max(self.snap.momentary);
            if self.blocks.len() < MAX_BLOCKS {
                self.blocks.push(m);
            }
        }
        if n >= 30 {
            let s: f64 = self.subs.iter().sum::<f64>() / 30.0;
            self.snap.short_term = lufs(s);
            self.st_now = self.snap.short_term;
            self.snap.max_short = self.snap.max_short.max(self.snap.short_term);
            if self.short_terms.len() < MAX_BLOCKS {
                self.short_terms.push(s);
            }
            if self.short_terms.len() % 10 == 0 {
                self.snap.lra = self.lra();
            }
        }
        self.snap.integrated = self.integrated();
    }

    /// Measure a block. `ceiling_db` is the target true-peak ceiling used for the overs count.
    pub fn process(&mut self, l: &[f32], r: Option<&[f32]>, ceiling_db: f32, t: &Transport) {
        self.nch = if r.is_some() { 2 } else { 1 };
        let ceiling = 10f32.powf(ceiling_db / 20.0);
        for i in 0..l.len() {
            let x = [l[i], r.map_or(0.0, |r| r[i])];
            let mut tp_here = 0.0f32;
            for ch in 0..self.nch {
                let v = x[ch];
                let a = self.kw[ch][0].process(v as f64);
                let k = self.kw[ch][1].process(a);
                self.hop_pow += k * k;
                let tp = self.os[ch].push(v);
                self.snap.tp_max[ch] = self.snap.tp_max[ch].max(db(tp));
                self.snap.sample_peak = self.snap.sample_peak.max(db(v.abs()));
                tp_here = tp_here.max(tp);
            }
            self.snap.tp_now = db(tp_here);
            if self.nch == 2 {
                let (a, b) = (x[0] as f64, x[1] as f64);
                self.corr[0] = self.corr[0] * self.corr_k + a * b;
                self.corr[1] = self.corr[1] * self.corr_k + a * a;
                self.corr[2] = self.corr[2] * self.corr_k + b * b;
            }
            // overs: consecutive samples above the ceiling, merged when within 50 ms
            let over = tp_here > ceiling;
            let pos = |off: usize| -> (Option<f64>, f64) {
                let secs = off as f64 / self.fs;
                (t.beats.map(|b| b + secs * t.tempo / 60.0), t.seconds + secs)
            };
            if over {
                match self.over_open.as_mut() {
                    Some((ev, quiet)) => {
                        *quiet = 0;
                        if db(tp_here) > ev.peak_db {
                            ev.peak_db = db(tp_here);
                        }
                    }
                    None => {
                        let (b, s) = pos(i);
                        self.over_open = Some((OverEvent { beats: b, seconds: s, peak_db: db(tp_here), bar_index: self.bars_total }, 0));
                        self.snap.overs += 1;
                        self.snap.over_list.push(OverEvent { beats: b, seconds: s, peak_db: db(tp_here), bar_index: self.bars_total });
                    }
                }
                self.bar_over = true;
            } else if let Some((ev, quiet)) = self.over_open.as_mut() {
                *quiet += 1;
                if *quiet > self.merge {
                    let _ = ev;
                    self.over_open = None;
                }
            }
            // keep the list entry's peak current while the event is open
            if let (Some((ev, _)), true) = (self.over_open.as_ref(), over) {
                if let Some(last) = self.snap.over_list.last_mut() {
                    last.peak_db = ev.peak_db;
                }
            }
            let clip = x[0].abs() >= 1.0 || x[1].abs() >= 1.0;
            if clip {
                if self.clip_quiet.is_none() {
                    self.snap.clips += 1;
                }
                self.clip_quiet = Some(0);
            } else if let Some(q) = self.clip_quiet.as_mut() {
                *q += 1;
                if *q > self.merge {
                    self.clip_quiet = None;
                }
            }
            self.bar_tp = self.bar_tp.max(tp_here);
            self.bar_fill += 1;
            if self.bar_fill >= self.bar_len {
                self.bars.push(Bar { tp: self.bar_tp, st: self.st_now, over: self.bar_over });
                self.bars_total += 1;
                self.bar_fill = 0;
                self.bar_tp = 0.0;
                self.bar_over = false;
                if self.bars.len() > 6000 {
                    self.bars.remove(0); // the editor is not draining (closed): keep only the latest
                    self.dropped_bars += 1;
                }
            }
            self.hop_fill += 1;
            if self.hop_fill >= self.hop_len {
                self.hop_fill = 0;
                self.finish_hop();
            }
        }
        let c = (self.corr[1] * self.corr[2]).sqrt();
        self.snap.correlation = if c > 1e-12 { (self.corr[0] / c) as f32 } else { 0.0 };
        self.samples_in_session += l.len() as u64;
    }
}

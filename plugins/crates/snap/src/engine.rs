//! Snap's engine: pitch detection, note decision, the correction curve, and a time-domain PSOLA shifter that works on a
//! delayed copy of the signal (the reported latency is the lookahead).
use crate::pitch::{Detector, Ring};
use crate::scale::{decide, hz_to_midi};
use phub_common::dsp::{db_to_gain, Biquad, Ramp};
use std::collections::VecDeque;

pub const TRACK_LATENCY_MS: f32 = 10.0;
pub const MIX_LATENCY_MS: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub track: bool,
    /// 0 High, 1 Mid, 2 Low
    pub voice: usize,
    pub notes: [bool; 12],
    pub retune_ms: f32,
    /// 0..1
    pub humanize: f32,
    /// 0..1
    pub flex: f32,
    /// 0..1
    pub amount: f32,
    /// 0..1
    pub mix: f32,
    pub out_gain_db: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let mut notes = [false; 12];
        for n in [0, 2, 4, 5, 7, 9, 11] {
            notes[n] = true;
        }
        Self { track: false, voice: 1, notes, retune_ms: 50.0, humanize: 0.2, flex: 0.5, amount: 1.0, mix: 1.0, out_gain_db: 0.0 }
    }
}

pub fn voice_range(voice: usize) -> (f32, f32) {
    match voice {
        0 => (150.0, 1000.0),
        2 => (55.0, 450.0),
        _ => (80.0, 800.0),
    }
}

/// What the editor shows (latest analysis hop).
#[derive(Clone, Copy, Debug)]
pub struct Display {
    /// MIDI note number of the detected pitch, NaN when unvoiced
    pub in_midi: f32,
    pub out_midi: f32,
    pub target: i32,
    pub voiced: bool,
}

#[derive(Clone, Copy, Debug)]
struct Ctl {
    ts: i64,
    voiced: bool,
    f0: f32,
    shift_st: f32,
}

fn smooth01(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

struct Corrector {
    target: Option<i32>,
    held: f32,
    was_voiced: bool,
    lp: Biquad,
    c_slow: f32,
}

impl Corrector {
    fn new(fs_hop: f64) -> Self {
        Self { target: None, held: 0.0, was_voiced: false, lp: Biquad::lowpass(fs_hop, 1.5, std::f64::consts::FRAC_1_SQRT_2), c_slow: 0.0 }
    }

    /// One analysis hop. Returns (applied shift in semitones, target note).
    fn step(&mut self, m: f32, s: &Settings, dt: f32) -> (f32, Option<i32>) {
        if !self.was_voiced {
            self.was_voiced = true;
            self.lp.init_dc(m as f64);
            self.c_slow = 0.0;
            self.target = None;
            self.held = 0.0;
        }
        let mut mc = self.lp.process(m as f64) as f32;
        if (m - mc).abs() > 0.8 {
            self.lp.init_dc(m as f64); // a new note, not vibrato: re-centre
            mc = m;
        }
        let v = m - mc;
        let new_t = decide(m, self.target, &s.notes, 0.15);
        if new_t != self.target {
            self.held = 0.0;
        } else {
            self.held += dt;
        }
        self.target = new_t;
        let Some(t) = new_t else {
            self.c_slow = 0.0;
            return (0.0, None);
        };
        let e_slow = t as f32 - mc;
        // Humanize: after 150 ms on a note the retune time stretches by up to 4x
        let r_eff = s.retune_ms * (1.0 + 3.0 * s.humanize * smooth01((self.held - 0.15) / 0.1));
        if r_eff <= 0.5 {
            self.c_slow = e_slow;
        } else {
            let tau = r_eff * 0.001 / std::f32::consts::LN_10; // 90 % of the way after r_eff
            self.c_slow += (e_slow - self.c_slow) * (1.0 - (-dt / tau).exp());
        }
        let c = (self.c_slow - (1.0 - s.flex) * v) * s.amount;
        (c.clamp(-6.0, 6.0), Some(t))
    }

    fn unvoiced(&mut self) {
        self.was_voiced = false;
    }
}

pub struct Engine {
    fs: f32,
    s: Settings,
    initialised: bool,
    lat: usize,
    hop: usize,
    det: Detector,
    mono: Ring,
    rings: [Ring; 2],
    corr: Corrector,
    // voicing state machine
    v_on: u32,
    v_off: u32,
    voiced: bool,
    recent: [f32; 3],
    n_recent: usize,
    ctl: VecDeque<Ctl>,
    cur: Ctl,
    marks: Vec<f64>,
    // synthesis
    acc: [Vec<f32>; 2],
    acc_mask: usize,
    s_next: f64,
    chain_active: bool,
    chain_start: i64,
    vg: Ramp,
    ws: Ramp,
    mix: Ramp,
    out_g: Ramp,
    pub display: Display,
    /// test hook: run the shifter even when the shift is zero
    pub force_psola: bool,
    peak_in: f32,
    peak_out: f32,
}

impl Engine {
    pub fn new(fs: f64, s: &Settings) -> Self {
        let fs = fs as f32;
        let (fmin, fmax) = voice_range(s.voice);
        let det = Detector::new(fs, fmin, fmax);
        let d = ((fs / 12_000.0).round() as usize).max(1);
        let hop = d * 16;
        let lat = (fs * 0.001 * if s.track { TRACK_LATENCY_MS } else { MIX_LATENCY_MS }).round() as usize;
        let ring_len = lat + (fs / 55.0) as usize * 10 + 4096;
        let acc_len = ((fs / 55.0) as usize * 6).next_power_of_two();
        let mut e = Self {
            fs,
            s: *s,
            initialised: false,
            lat,
            hop,
            det,
            mono: Ring::new(ring_len),
            rings: [Ring::new(ring_len), Ring::new(ring_len)],
            corr: Corrector::new(fs as f64 / hop as f64),
            v_on: 0,
            v_off: 0,
            voiced: false,
            recent: [0.0; 3],
            n_recent: 0,
            ctl: VecDeque::new(),
            cur: Ctl { ts: 0, voiced: false, f0: 0.0, shift_st: 0.0 },
            marks: Vec::new(),
            acc: [vec![0.0; acc_len], vec![0.0; acc_len]],
            acc_mask: acc_len - 1,
            s_next: 0.0,
            chain_active: false,
            chain_start: 0,
            vg: Ramp::new(0.0),
            ws: Ramp::new(0.0),
            mix: Ramp::new(s.mix),
            out_g: Ramp::new(db_to_gain(s.out_gain_db)),
            display: Display { in_midi: f32::NAN, out_midi: f32::NAN, target: -1, voiced: false },
            force_psola: false,
            peak_in: 0.0,
            peak_out: 0.0,
        };
        e.initialised = true;
        e
    }

    pub fn latency(&self) -> usize {
        self.lat
    }
    pub fn sample_rate(&self) -> f32 {
        self.fs
    }
    pub fn settings(&self) -> Settings {
        self.s
    }

    /// Mode and Voice change the latency and the detector, so they rebuild the engine (a brief dropout is fine).
    pub fn set_settings(&mut self, s: &Settings) {
        if s.track != self.s.track || s.voice != self.s.voice {
            *self = Engine::new(self.fs as f64, s);
            return;
        }
        let n = 0.02 * self.fs;
        self.mix.set(s.mix, n);
        self.out_g.set(db_to_gain(s.out_gain_db), n);
        self.s = *s;
    }

    pub fn take_peaks(&mut self) -> (f32, f32) {
        let r = (self.peak_in, self.peak_out);
        self.peak_in = 0.0;
        self.peak_out = 0.0;
        r
    }

    fn analyse_hop(&mut self) {
        let newest = self.mono.n as i64 - 1;
        if !self.det.ready() {
            return;
        }
        let frame = self.det.analyse(&self.mono);
        let (fmin, fmax) = voice_range(self.s.voice);
        let ok = frame.f0 >= fmin * 0.95 && frame.f0 <= fmax * 1.05 && frame.aperiodicity < 0.2;
        if ok {
            self.v_on += 1;
            self.v_off = 0;
        } else {
            self.v_off += 1;
            self.v_on = 0;
        }
        if !self.voiced && self.v_on >= 3 {
            self.voiced = true;
            self.n_recent = 0;
        }
        if self.voiced && self.v_off >= 3 {
            self.voiced = false;
            self.corr.unvoiced();
            self.marks.clear();
        }
        let dt = self.hop as f32 / self.fs;
        let ts = (newest - self.det.lag_samples as i64).max(0);
        if self.voiced && ok {
            let m = hz_to_midi(frame.f0);
            self.recent = [self.recent[1], self.recent[2], m];
            self.n_recent = (self.n_recent + 1).min(3);
            let med = if self.n_recent >= 3 {
                let mut a = self.recent;
                a.sort_by(|x, y| x.partial_cmp(y).unwrap());
                a[1]
            } else {
                m
            };
            let (shift, target) = self.corr.step(med, &self.s, dt);
            let f0 = crate::scale::midi_to_hz(med);
            self.display = Display { in_midi: med, out_midi: med + shift, target: target.unwrap_or(-1), voiced: true };
            self.ctl.push_back(Ctl { ts, voiced: true, f0, shift_st: shift });
            self.extend_marks(f0);
        } else {
            if !self.voiced {
                self.display = Display { in_midi: f32::NAN, out_midi: f32::NAN, target: -1, voiced: false };
            }
            self.ctl.push_back(Ctl { ts, voiced: false, f0: 0.0, shift_st: 0.0 });
        }
        // forget marks the ring no longer holds
        let keep_from = newest - (self.mono.len() as i64 - 2048);
        self.marks.retain(|m| *m > keep_from as f64);
    }

    /// Pitch marks: positions one period apart, each placed (to a fraction of a sample) where the waveform best
    /// matches the previous cycle.
    fn extend_marks(&mut self, f0: f32) {
        let t = self.fs / f0;
        let newest = self.mono.n as i64 - 1;
        let ti = t.round() as i64;
        if self.marks.is_empty() {
            let (a, b) = (newest - (1.6 * t) as i64, newest - (0.6 * t) as i64);
            let mut best = (0.0f32, b);
            for i in a..=b {
                let v = self.mono.at(i).abs();
                if v > best.0 {
                    best = (v, i);
                }
            }
            self.marks.push(best.1 as f64);
        }
        while let Some(&last) = self.marks.last() {
            if last + 1.3 * t as f64 > newest as f64 {
                break;
            }
            let r = last.round() as i64;
            let cand = r + ti;
            let half = (ti / 2).max(2);
            let reach = (0.3 * t) as i64;
            let mut refseg = vec![0.0f32; 2 * half as usize + 1];
            self.mono.copy_range(r - half, &mut refseg);
            let e1 = refseg.iter().map(|v| v * v).sum::<f32>() + 1e-9;
            let region_len = 2 * reach as usize + 2 * half as usize + 1;
            let mut region = vec![0.0f32; region_len];
            self.mono.copy_range(cand - reach - half, &mut region);
            let mut scores: Vec<f32> = Vec::with_capacity(2 * reach as usize + 1);
            for sh in 0..=2 * reach as usize {
                let win = &region[sh..sh + refseg.len()];
                let dot: f32 = refseg.iter().zip(win).map(|(a, b)| a * b).sum();
                let e2: f32 = win.iter().map(|v| v * v).sum::<f32>() + 1e-9;
                scores.push(dot / (e1 * e2).sqrt());
            }
            let (bi, _) = scores.iter().enumerate().fold((0usize, f32::MIN), |m, (i, v)| if *v > m.1 { (i, *v) } else { m });
            let frac = if bi > 0 && bi + 1 < scores.len() {
                let (a, b, c) = (scores[bi - 1], scores[bi], scores[bi + 1]);
                let den = a - 2.0 * b + c;
                if den.abs() > 1e-12 { (0.5 * (a - c) / den).clamp(-0.5, 0.5) as f64 } else { 0.0 }
            } else {
                0.0
            };
            let c = (cand + bi as i64 - reach) as f64 + frac + (last - r as f64);
            if c <= last + 1.0 {
                break;
            }
            self.marks.push(c);
        }
    }

    fn place_grain(&mut self, s: f64, t_in: f32, t_out: f32, nch: usize) {
        let newest = self.mono.n as i64 - 1;
        // flat-topped grain of one output period: the cross-fades sit half a period from the mark, where the voice has
        // least energy, so shifted cycles are not blurred into each other
        let c = 0.35 * t_out;
        let a = 0.5 * (t_out - c);
        let h = ((a + c).ceil() as i64).max(8);
        // nearest mark whose whole grain is already available
        let mut best: Option<(f64, f64)> = None;
        for &m in self.marks.iter().rev() {
            if m + h as f64 + 1.0 > newest as f64 {
                continue;
            }
            let d = (m - s).abs();
            if best.map_or(true, |(bd, _)| d < bd) {
                best = Some((d, m));
            }
            if m < s - 2.0 * t_in as f64 {
                break;
            }
        }
        let Some((_, m)) = best else { return };
        // sub-sample placement: the source is read at (mark - synthesis fraction) with linear interpolation, so the
        // output period is exact on average and each grain keeps its pulse where it belongs
        let cs = s.floor() as i64;
        let mi = m.floor() as i64;
        let d = (m - mi as f64 - (s - cs as f64)) as f32; // -1..1
        for k in -h..=h {
            let ak = (k as f32).abs();
            let w = if ak <= a { 1.0 } else if ak < a + c { 0.5 * (1.0 + (std::f32::consts::PI * (ak - a) / c).cos()) } else { 0.0 };
            let pos = ((cs + k) as usize) & self.acc_mask;
            for ch in 0..nch {
                let r = &self.rings[ch];
                let v = if d >= 0.0 { (1.0 - d) * r.at(mi + k) + d * r.at(mi + k + 1) } else { (1.0 + d) * r.at(mi + k) - d * r.at(mi + k - 1) };
                self.acc[ch][pos] += v * w;
            }
        }
    }

    pub fn process(&mut self, l: &mut [f32], mut r: Option<&mut [f32]>) {
        let nch = if r.is_some() { 2 } else { 1 };
        let a5 = 0.005 * self.fs;
        let n = l.len();
        let identity_bypass = false;
        let _ = identity_bypass;
        for i in 0..n {
            let x = [l[i], r.as_ref().map_or(0.0, |r| r[i])];
            self.peak_in = self.peak_in.max(x[0].abs()).max(x[1].abs());
            self.rings[0].push(x[0]);
            self.rings[1].push(x[1]);
            let mono = if nch == 2 { 0.5 * (x[0] + x[1]) } else { x[0] };
            self.mono.push(mono);
            self.det.push(mono);
            let newest = self.mono.n as i64 - 1;
            if (self.mono.n as usize) % self.hop == 0 {
                self.analyse_hop();
            }
            let o = newest - self.lat as i64;
            // control state as of output time
            while let Some(c) = self.ctl.front() {
                if c.ts <= o {
                    self.cur = *c;
                    self.ctl.pop_front();
                } else {
                    break;
                }
            }
            let mut psola = [0.0f32; 2];
            let mut vg_target = 0.0;
            let mut ws_target = 0.0;
            if o >= 0 {
                let c = self.cur;
                if c.voiced && c.f0 > 0.0 {
                    let t_in = self.fs / c.f0;
                    let ratio = 2f32.powf(c.shift_st / 12.0);
                    let t_out = t_in / ratio;
                    if !self.chain_active {
                        self.chain_active = true;
                        self.s_next = o as f64;
                        self.chain_start = o;
                    }
                    while self.s_next - t_out as f64 <= o as f64 {
                        let s = self.s_next;
                        self.place_grain(s, t_in, t_out, nch);
                        self.s_next += t_out as f64;
                    }
                    if o >= self.chain_start + t_out as i64 {
                        vg_target = 1.0;
                    }
                    ws_target = if self.force_psola { 1.0 } else { smooth01((c.shift_st.abs() * 100.0 - 1.0) / 2.0) };
                } else {
                    self.chain_active = false;
                }
                let pos = (o as usize) & self.acc_mask;
                for ch in 0..nch {
                    psola[ch] = self.acc[ch][pos];
                    self.acc[ch][pos] = 0.0;
                }
            }
            self.vg.set(vg_target, a5);
            self.ws.set(ws_target, a5);
            let a = self.vg.tick() * self.ws.tick();
            let mix = self.mix.tick();
            let g = self.out_g.tick();
            let mut out = [0.0f32; 2];
            for ch in 0..nch {
                let dry = self.rings[ch].at(o);
                let wet = if a == 0.0 { dry } else { dry + (psola[ch] - dry) * a };
                let m = if mix >= 1.0 { wet } else { dry + (wet - dry) * mix };
                out[ch] = if g == 1.0 { m } else { m * g };
            }
            self.peak_out = self.peak_out.max(out[0].abs()).max(out[1].abs());
            l[i] = out[0];
            if let Some(r) = r.as_mut() {
                r[i] = out[1];
            }
        }
    }
}

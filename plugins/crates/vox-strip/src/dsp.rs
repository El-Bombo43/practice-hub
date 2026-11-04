//! Vox Strip's signal chain: input gain, gate, high-pass, de-esser, compressor, saturation (2x oversampled), output gain,
//! whole-strip mix. Zero latency: nothing looks ahead. Stereo channels share one detector per module.
use phub_common::dsp::{db_to_gain, tau_coef, Biquad, Ramp};

const BLOCKS: usize = 5; // modules with an On switch: gate, hpf, de-esser, compressor, saturation

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub in_gain_db: f32,
    pub on: [bool; BLOCKS],
    pub gate_thresh: f32,
    pub gate_range: f32,
    pub gate_attack_ms: f32,
    pub gate_release_ms: f32,
    pub hpf_freq: f32,
    pub hpf_24: bool,
    pub ds_freq: f32,
    pub ds_thresh: f32,
    pub ds_range: f32,
    pub ds_wide: bool,
    pub ds_listen: bool,
    pub comp_opto: bool,
    pub comp_thresh: f32,
    pub comp_ratio: f32,
    pub comp_attack_ms: f32,
    pub comp_release_ms: f32,
    pub comp_knee: f32,
    pub comp_makeup: f32,
    pub comp_auto: bool,
    pub sat_tube: bool,
    /// 0..1
    pub sat_drive: f32,
    /// 0..1
    pub sat_mix: f32,
    pub out_gain_db: f32,
    /// 0..1
    pub mix: f32,
}

pub const GATE: usize = 0;
pub const HPF: usize = 1;
pub const DESS: usize = 2;
pub const COMP: usize = 3;
pub const SAT: usize = 4;

impl Default for Settings {
    fn default() -> Self {
        Self {
            in_gain_db: 0.0,
            on: [false, true, true, true, false],
            gate_thresh: -50.0,
            gate_range: 20.0,
            gate_attack_ms: 1.0,
            gate_release_ms: 120.0,
            hpf_freq: 80.0,
            hpf_24: false,
            ds_freq: 6500.0,
            ds_thresh: -30.0,
            ds_range: 6.0,
            ds_wide: false,
            ds_listen: false,
            comp_opto: false,
            comp_thresh: -18.0,
            comp_ratio: 3.0,
            comp_attack_ms: 10.0,
            comp_release_ms: 120.0,
            comp_knee: 6.0,
            comp_makeup: 0.0,
            comp_auto: false,
            sat_tube: false,
            sat_drive: 0.2,
            sat_mix: 1.0,
            out_gain_db: 0.0,
            mix: 1.0,
        }
    }
}

/// Peaks and states for the meters, collected since the last `take_meters`.
#[derive(Clone, Copy, Default, Debug)]
pub struct Meters {
    pub in_peak: f32,
    pub out_peak: f32,
    pub gate_open: bool,
    pub ds_gr: f32,
    pub comp_gr: f32,
}

// Two-path polyphase allpass half-band filter (designed offline: 72 dB stop-band from 0.583 of the sampling rate,
// flat to 0.417). Used for the saturator's 2x oversampling; no latency is reported for it.
const HB0: [f64; 4] = [0.069600654026, 0.408999188145, 0.365047445805, 0.787254500312];
const HB1: [f64; 4] = [0.26481427179, 0.627239340884, 0.310199671489, 0.927207244207];

#[derive(Clone, Copy, Default)]
struct Ap2 {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}
impl Ap2 {
    #[inline]
    fn run(&mut self, a: f64, x: f64) -> f64 {
        let y = a * x + self.x2 - a * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[derive(Clone, Copy, Default)]
struct Path {
    s: [Ap2; 4],
}
impl Path {
    #[inline]
    fn run(&mut self, coef: &[f64; 4], x: f64) -> f64 {
        let mut v = x;
        for k in 0..4 {
            v = self.s[k].run(coef[k], v);
        }
        v
    }
}

/// 2x oversampler for one channel: `process` takes a base-rate sample, applies `f` at twice the rate, returns base rate.
#[derive(Clone, Copy, Default)]
struct Oversampler {
    up0: Path,
    up1: Path,
    dn0: Path,
    dn1: Path,
    prev_odd: f64,
}
impl Oversampler {
    #[inline]
    fn process(&mut self, x: f64, mut f: impl FnMut(f64) -> f64) -> f64 {
        let e = f(self.up0.run(&HB0, x));
        let o = f(self.up1.run(&HB1, x));
        let y = 0.5 * (self.dn0.run(&HB0, e) + self.dn1.run(&HB1, self.prev_odd));
        self.prev_odd = o;
        y
    }
}

pub struct Engine {
    fs: f32,
    s: Settings,
    initialised: bool,
    in_g: Ramp,
    out_g: Ramp,
    mix: Ramp,
    amt: [Ramp; BLOCKS],
    // gate
    gate_env: f32,
    gate_open: bool,
    gate_hold: u32,
    gate_gain: f32,
    // hpf
    hp: [[Biquad; 2]; 2],
    // de-esser
    bp: [Biquad; 2],
    ds_env: f32,
    ds_gr: f32,
    // compressor
    comp_env: f32,
    comp_ms: f32,
    comp_gr: f32,
    comp_mem: f32,
    // saturation
    os: [Oversampler; 2],
    tape_lp: [f64; 2],
    dc: [(f64, f64); 2],
    sat_amt: Ramp,
    comp_key: (f32, bool),
    comp_val: f64,
    pub meters: Meters,
}

fn ramp_samples(ms: f32, fs: f32) -> f32 {
    ms * 0.001 * fs
}

/// Static compressor curve: gain reduction in dB (positive) for an input level.
pub fn comp_reduction(level_db: f32, thresh: f32, ratio: f32, knee: f32) -> f32 {
    let over = level_db - thresh;
    let slope = 1.0 - 1.0 / ratio.max(1.0);
    if knee <= 0.0 {
        return if over > 0.0 { over * slope } else { 0.0 };
    }
    if 2.0 * over < -knee {
        0.0
    } else if 2.0 * over.abs() <= knee {
        slope * (over + knee * 0.5).powi(2) / (2.0 * knee)
    } else {
        over * slope
    }
}

impl Engine {
    pub fn new(fs: f64) -> Self {
        let fs = fs as f32;
        let mut e = Self {
            fs,
            s: Settings::default(),
            initialised: false,
            in_g: Ramp::new(1.0),
            out_g: Ramp::new(1.0),
            mix: Ramp::new(1.0),
            amt: [Ramp::new(0.0); BLOCKS],
            gate_env: 0.0,
            gate_open: false,
            gate_hold: 0,
            gate_gain: 1.0,
            hp: [[Biquad::identity(); 2]; 2],
            bp: [Biquad::identity(); 2],
            ds_env: 0.0,
            ds_gr: 0.0,
            comp_env: 0.0,
            comp_ms: 0.0,
            comp_gr: 0.0,
            comp_mem: 0.0,
            os: [Oversampler::default(); 2],
            tape_lp: [0.0; 2],
            dc: [(0.0, 0.0); 2],
            sat_amt: Ramp::new(0.0),
            comp_key: (-1.0, false),
            comp_val: 1.0,
            meters: Meters::default(),
        };
        e.design();
        e
    }

    pub fn sample_rate(&self) -> f32 {
        self.fs
    }

    pub fn reset(&mut self, fs: f64) {
        let s = self.s;
        let init = self.initialised;
        *self = Engine::new(fs);
        if init {
            self.set_settings(&s);
        }
    }

    pub fn gate_is_open(&self) -> bool {
        self.gate_open
    }

    fn design(&mut self) {
        let fs = self.fs as f64;
        let s = self.s;
        let f = (s.hpf_freq as f64).clamp(20.0, 400.0);
        let (q1, q2) = if s.hpf_24 { (0.5411961, 1.3065630) } else { (std::f64::consts::FRAC_1_SQRT_2, 0.0) };
        for c in 0..2 {
            self.hp[c][0].set_coeffs(&Biquad::highpass(fs, f, q1));
            self.hp[c][1].set_coeffs(&if s.hpf_24 { Biquad::highpass(fs, f, q2) } else { Biquad::identity() });
            self.bp[c].set_coeffs(&Biquad::bandpass(fs, s.ds_freq as f64, 2.2));
        }
    }

    pub fn set_settings(&mut self, s: &Settings) {
        let fs = self.fs;
        let first = !self.initialised;
        let changed_design = first || s.hpf_freq != self.s.hpf_freq || s.hpf_24 != self.s.hpf_24 || s.ds_freq != self.s.ds_freq;
        if first {
            self.in_g.snap(db_to_gain(s.in_gain_db));
            self.out_g.snap(db_to_gain(s.out_gain_db));
            self.mix.snap(s.mix);
            for k in 0..BLOCKS {
                self.amt[k].snap(if s.on[k] { 1.0 } else { 0.0 });
            }
            self.sat_amt.snap(s.sat_drive);
            self.initialised = true;
        }
        let n20 = ramp_samples(20.0, fs);
        self.in_g.set(db_to_gain(s.in_gain_db), n20);
        self.out_g.set(db_to_gain(s.out_gain_db), n20);
        self.mix.set(s.mix, n20);
        self.sat_amt.set(s.sat_drive, n20);
        for k in 0..BLOCKS {
            self.amt[k].set(if s.on[k] { 1.0 } else { 0.0 }, ramp_samples(10.0, fs));
        }
        self.s = *s;
        if changed_design {
            self.design();
        }
    }

    /// Auto makeup: the static curve's reduction at -18 dBFS in.
    pub fn auto_makeup(s: &Settings) -> f32 {
        comp_reduction(-18.0, s.comp_thresh, s.comp_ratio, s.comp_knee)
    }

    /// Process `l` (and `r` for stereo) in place.
    pub fn process(&mut self, l: &mut [f32], mut r: Option<&mut [f32]>) {
        let n = l.len();
        let fs = self.fs;
        let s = self.s;
        let nch = if r.is_some() { 2 } else { 1 };
        let gate_dec = tau_coef(0.005, fs);
        let (g_atk, g_rel) = (tau_coef(s.gate_attack_ms * 0.001, fs), tau_coef(s.gate_release_ms * 0.001, fs));
        let hold_n = (0.020 * fs) as u32;
        let ds_dec = tau_coef(0.025, fs);
        let (ds_atk, ds_rel) = (tau_coef(0.001, fs), tau_coef(0.02, fs));
        let det_dec = tau_coef(0.010, fs);
        let rms_c = tau_coef(0.010, fs);
        let (mut c_atk, mut c_rel) = (tau_coef(s.comp_attack_ms * 0.001, fs), tau_coef(s.comp_release_ms * 0.001, fs));
        let mem_c = tau_coef(1.5, fs);
        let makeup = s.comp_makeup + if s.comp_auto { Self::auto_makeup(&s) } else { 0.0 };
        let (mut in_pk, mut out_pk, mut ds_pk, mut comp_pk) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);

        for i in 0..n {
            let raw = [l[i], r.as_ref().map_or(0.0, |r| r[i])];
            in_pk = in_pk.max(raw[0].abs()).max(raw[1].abs());
            let mut x = raw;
            if self.in_g.moving() || self.in_g.cur != 1.0 {
                let g = self.in_g.tick();
                x = [x[0] * g, x[1] * g];
            }

            // ---- gate ----
            let (a, moving) = (self.amt[GATE].tick(), self.amt[GATE].moving());
            if a > 0.0 || moving || self.gate_open {
                let det = x[0].abs().max(x[1].abs());
                self.gate_env = det.max(self.gate_env * gate_dec);
                let lvl = 20.0 * (self.gate_env + 1e-9).log10();
                if !self.gate_open {
                    if lvl >= s.gate_thresh {
                        self.gate_open = true;
                        self.gate_hold = hold_n;
                    }
                } else if lvl >= s.gate_thresh - 4.0 {
                    self.gate_hold = hold_n;
                } else if self.gate_hold > 0 {
                    self.gate_hold -= 1;
                } else {
                    self.gate_open = false;
                }
                let target = if self.gate_open { 1.0 } else { db_to_gain(-s.gate_range) };
                let c = if target > self.gate_gain { g_atk } else { g_rel };
                self.gate_gain = target + (self.gate_gain - target) * c;
                let g = self.gate_gain;
                for ch in 0..nch {
                    let y = x[ch] * g;
                    x[ch] = if a >= 1.0 { y } else { x[ch] + (y - x[ch]) * a };
                }
            } else {
                self.gate_gain = 1.0;
            }

            // ---- high-pass ----
            let a = self.amt[HPF].tick();
            if a > 0.0 {
                for ch in 0..nch {
                    let mut y = self.hp[ch][0].process(x[ch] as f64);
                    if s.hpf_24 {
                        y = self.hp[ch][1].process(y);
                    }
                    let y = y as f32;
                    x[ch] = if a >= 1.0 { y } else { x[ch] + (y - x[ch]) * a };
                }
            }

            // ---- de-esser ----
            let a = self.amt[DESS].tick();
            if a > 0.0 || self.amt[DESS].moving() {
                let band = [self.bp[0].process(x[0] as f64) as f32, if nch > 1 { self.bp[1].process(x[1] as f64) as f32 } else { 0.0 }];
                let det = band[0].abs().max(band[1].abs());
                self.ds_env = det.max(self.ds_env * ds_dec);
                let lvl = 20.0 * (self.ds_env + 1e-9).log10();
                let red = (lvl - s.ds_thresh).clamp(0.0, s.ds_range);
                let c = if red > self.ds_gr { ds_atk } else { ds_rel };
                self.ds_gr = red + (self.ds_gr - red) * c;
                ds_pk = ds_pk.max(self.ds_gr);
                let g = db_to_gain(-self.ds_gr);
                for ch in 0..nch {
                    let y = if s.ds_listen { band[ch] } else if s.ds_wide { x[ch] * g } else { x[ch] - (1.0 - g) * band[ch] };
                    x[ch] = if a >= 1.0 { y } else { x[ch] + (y - x[ch]) * a };
                }
            } else {
                self.ds_gr = 0.0;
            }

            // ---- compressor ----
            let a = self.amt[COMP].tick();
            if a > 0.0 || self.amt[COMP].moving() || self.comp_gr > 0.001 {
                let lvl = if s.comp_opto {
                    let p = (x[0] * x[0]).max(x[1] * x[1]);
                    self.comp_ms = p + (self.comp_ms - p) * rms_c;
                    10.0 * (self.comp_ms + 1e-18).log10() + 3.0103
                } else {
                    let det = x[0].abs().max(x[1].abs());
                    self.comp_env = det.max(self.comp_env * det_dec);
                    20.0 * (self.comp_env + 1e-9).log10()
                };
                let target = comp_reduction(lvl, s.comp_thresh, s.comp_ratio, s.comp_knee);
                if s.comp_opto {
                    // program-dependent release: short peaks recover fast, long loud passages slowly
                    c_atk = tau_coef(s.comp_attack_ms * 0.0015, fs);
                    c_rel = tau_coef(s.comp_release_ms * 0.001 * (0.25 + 1.75 * self.comp_mem), fs);
                    self.comp_mem = (self.comp_gr / 15.0).min(1.0) + (self.comp_mem - (self.comp_gr / 15.0).min(1.0)) * mem_c;
                }
                let c = if target > self.comp_gr { c_atk } else { c_rel };
                self.comp_gr = target + (self.comp_gr - target) * c;
                comp_pk = comp_pk.max(self.comp_gr);
                let g = db_to_gain(makeup - self.comp_gr);
                for ch in 0..nch {
                    let y = x[ch] * g;
                    x[ch] = if a >= 1.0 { y } else { x[ch] + (y - x[ch]) * a };
                }
            }

            // ---- saturation (2x oversampled inside) ----
            let a = self.amt[SAT].tick();
            let drive = self.sat_amt.tick();
            if (a > 0.0 || self.amt[SAT].moving()) && drive > 0.0 {
                let w = (drive * 4.0).min(1.0);
                let g = 1.0 + 11.0 * drive as f64; // up to +21 dB
                let tube = s.sat_tube;
                let bias = 0.25;
                let rolloff = if tube { 0.0 } else { (drive as f64) * 0.8 }; // tape: duller as Drive rises
                if self.comp_key != (drive, tube) {
                    // level compensation: the curve's RMS on a -18 dBFS sine is brought back to the sine's own
                    let curve = |v: f64| if tube { (g * v + bias).tanh() - bias.tanh() } else { (g * v).tanh() };
                    let (mut sum, mut mean) = (0.0, 0.0);
                    for k in 0..64 {
                        let y = curve(0.126 * (std::f64::consts::TAU * k as f64 / 64.0).sin());
                        sum += y * y;
                        mean += y;
                    }
                    let var = sum / 64.0 - (mean / 64.0).powi(2);
                    self.comp_val = (0.126 * std::f64::consts::FRAC_1_SQRT_2 / var.sqrt().max(1e-9)).min(8.0);
                    self.comp_key = (drive, tube);
                }
                let comp = self.comp_val;
                for ch in 0..nch {
                    let xin = x[ch] as f64;
                    let mut lp = self.tape_lp[ch];
                    let coef = 1.0 - rolloff * 0.85; // one-pole smoothing on the oversampled signal
                    let y = self.os[ch].process(xin, |v| {
                        let t = if tube { ((g * v + bias).tanh() - bias.tanh()) * comp } else { (g * v).tanh() * comp };
                        lp += (t - lp) * coef;
                        lp
                    });
                    self.tape_lp[ch] = lp;
                    let mut y = y;
                    if tube {
                        // asymmetric curve makes DC: block it
                        let (px, py) = self.dc[ch];
                        let o = y - px + 0.9995 * py;
                        self.dc[ch] = (y, o);
                        y = o;
                    }
                    let wet = xin * (1.0 - w as f64) + y * w as f64;
                    let wet = xin + (wet - xin) * s.sat_mix as f64;
                    let wet = wet as f32;
                    x[ch] = if a >= 1.0 { wet } else { x[ch] + (wet - x[ch]) * a };
                }
            }

            // ---- output gain, mix ----
            if self.out_g.moving() || self.out_g.cur != 1.0 {
                let g = self.out_g.tick();
                x = [x[0] * g, x[1] * g];
            }
            let m = self.mix.tick();
            let out = if m >= 1.0 { x } else { [raw[0] + (x[0] - raw[0]) * m, raw[1] + (x[1] - raw[1]) * m] };
            out_pk = out_pk.max(out[0].abs()).max(out[1].abs());
            l[i] = out[0];
            if let Some(r) = r.as_mut() {
                r[i] = out[1];
            }
        }
        self.meters.in_peak = self.meters.in_peak.max(in_pk);
        self.meters.out_peak = self.meters.out_peak.max(out_pk);
        self.meters.ds_gr = self.meters.ds_gr.max(ds_pk);
        self.meters.comp_gr = self.meters.comp_gr.max(comp_pk);
        self.meters.gate_open = self.gate_open;
    }

    pub fn take_meters(&mut self) -> Meters {
        let m = self.meters;
        self.meters.in_peak = 0.0;
        self.meters.out_peak = 0.0;
        self.meters.ds_gr = 0.0;
        self.meters.comp_gr = 0.0;
        m
    }
}

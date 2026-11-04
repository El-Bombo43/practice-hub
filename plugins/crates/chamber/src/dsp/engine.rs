//! The Chamber engine: input filters, pre-delay, tank (with a 100 ms mode crossfade), width, ducking, freeze, mix.
use super::tank::{Tank, TankParams};
use super::util::{db_to_gain, Biquad, Delay, Ramp};

const BLOCK: usize = 32;
/// Pre-delay note values in beats (a quarter note = 1 beat): 1/64, 1/32T, 1/32, 1/16T, 1/16, 1/16D, 1/8T, 1/8, 1/8D, 1/4.
pub const DIV_BEATS: [f32; 10] = [0.0625, 0.0833333, 0.125, 0.1666667, 0.25, 0.375, 0.3333333, 0.5, 0.75, 1.0];
const SLEEP_LEVEL: f32 = 1.0e-7; // -140 dBFS
const MAX_PREDELAY_MS: f32 = 2000.0;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// 0 Plate, 1 Hall, 2 Room
    pub mode: usize,
    /// 0..1
    pub mix: f32,
    pub predelay_ms: f32,
    pub predelay_sync: bool,
    pub predelay_div: usize,
    /// 0..1
    pub size: f32,
    pub decay_s: f32,
    /// 0..1
    pub diffusion: f32,
    pub early_db: f32,
    pub low_cut: f32,
    pub high_cut: f32,
    pub damping_hz: f32,
    pub bass_decay: f32,
    pub mod_rate: f32,
    /// 0..1
    pub mod_depth: f32,
    /// 0..1.5
    pub width: f32,
    pub duck_db: f32,
    pub duck_thresh_db: f32,
    pub duck_release_ms: f32,
    pub freeze: bool,
    pub out_gain_db: f32,
    pub tempo_bpm: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: 0,
            mix: 0.3,
            predelay_ms: 10.0,
            predelay_sync: false,
            predelay_div: 2,
            size: 0.5,
            decay_s: 2.0,
            diffusion: 0.75,
            early_db: 0.0,
            low_cut: 120.0,
            high_cut: 12_000.0,
            damping_hz: 7000.0,
            bass_decay: 1.0,
            mod_rate: 0.6,
            mod_depth: 0.25,
            width: 1.0,
            duck_db: 0.0,
            duck_thresh_db: -30.0,
            duck_release_ms: 400.0,
            freeze: false,
            out_gain_db: 0.0,
            tempo_bpm: 120.0,
        }
    }
}

/// Peaks since the last time the host-side code collected them.
#[derive(Clone, Copy, Default, Debug)]
pub struct Meters {
    pub in_peak: f32,
    pub wet_peak: f32,
    /// duck gain reduction, dB (positive)
    pub gr_db: f32,
    /// smoothed wet level (RMS-ish, linear)
    pub wet_level: f32,
}

pub struct Engine {
    fs: f32,
    tanks: [Tank; 3],
    mode: usize,
    prev_mode: Option<usize>,
    xf: f32,
    xf_step: f32,
    asleep: bool,
    quiet: u32,
    counter: usize,
    // filters
    hp: [Biquad; 2],
    lp: [Biquad; 2],
    low_cut: Ramp,
    high_cut: Ramp,
    // pre-delay
    pre: [Delay; 2],
    pre_cur: f32,
    pre_target: f32,
    pre_k: f32,
    // smoothed parameters
    mix: Ramp,
    out_db: Ramp,
    width: Ramp,
    duck_amt: Ramp,
    duck_thresh: Ramp,
    size: Ramp,
    decay: Ramp,
    diffusion: Ramp,
    damping: Ramp,
    bass: Ramp,
    mod_rate: Ramp,
    mod_depth: Ramp,
    early: Ramp,
    freeze_amt: Ramp,
    // cached
    dry_g: f32,
    wet_g: f32,
    out_g: f32,
    duck_env: f32,
    duck_attack: f32,
    duck_release: f32,
    gr_db_now: f32,
    pub meters: Meters,
    s: Settings,
    initialised: bool,
}

impl Engine {
    pub fn new(fs: f64) -> Self {
        let fs = fs as f32;
        let mut e = Engine {
            fs,
            tanks: [Tank::new(fs, 0), Tank::new(fs, 1), Tank::new(fs, 2)],
            mode: 0,
            prev_mode: None,
            xf: 1.0,
            xf_step: 0.0,
            asleep: true,
            quiet: 0,
            counter: 0,
            hp: [Biquad::default(); 2],
            lp: [Biquad::default(); 2],
            low_cut: Ramp::new(120.0),
            high_cut: Ramp::new(12_000.0),
            pre: [Delay::new((fs * (MAX_PREDELAY_MS * 0.001 + 0.01)) as usize), Delay::new((fs * (MAX_PREDELAY_MS * 0.001 + 0.01)) as usize)],
            pre_cur: 0.0,
            pre_target: 0.0,
            pre_k: 1.0 - (-1.0 / (0.05 * fs)).exp(),
            mix: Ramp::new(0.3),
            out_db: Ramp::new(0.0),
            width: Ramp::new(1.0),
            duck_amt: Ramp::new(0.0),
            duck_thresh: Ramp::new(-30.0),
            size: Ramp::new(0.5),
            decay: Ramp::new(2.0),
            diffusion: Ramp::new(0.75),
            damping: Ramp::new(7000.0),
            bass: Ramp::new(1.0),
            mod_rate: Ramp::new(0.6),
            mod_depth: Ramp::new(0.25),
            early: Ramp::new(1.0),
            freeze_amt: Ramp::new(0.0),
            dry_g: 1.0,
            wet_g: 0.0,
            out_g: 1.0,
            duck_env: 0.0,
            duck_attack: (-1.0 / (0.005 * fs)).exp(),
            duck_release: (-1.0 / (0.4 * fs)).exp(),
            gr_db_now: 0.0,
            meters: Meters::default(),
            s: Settings::default(),
            initialised: false,
        };
        e.snap_all(&Settings::default());
        e.initialised = false; // the first settings call snaps instead of ramping
        e
    }

    pub fn sample_rate(&self) -> f32 {
        self.fs
    }

    /// Clear everything (sample-rate change, host reset). Settings are kept.
    pub fn reset(&mut self, fs: f64) {
        let s = self.s;
        *self = Engine::new(fs);
        self.s = s;
        self.snap_all(&s);
    }

    fn pre_delay_samples(&self, s: &Settings) -> f32 {
        let ms = if s.predelay_sync {
            let bpm = if s.tempo_bpm.is_finite() && s.tempo_bpm > 1.0 { s.tempo_bpm } else { 120.0 };
            (DIV_BEATS[s.predelay_div.min(9)] as f64 * 60_000.0 / bpm) as f32
        } else {
            s.predelay_ms
        };
        (ms.clamp(0.0, MAX_PREDELAY_MS) * 0.001 * self.fs).min(self.pre[0].capacity() as f32 - 4.0)
    }

    fn snap_all(&mut self, s: &Settings) {
        self.mode = s.mode.min(2);
        self.prev_mode = None;
        self.xf = 1.0;
        self.mix.snap(s.mix);
        self.out_db.snap(s.out_gain_db);
        self.width.snap(s.width);
        self.duck_amt.snap(s.duck_db);
        self.duck_thresh.snap(s.duck_thresh_db);
        self.size.snap(s.size);
        self.decay.snap(s.decay_s);
        self.diffusion.snap(s.diffusion);
        self.damping.snap(s.damping_hz);
        self.bass.snap(s.bass_decay);
        self.mod_rate.snap(s.mod_rate);
        self.mod_depth.snap(s.mod_depth);
        self.early.snap(db_to_gain(s.early_db));
        self.low_cut.snap(s.low_cut);
        self.high_cut.snap(s.high_cut);
        self.freeze_amt.snap(if s.freeze { 1.0 } else { 0.0 });
        self.pre_target = self.pre_delay_samples(s);
        self.pre_cur = self.pre_target;
        self.update_mix_gains();
        self.out_g = db_to_gain(s.out_gain_db);
        self.duck_release = (-1.0 / (s.duck_release_ms.max(1.0) * 0.001 * self.fs)).exp();
        self.initialised = true;
    }

    pub fn set_settings(&mut self, s: &Settings) {
        let fs = self.fs;
        let ms = |t: f32| t * 0.001 * fs;
        if !self.initialised {
            self.snap_all(s);
        }
        if s.mode.min(2) != self.mode {
            if let Some(p) = self.prev_mode.take() {
                self.tanks[p].clear();
            }
            self.prev_mode = Some(self.mode);
            self.mode = s.mode.min(2);
            self.xf = 0.0;
            self.xf_step = 1.0 / ms(100.0);
            self.asleep = false;
            self.quiet = 0;
            self.counter = 0; // recompute tank controls before the first sample
        }
        self.mix.set(s.mix, ms(20.0));
        self.out_db.set(s.out_gain_db, ms(20.0));
        self.width.set(s.width, ms(20.0));
        self.duck_amt.set(s.duck_db, ms(20.0));
        self.duck_thresh.set(s.duck_thresh_db, ms(20.0));
        self.size.set(s.size, ms(200.0));
        self.decay.set(s.decay_s.max(0.05), ms(50.0));
        self.diffusion.set(s.diffusion, ms(50.0));
        self.damping.set(s.damping_hz, ms(50.0));
        self.bass.set(s.bass_decay, ms(50.0));
        self.mod_rate.set(s.mod_rate, ms(50.0));
        self.mod_depth.set(s.mod_depth, ms(50.0));
        self.early.set(db_to_gain(s.early_db), ms(20.0));
        self.low_cut.set(s.low_cut, ms(20.0));
        self.high_cut.set(s.high_cut, ms(20.0));
        if s.freeze != self.s.freeze {
            if self.asleep {
                self.counter = 0;
            }
            self.asleep = false;
            self.quiet = 0;
        }
        self.freeze_amt.set(if s.freeze { 1.0 } else { 0.0 }, ms(50.0));
        self.pre_target = self.pre_delay_samples(s);
        self.duck_release = (-1.0 / (s.duck_release_ms.max(1.0) * 0.001 * fs)).exp();
        self.s = *s;
    }

    fn update_mix_gains(&mut self) {
        let a = self.mix.cur.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
        self.dry_g = if self.mix.cur <= 0.0 { 1.0 } else { a.cos() };
        self.wet_g = if self.mix.cur <= 0.0 { 0.0 } else { a.sin() };
    }

    fn control_tick(&mut self) {
        let n = BLOCK as f32;
        let fs = self.fs as f64;
        let lc = self.low_cut.tick(n);
        let hc = self.high_cut.tick(n);
        for c in 0..2 {
            self.hp[c].set_highpass(fs, lc as f64);
            self.lp[c].set_lowpass(fs, hc as f64);
        }
        let tp = TankParams {
            scale: 0.4 + 1.2 * self.size.tick(n),
            rt60: self.decay.tick(n),
            diffusion: self.diffusion.tick(n),
            damping_hz: self.damping.tick(n),
            bass: self.bass.tick(n),
            mod_rate: self.mod_rate.tick(n),
            mod_depth: self.mod_depth.tick(n),
            early_gain: self.early.tick(n),
        };
        if !self.asleep {
            self.tanks[self.mode].control(&tp);
            if let Some(p) = self.prev_mode {
                self.tanks[p].control(&tp);
            }
        }
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        let (mut in_pk, mut wet_pk, mut gr_pk) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..n {
            if self.counter == 0 {
                if self.asleep {
                    // keep ramps and filters tracking while the tanks sleep
                    self.control_tick_asleep();
                } else {
                    self.control_tick();
                }
            }
            self.counter = (self.counter + 1) % BLOCK;

            let (xl, xr) = (l[i], r[i]);
            in_pk = in_pk.max(xl.abs()).max(xr.abs());
            let (sl, sr) = (if xl.is_finite() { xl } else { 0.0 }, if xr.is_finite() { xr } else { 0.0 });

            // freeze amount: input fades out as the loop becomes lossless
            let fz = self.freeze_amt.tick(1.0);
            // input filters
            let fl = self.lp[0].process(self.hp[0].process(sl));
            let fr = self.lp[1].process(self.hp[1].process(sr));
            // pre-delay (exponentially smoothed so tempo changes and sweeps glide)
            self.pre[0].push(fl);
            self.pre[1].push(fr);
            let diff = self.pre_target - self.pre_cur;
            if diff.abs() < 1e-3 {
                self.pre_cur = self.pre_target;
            } else {
                self.pre_cur += diff * self.pre_k;
            }
            let (pl, pr) = (self.pre[0].read_after_push(self.pre_cur), self.pre[1].read_after_push(self.pre_cur));
            // the tank wakes the moment (pre-delayed) sound reaches it
            if self.asleep && (pl.abs() > SLEEP_LEVEL || pr.abs() > SLEEP_LEVEL) {
                self.asleep = false;
                self.quiet = 0;
                self.control_tick();
            }
            let fin = (1.0 - fz) * (1.0 - fz); // input fades out over the 50 ms freeze ramp, front-loaded so the held level never overshoots

            let (mut wl, mut wr) = (0.0f32, 0.0f32);
            if !self.asleep {
                let (a, b) = self.tanks[self.mode].process(pl * fin, pr * fin, fz);
                if let Some(p) = self.prev_mode {
                    let (c, d) = self.tanks[p].process(pl * fin, pr * fin, fz);
                    self.xf += self.xf_step;
                    let t = self.xf.min(1.0) * std::f32::consts::FRAC_PI_2;
                    let (gn, go) = (t.sin(), t.cos());
                    wl = a * gn + c * go;
                    wr = b * gn + d * go;
                    if self.xf >= 1.0 {
                        self.tanks[p].clear();
                        self.prev_mode = None;
                    }
                } else {
                    wl = a;
                    wr = b;
                }
                if !(wl.is_finite() && wr.is_finite()) {
                    for t in self.tanks.iter_mut() {
                        t.clear();
                    }
                    wl = 0.0;
                    wr = 0.0;
                }
                if wl.abs() < SLEEP_LEVEL && wr.abs() < SLEEP_LEVEL && fz < 1e-3 && !self.freeze_amt.moving() {
                    self.quiet += 1;
                    if self.quiet as f32 > 0.5 * self.fs {
                        self.asleep = true;
                        for t in self.tanks.iter_mut() {
                            t.clear();
                        }
                        wl = 0.0;
                        wr = 0.0;
                    }
                } else {
                    self.quiet = 0;
                }
            }

            // width (M/S)
            let w = self.width.tick(1.0);
            let (m, s) = (0.5 * (wl + wr), 0.5 * (wl - wr) * w);
            wl = m + s;
            wr = m - s;

            // ducking: the wet turns down while the dry input is loud
            let amt = self.duck_amt.tick(1.0);
            let thr = self.duck_thresh.tick(1.0);
            let mut gr = 0.0;
            if amt > 0.0 || self.duck_amt.moving() {
                let x = sl.abs().max(sr.abs());
                let a = if x > self.duck_env { self.duck_attack } else { self.duck_release };
                self.duck_env = x + a * (self.duck_env - x);
                let over = 20.0 * (self.duck_env + 1e-9).log10() - thr;
                let raw = if over <= -3.0 {
                    0.0
                } else if over < 3.0 {
                    (over + 3.0) * (over + 3.0) / 144.0
                } else {
                    (over / 12.0).min(1.0)
                };
                gr = amt * raw;
                let g = db_to_gain(-gr);
                wl *= g;
                wr *= g;
            } else {
                self.duck_env = 0.0;
            }
            self.gr_db_now = gr;
            gr_pk = gr_pk.max(gr);

            let wp = wl.abs().max(wr.abs());
            wet_pk = wet_pk.max(wp);
            self.meters.wet_level += (wp - self.meters.wet_level) * (1.0 - (-1.0 / (0.1 * self.fs)).exp());

            // mix (equal power) and output gain
            if self.mix.moving() {
                self.mix.tick(1.0);
                self.update_mix_gains();
            }
            if self.out_db.moving() {
                self.out_g = db_to_gain(self.out_db.tick(1.0));
            }
            l[i] = (xl * self.dry_g + wl * self.wet_g) * self.out_g;
            r[i] = (xr * self.dry_g + wr * self.wet_g) * self.out_g;
        }
        self.meters.in_peak = self.meters.in_peak.max(in_pk);
        self.meters.wet_peak = self.meters.wet_peak.max(wet_pk);
        self.meters.gr_db = self.meters.gr_db.max(gr_pk);
    }

    fn control_tick_asleep(&mut self) {
        let n = BLOCK as f32;
        let fs = self.fs as f64;
        let lc = self.low_cut.tick(n);
        let hc = self.high_cut.tick(n);
        for c in 0..2 {
            self.hp[c].set_highpass(fs, lc as f64);
            self.lp[c].set_lowpass(fs, hc as f64);
        }
        for r in [&mut self.size, &mut self.decay, &mut self.diffusion, &mut self.damping, &mut self.bass, &mut self.mod_rate, &mut self.mod_depth, &mut self.early] {
            r.tick(n);
        }
    }

    pub fn is_asleep(&self) -> bool {
        self.asleep
    }

    /// Take the peaks gathered since the last call.
    pub fn take_meters(&mut self) -> Meters {
        let m = self.meters;
        self.meters.in_peak = 0.0;
        self.meters.wet_peak = 0.0;
        self.meters.gr_db = 0.0;
        m
    }

    pub fn duck_gr_db(&self) -> f32 {
        self.gr_db_now
    }
}

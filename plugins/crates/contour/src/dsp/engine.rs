//! The whole EQ: 16 bands in series, solo, Gain Scale, Auto Gain and Output Gain.

use super::band::{BandRt, BLOCK};
use super::design::{solo_filter, Biquad, FilterType};
use super::response::mean_db_100_10k;

pub use super::design::BAND_COUNT;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandSettings {
    pub used: bool,
    pub enabled: bool,
    pub ftype: FilterType,
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
    pub slope: u32,
    pub dynamic: bool,
    pub thresh_db: f32,
    pub range_db: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub ext: bool,
}

impl Default for BandSettings {
    fn default() -> Self {
        Self {
            used: false,
            enabled: true,
            ftype: FilterType::Bell,
            freq: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            slope: 12,
            dynamic: false,
            thresh_db: -20.0,
            range_db: -6.0,
            attack_ms: 10.0,
            release_ms: 150.0,
            ext: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalSettings {
    pub out_gain_db: f32,
    pub auto_gain: bool,
    /// 0.0 to 2.0 (the 0 to 200 % parameter).
    pub gain_scale: f32,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self { out_gain_db: 0.0, auto_gain: false, gain_scale: 1.0 }
    }
}

pub struct Engine {
    fs: f64,
    pub bands: Vec<BandRt>,
    bands_prev: [BandSettings; BAND_COUNT],
    global: GlobalSettings,
    scale: f64,
    out_lin: f64,
    auto_target_db: f64,
    auto_db: f64,
    solo: Option<(usize, FilterType, f32, f32)>,
    solo_mix: f64,
    solo_filter: Biquad,
    solo_st: [[f64; 2]; 2],
    settled: bool,
}

impl Engine {
    pub fn new(fs: f64) -> Self {
        Self {
            fs,
            bands: (0..BAND_COUNT).map(|_| BandRt::new()).collect(),
            bands_prev: [BandSettings::default(); BAND_COUNT],
            global: GlobalSettings::default(),
            scale: 1.0,
            out_lin: 1.0,
            auto_target_db: 0.0,
            auto_db: 0.0,
            solo: None,
            solo_mix: 0.0,
            solo_filter: Biquad::IDENTITY,
            solo_st: [[0.0; 2]; 2],
            settled: false,
        }
    }

    pub fn sample_rate(&self) -> f64 {
        self.fs
    }

    /// New sample rate or host reset: recompute everything and clear all state.
    pub fn reset(&mut self, fs: f64) {
        self.fs = fs;
        for b in self.bands.iter_mut() {
            b.reset();
        }
        self.solo_st = [[0.0; 2]; 2];
        self.settled = false;
    }

    pub fn set_settings(&mut self, bands: &[BandSettings; BAND_COUNT], global: GlobalSettings) {
        let changed = !self.settled || *bands != self.bands_prev || global != self.global;
        for (rt, s) in self.bands.iter_mut().zip(bands.iter()) {
            rt.s = *s;
        }
        if changed {
            self.bands_prev = *bands;
            self.global = global;
            self.auto_target_db = if global.auto_gain {
                (-mean_db_100_10k(bands, global.gain_scale as f64, self.fs)).clamp(-24.0, 24.0)
            } else {
                0.0
            };
            if !self.settled {
                self.scale = global.gain_scale as f64;
                self.auto_db = self.auto_target_db;
                self.out_lin = 10f64.powf(global.out_gain_db as f64 / 20.0);
                self.settled = true;
            }
        }
    }

    /// Solo one band (editor state): output becomes the input through that band's solo filter. `None` clears it.
    pub fn set_solo(&mut self, solo: Option<usize>) {
        self.solo = solo.filter(|&i| i < BAND_COUNT).map(|i| {
            let b = &self.bands_prev[i];
            (i, b.ftype, b.freq, b.q)
        });
    }

    /// Live dynamic movement of band `i` in dB, for the editor.
    pub fn dyn_db(&self, i: usize) -> f32 {
        self.bands[i].dyn_db as f32
    }

    /// Stereo in place. `sc` is the external sidechain (same length), if the host provides one.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], sc: Option<(&[f32], &[f32])>) {
        let len = l.len().min(r.len());
        let mut i = 0;
        while i < len {
            let n = BLOCK.min(len - i);
            let mut buf = [[0.0f64; BLOCK]; 2];
            let mut scb = [[0.0f64; BLOCK]; 2];
            for k in 0..n {
                buf[0][k] = l[i + k] as f64;
                buf[1][k] = r[i + k] as f64;
            }
            if let Some((a, b)) = sc {
                for k in 0..n {
                    scb[0][k] = a.get(i + k).copied().unwrap_or(0.0) as f64;
                    scb[1][k] = b.get(i + k).copied().unwrap_or(0.0) as f64;
                }
            }
            self.block(2, &mut buf, n, sc.is_some().then_some(&scb));
            for k in 0..n {
                l[i + k] = buf[0][k] as f32;
                r[i + k] = buf[1][k] as f32;
            }
            i += n;
        }
    }

    /// Mono in place.
    pub fn process_mono(&mut self, x: &mut [f32], sc: Option<&[f32]>) {
        let mut i = 0;
        while i < x.len() {
            let n = BLOCK.min(x.len() - i);
            let mut buf = [[0.0f64; BLOCK]; 2];
            let mut scb = [[0.0f64; BLOCK]; 2];
            for k in 0..n {
                buf[0][k] = x[i + k] as f64;
            }
            if let Some(s) = sc {
                for k in 0..n {
                    scb[0][k] = s.get(i + k).copied().unwrap_or(0.0) as f64;
                }
            }
            self.block(1, &mut buf, n, sc.is_some().then_some(&scb));
            for k in 0..n {
                x[i + k] = buf[0][k] as f32;
            }
            i += n;
        }
    }

    fn block(&mut self, nch: usize, buf: &mut [[f64; BLOCK]; 2], n: usize, sc: Option<&[[f64; BLOCK]; 2]>) {
        let nf = n as f64;
        let alpha = 1.0 - (-nf / (self.fs * 0.007)).exp();
        self.scale += (self.global.gain_scale as f64 - self.scale) * alpha;
        if (self.scale - self.global.gain_scale as f64).abs() < 1e-7 {
            self.scale = self.global.gain_scale as f64;
        }
        let out_t = 10f64.powf(self.global.out_gain_db as f64 / 20.0);
        self.out_lin += (out_t - self.out_lin) * alpha;
        if (self.out_lin - out_t).abs() < 1e-9 * out_t {
            self.out_lin = out_t;
        }
        let auto_alpha = 1.0 - (-nf / (self.fs * 0.0167)).exp();
        self.auto_db += (self.auto_target_db - self.auto_db) * auto_alpha;
        if (self.auto_db - self.auto_target_db).abs() < 1e-6 {
            self.auto_db = self.auto_target_db;
        }

        let dry = *buf;
        let solo_target = if self.solo.is_some() { 1.0 } else { 0.0 };
        self.solo_mix += (solo_target - self.solo_mix).clamp(-nf / (self.fs * 0.01), nf / (self.fs * 0.01));

        if self.solo_mix < 1.0 {
            for b in self.bands.iter_mut() {
                b.process(nch, buf, n, sc, self.fs, self.scale);
            }
        }
        if self.solo_mix > 0.0 {
            if let Some((_, t, f, q)) = self.solo {
                self.solo_filter = solo_filter(t, f as f64, q as f64, self.fs);
            }
            let d = self.solo_filter;
            let m = self.solo_mix;
            for ch in 0..nch {
                for k in 0..n {
                    let x = dry[ch][k];
                    let z = &mut self.solo_st[ch];
                    let y = d.b0 * x + z[0];
                    z[0] = d.b1 * x - d.a1 * y + z[1];
                    z[1] = d.b2 * x - d.a2 * y;
                    buf[ch][k] = buf[ch][k] * (1.0 - m) + y * m;
                }
            }
        }

        let g = self.out_lin * 10f64.powf(self.auto_db / 20.0);
        if g != 1.0 {
            for ch in 0..nch {
                for k in 0..n {
                    buf[ch][k] *= g;
                }
            }
        }
    }
}

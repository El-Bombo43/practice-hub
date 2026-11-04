//! Frequency response of the EQ curve, used by the editor, by Auto Gain, and by the tests.

use super::design::{design, DesignParams};
use super::engine::{BandSettings, BAND_COUNT};
use std::f64::consts::PI;

/// One band's response in dB at `freq` Hz. `gain_db` is the *total* gain to evaluate (static + dynamic, already
/// scaled); it is ignored by filter types that have no gain.
pub fn band_db(s: &BandSettings, gain_db: f64, fs: f64, freq: f64) -> f64 {
    let p = DesignParams { ftype: s.ftype, freq: s.freq as f64, gain_db, q: s.q as f64, slope: s.slope };
    let sections = design(&p, fs);
    if sections.n == 0 {
        return 0.0;
    }
    20.0 * sections.magnitude(2.0 * PI * freq / fs).max(1e-12).log10()
}

/// Combined static curve (all used and enabled bands) in dB at `freq`. `gain_scale` is 0..2.
/// `dyn_db[i]` adds the live dynamic movement of band i (0 for the static curve).
pub fn curve_db(bands: &[BandSettings; BAND_COUNT], gain_scale: f64, dyn_db: &[f32; BAND_COUNT], fs: f64, freq: f64) -> f64 {
    let mut total = 0.0;
    for (i, b) in bands.iter().enumerate() {
        if !(b.used && b.enabled) {
            continue;
        }
        let g = gain_scale * (b.gain_db as f64 + dyn_db[i] as f64);
        total += band_db(b, g, fs, freq);
    }
    total
}

/// Mean level of the static curve over 100 Hz to 10 kHz, equal weight per octave. Auto Gain applies the negative of this.
pub fn mean_db_100_10k(bands: &[BandSettings; BAND_COUNT], gain_scale: f64, fs: f64) -> f64 {
    const N: usize = 48;
    let zero = [0.0f32; BAND_COUNT];
    let (lo, hi) = (100.0f64.ln(), 10_000.0f64.ln());
    (0..N)
        .map(|i| {
            let f = (lo + (hi - lo) * (i as f64 + 0.5) / N as f64).exp();
            curve_db(bands, gain_scale, &zero, fs, f)
        })
        .sum::<f64>()
        / N as f64
}

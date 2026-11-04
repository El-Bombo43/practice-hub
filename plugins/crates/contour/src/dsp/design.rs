//! Filter design: analog-matched second-order sections (no cramping near Nyquist) and the filter types of the spec.
//!
//! Method: Martin Vicanek, "Matched Second Order Digital Filters" (2016). The poles come from mapping the analog
//! prototype's poles with z = e^{sT}; the numerator is solved so the digital magnitude equals the analog magnitude
//! at DC, at the centre frequency and at Nyquist. Zero latency, minimum phase.

use std::f64::consts::PI;

pub const BAND_COUNT: usize = 16;
/// Most biquad sections one band can need (96 dB/oct = 16th order = 8 biquads; tilt needs 2).
pub const MAX_SECTIONS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterType {
    #[default]
    Bell,
    LowShelf,
    HighShelf,
    TiltShelf,
    LowCut,
    HighCut,
    Notch,
    BandPass,
}

impl FilterType {
    pub fn uses_gain(self) -> bool {
        matches!(self, Self::Bell | Self::LowShelf | Self::HighShelf | Self::TiltShelf)
    }
    pub fn is_cut(self) -> bool {
        matches!(self, Self::LowCut | Self::HighCut)
    }
    pub fn allows_dynamic(self) -> bool {
        self.uses_gain()
    }
}

/// One biquad, a0 normalised to 1: y = b0 x + z1; z1 = b1 x - a1 y + z2; z2 = b2 x - a2 y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Biquad {
    pub const IDENTITY: Biquad = Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0 };

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// |H(e^{jw})| (linear) at normalised angular frequency `w` (radians per sample).
    pub fn magnitude(&self, w: f64) -> f64 {
        let (c1, s1) = (w.cos(), w.sin());
        let (c2, s2) = ((2.0 * w).cos(), (2.0 * w).sin());
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        ((nr * nr + ni * ni) / (dr * dr + di * di).max(1e-300)).sqrt()
    }
}

/// Everything needed to build a band's sections.
#[derive(Clone, Copy, Debug)]
pub struct DesignParams {
    pub ftype: FilterType,
    pub freq: f64,
    pub gain_db: f64,
    pub q: f64,
    /// Cut slope in dB/oct (6, 12, 18, 24, 36, 48, 72, 96).
    pub slope: u32,
}

/// Up to `MAX_SECTIONS` biquads, in order.
#[derive(Clone, Copy, Debug)]
pub struct Sections {
    pub n: usize,
    pub s: [Biquad; MAX_SECTIONS],
}

impl Sections {
    fn new() -> Self {
        Self { n: 0, s: [Biquad::IDENTITY; MAX_SECTIONS] }
    }
    fn push(&mut self, b: Biquad) {
        if self.n < MAX_SECTIONS {
            self.s[self.n] = b;
            self.n += 1;
        }
    }
    pub fn magnitude(&self, w: f64) -> f64 {
        self.s[..self.n].iter().map(|b| b.magnitude(w)).product()
    }
}

/// Keep the centre frequency inside the range where a second-order section is well behaved.
pub fn clamp_freq(freq: f64, fs: f64) -> f64 {
    freq.clamp(1.0, fs * 0.49)
}

pub(crate) fn solve_matched(w0: f64, rho: f64, zeta: f64, mag2: impl Fn(f64) -> f64) -> Biquad {
    // poles: analog natural frequency rho*w0, damping zeta
    let wp = rho * w0;
    let e = (-zeta * wp).exp();
    let a1 = if zeta < 1.0 {
        -2.0 * e * (wp * (1.0 - zeta * zeta).sqrt()).cos()
    } else {
        -2.0 * e * (wp * (zeta * zeta - 1.0).sqrt()).cosh()
    };
    let a2 = e * e;

    // |A|^2 = A0 phi0 + A1 phi1 + A2 phi2 with phi0 = cos^2(w/2), phi1 = sin^2(w/2), phi2 = 4 phi0 phi1
    let aa0 = (1.0 + a1 + a2).powi(2);
    let aa1 = (1.0 - a1 + a2).powi(2);
    let aa2 = -4.0 * a2;
    let phi = |w: f64| {
        let s = (w / 2.0).sin().powi(2);
        let c = 1.0 - s;
        (c, s, 4.0 * c * s)
    };
    let (p0, p1, p2) = phi(w0);
    let a_mid = aa0 * p0 + aa1 * p1 + aa2 * p2;

    let bb0 = mag2(0.0) * aa0;
    let bb1 = mag2(PI / w0) * aa1;
    let bb2 = if p2.abs() > 1e-12 { (mag2(1.0) * a_mid - bb0 * p0 - bb1 * p1) / p2 } else { 0.0 };

    let (s0, s1) = (bb0.max(0.0).sqrt(), bb1.max(0.0).sqrt());
    let w = 0.5 * (s0 + s1);
    let b0 = 0.5 * (w + (w * w + bb2).max(0.0).sqrt());
    let b1 = 0.5 * (s0 - s1);
    let b2 = if b0.abs() > 1e-15 { -bb2 / (4.0 * b0) } else { 0.0 };
    Biquad { b0, b1, b2, a1, a2 }
}

/// Peaking section with boost `gain_db` at the centre (analog: (s²+sA/Q+1)/(s²+s/(AQ)+1), A = 10^(dB/40)).
pub fn peaking(w0: f64, gain_db: f64, q: f64) -> Biquad {
    let a = 10f64.powf(gain_db / 40.0);
    let zeta = 1.0 / (2.0 * a * q);
    solve_matched(w0, 1.0, zeta, move |o| {
        let d = (1.0 - o * o).powi(2);
        (d + (o * a / q).powi(2)) / (d + (o / (a * q)).powi(2))
    })
}

pub fn low_shelf(w0: f64, gain_db: f64, q: f64) -> Biquad {
    let a = 10f64.powf(gain_db / 40.0);
    solve_matched(w0, 1.0 / a.sqrt(), 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        let num = (a - o2).powi(2) + a * o2 / (q * q);
        let den = (1.0 - a * o2).powi(2) + a * o2 / (q * q);
        a * a * num / den
    })
}

pub fn high_shelf(w0: f64, gain_db: f64, q: f64) -> Biquad {
    let a = 10f64.powf(gain_db / 40.0);
    solve_matched(w0, a.sqrt(), 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        let num = (1.0 - a * o2).powi(2) + a * o2 / (q * q);
        let den = (a - o2).powi(2) + a * o2 / (q * q);
        a * a * num / den
    })
}

pub fn high_pass2(w0: f64, q: f64) -> Biquad {
    solve_matched(w0, 1.0, 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        o2 * o2 / ((1.0 - o2).powi(2) + o2 / (q * q))
    })
}

pub fn low_pass2(w0: f64, q: f64) -> Biquad {
    solve_matched(w0, 1.0, 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        1.0 / ((1.0 - o2).powi(2) + o2 / (q * q))
    })
}

pub fn notch(w0: f64, q: f64) -> Biquad {
    solve_matched(w0, 1.0, 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        (1.0 - o2).powi(2) / ((1.0 - o2).powi(2) + o2 / (q * q))
    })
}

pub fn band_pass(w0: f64, q: f64) -> Biquad {
    solve_matched(w0, 1.0, 1.0 / (2.0 * q), move |o| {
        let o2 = o * o;
        (o2 / (q * q)) / ((1.0 - o2).powi(2) + o2 / (q * q))
    })
}

/// First-order high-pass (6 dB/oct), matched to the analog magnitude at DC and at Nyquist (no cramping).
pub fn high_pass1(w0: f64) -> Biquad {
    let p = (-w0).exp();
    let o = PI / w0;
    let m = o / (1.0 + o * o).sqrt(); // analog |H| at Nyquist
    let b0 = m * (1.0 + p) / 2.0;
    Biquad { b0, b1: -b0, b2: 0.0, a1: -p, a2: 0.0 }
}

/// First-order low-pass (6 dB/oct), matched at DC and at Nyquist.
pub fn low_pass1(w0: f64) -> Biquad {
    let p = (-w0).exp();
    let o = PI / w0;
    let m = 1.0 / (1.0 + o * o).sqrt();
    let b0 = (1.0 - p + m * (1.0 + p)) / 2.0;
    let b1 = (1.0 - p - m * (1.0 + p)) / 2.0;
    Biquad { b0, b1, b2: 0.0, a1: -p, a2: 0.0 }
}

/// Butterworth section Qs for an even/odd order: returns (first-order needed, Q of each biquad), highest-Q first.
fn butterworth_qs(order: u32) -> (bool, Vec<f64>) {
    let n = order as f64;
    let pairs = (order / 2) as usize;
    let qs = (0..pairs).map(|k| 1.0 / (2.0 * ((2 * k + 1) as f64 * PI / (2.0 * n)).sin())).collect();
    (order % 2 == 1, qs)
}

/// The sections for a band. `fs` is the sample rate. Cuts cascade Butterworth sections; the user's Q replaces the
/// highest-Q section's Q (scaled so Q = 0.7071 is exactly Butterworth).
pub fn design(p: &DesignParams, fs: f64) -> Sections {
    let w0 = 2.0 * PI * clamp_freq(p.freq, fs) / fs;
    let q = p.q.max(0.02);
    let mut out = Sections::new();
    match p.ftype {
        FilterType::Bell => {
            if p.gain_db.abs() > 1e-4 {
                out.push(peaking(w0, p.gain_db, q));
            }
        }
        FilterType::LowShelf => {
            if p.gain_db.abs() > 1e-4 {
                out.push(low_shelf(w0, p.gain_db, q));
            }
        }
        FilterType::HighShelf => {
            if p.gain_db.abs() > 1e-4 {
                out.push(high_shelf(w0, p.gain_db, q));
            }
        }
        FilterType::TiltShelf => {
            // positive gain = brighter: lows down by half, highs up by half, both pivoting at the frequency
            if p.gain_db.abs() > 1e-4 {
                out.push(low_shelf(w0, -p.gain_db / 2.0, q));
                out.push(high_shelf(w0, p.gain_db / 2.0, q));
            }
        }
        FilterType::LowCut | FilterType::HighCut => {
            let high_pass = p.ftype == FilterType::LowCut;
            let order = (p.slope / 6).clamp(1, 16);
            let (first, qs) = butterworth_qs(order);
            if first {
                out.push(if high_pass { high_pass1(w0) } else { low_pass1(w0) });
            }
            for (i, qb) in qs.iter().enumerate() {
                let qq = if i == 0 { qb * (q / std::f64::consts::FRAC_1_SQRT_2) } else { *qb };
                out.push(if high_pass { high_pass2(w0, qq) } else { low_pass2(w0, qq) });
            }
        }
        FilterType::Notch => out.push(notch(w0, q)),
        FilterType::BandPass => out.push(band_pass(w0, q)),
    }
    out
}

/// The solo filter: what a band *acts on*. Bandpass for peaking/shelf/notch/bandpass bands; for a cut, the part it removes.
pub fn solo_filter(ftype: FilterType, freq: f64, q: f64, fs: f64) -> Biquad {
    let w0 = 2.0 * PI * clamp_freq(freq, fs) / fs;
    let q = q.max(0.1);
    match ftype {
        FilterType::LowCut => low_pass2(w0, 0.7071),
        FilterType::HighCut => high_pass2(w0, 0.7071),
        _ => band_pass(w0, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Analog prototype magnitude of a peaking filter, in dB, at frequency f.
    fn analog_peak_db(f: f64, f0: f64, gain_db: f64, q: f64) -> f64 {
        let a = 10f64.powf(gain_db / 40.0);
        let o = f / f0;
        let d = (1.0 - o * o).powi(2);
        10.0 * ((d + (o * a / q).powi(2)) / (d + (o / (a * q)).powi(2))).log10()
    }

    fn digital_db(s: &Sections, f: f64, fs: f64) -> f64 {
        20.0 * s.magnitude(2.0 * PI * f / fs).log10()
    }

    #[test]
    fn matched_bell_agrees_with_analog_at_the_three_match_points() {
        let fs = 48_000.0;
        let b = peaking(2.0 * PI * 1000.0 / fs, 6.0, 1.0);
        for f in [0.01, 1000.0, 24_000.0 - 1.0] {
            let d = 20.0 * b.magnitude(2.0 * PI * f / fs).log10();
            assert!((d - analog_peak_db(f, 1000.0, 6.0, 1.0)).abs() < 0.02, "f={f} {d}");
        }
    }

    #[test]
    fn identity_for_zero_gain_bell() {
        let s = design(&DesignParams { ftype: FilterType::Bell, freq: 1000.0, gain_db: 0.0, q: 1.0, slope: 12 }, 48_000.0);
        assert_eq!(s.n, 0);
        assert!((s.magnitude(0.3) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cut_orders_and_section_counts() {
        for (slope, n) in [(6, 1), (12, 1), (18, 2), (24, 2), (36, 3), (48, 4), (72, 6), (96, 8)] {
            let s = design(&DesignParams { ftype: FilterType::LowCut, freq: 100.0, gain_db: 0.0, q: 0.7071, slope }, 48_000.0);
            assert_eq!(s.n, n, "slope {slope}");
        }
    }

    #[test]
    fn digital_magnitude_helper_is_consistent() {
        let fs = 96_000.0;
        let s = design(&DesignParams { ftype: FilterType::Bell, freq: 1000.0, gain_db: 6.0, q: 1.0, slope: 12 }, fs);
        assert!((digital_db(&s, 1000.0, fs) - 6.0).abs() < 0.01);
    }
}

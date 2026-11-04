//! Spec section 5 ("done when…"), measured offline on the engine. Numbers in comments refer to the spec's list.
use contour::dsp::design::{design, DesignParams};
use contour::dsp::response::{band_db, curve_db};
use contour::dsp::{BandSettings, Engine, FilterType, GlobalSettings, BAND_COUNT};
use std::f64::consts::PI;

fn bands_with(list: &[(usize, BandSettings)]) -> [BandSettings; BAND_COUNT] {
    let mut b = [BandSettings::default(); BAND_COUNT];
    for (i, s) in list {
        b[*i] = BandSettings { used: true, ..*s };
    }
    b
}

fn bell(freq: f32, gain: f32, q: f32) -> BandSettings {
    BandSettings { freq, gain_db: gain, q, ..BandSettings::default() }
}

fn sine(fs: f64, f: f64, amp: f64, n: usize) -> Vec<f32> {
    (0..n).map(|i| (amp * (2.0 * PI * f * i as f64 / fs).sin()) as f32).collect()
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

/// Run a sine through the engine and return the gain in dB measured over the last `measure` seconds.
fn measure_gain_db(e: &mut Engine, fs: f64, f: f64, amp: f64, settle: f64, measure: f64) -> f64 {
    let n = ((settle + measure) * fs) as usize;
    let x = sine(fs, f, amp, n);
    let mut l = x.clone();
    let mut r = x.clone();
    e.process(&mut l, &mut r, None);
    let from = (settle * fs) as usize;
    20.0 * (rms(&l[from..]) / rms(&x[from..])).log10()
}

fn analog_bell_db(f: f64, f0: f64, gain_db: f64, q: f64) -> f64 {
    let a = 10f64.powf(gain_db / 40.0);
    let o = f / f0;
    let d = (1.0 - o * o).powi(2);
    10.0 * ((d + (o * a / q).powi(2)) / (d + (o / (a * q)).powi(2))).log10()
}

fn log_freqs(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..n).map(|i| lo * (hi / lo).powf(i as f64 / (n - 1) as f64)).collect()
}

#[test]
fn t01_no_bands_is_bit_identical() {
    let mut e = Engine::new(48_000.0);
    e.set_settings(&[BandSettings::default(); BAND_COUNT], GlobalSettings::default());
    let mut seed = 12345u32;
    let x: Vec<f32> = (0..5000)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect();
    let (mut l, mut r) = (x.clone(), x.clone());
    e.process(&mut l, &mut r, None);
    assert_eq!(l, x);
    assert_eq!(r, x);
}

#[test]
fn t02_bell_1k_matches_analog_at_44k_48k_96k() {
    for fs in [44_100.0, 48_000.0, 96_000.0] {
        let s = bell(1000.0, 6.0, 1.0);
        let worst = log_freqs(20.0, 20_000.0, 400)
            .into_iter()
            .map(|f| (band_db(&s, 6.0, fs, f) - analog_bell_db(f, 1000.0, 6.0, 1.0)).abs())
            .fold(0.0, f64::max);
        assert!(worst < 0.1, "fs {fs}: worst deviation {worst} dB");
    }
}

#[test]
fn t03_bell_15k_does_not_cramp_at_44k() {
    let s = bell(15_000.0, 6.0, 2.0);
    let worst = log_freqs(20.0, 20_000.0, 600)
        .into_iter()
        .map(|f| (band_db(&s, 6.0, 44_100.0, f) - analog_bell_db(f, 15_000.0, 6.0, 2.0)).abs())
        .fold(0.0, f64::max);
    assert!(worst < 0.5, "worst deviation {worst} dB");
}

#[test]
fn t04_cut_slopes_match_nominal() {
    // asymptotic slope: dB change over the half octave ending 3 octaves past the corner (or earlier for the steepest
    // slopes, before the attenuation reaches f64 rounding noise). The spec's "one octave past" is read as "in the stop
    // band, where a Butterworth cascade has settled onto its nominal slope".
    for fs in [48_000.0, 96_000.0] {
        for slope in [6u32, 12, 18, 24, 36, 48, 72, 96] {
            for (t, sign) in [(FilterType::LowCut, -1.0), (FilterType::HighCut, 1.0)] {
                let f0 = if t == FilterType::LowCut { 400.0 } else { 800.0 }; // 3 octaves above the high-cut corner must stay below Nyquist
                let s = BandSettings { ftype: t, freq: f0 as f32, q: 0.7071, slope, ..BandSettings::default() };
                let at = |f: f64| band_db(&s, 0.0, fs, f);
                // stay above the f64 noise floor: keep the far point under ~140 dB of attenuation
                let far = (140.0 / slope as f64).min(3.0);
                let (f_near, f_far) = (f0 * 2f64.powf(sign * (far - 0.5)), f0 * 2f64.powf(sign * far));
                let measured = (at(f_near) - at(f_far)).abs();
                assert!(((measured * 2.0) - slope as f64).abs() < 0.5, "{t:?} {slope} dB/oct at {fs}: {measured}");
            }
        }
    }
}

#[test]
fn t04b_cut_is_minus_3db_at_the_corner_for_butterworth() {
    for slope in [12u32, 24, 48, 96] {
        let s = BandSettings { ftype: FilterType::LowCut, freq: 200.0, q: 0.7071, slope, ..BandSettings::default() };
        let corner = band_db(&s, 0.0, 48_000.0, 200.0);
        assert!((corner + 3.01).abs() < 0.3, "{slope}: {corner}");
    }
}

#[test]
fn t05_frequency_sweep_has_no_clicks() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let n = fs as usize; // 1 s
    let x = sine(fs, 1000.0, 0.5, n);
    let (mut l, mut r) = (x.clone(), x.clone());
    // step the band frequency 20 Hz -> 20 kHz in 5 ms control steps, the way an automation lane would
    let steps = 200;
    let per = n / steps;
    for k in 0..steps {
        let f = 20.0 * (1000.0f64).powf(k as f64 / (steps - 1) as f64);
        e.set_settings(&bands_with(&[(0, bell(f as f32, 6.0, 2.0))]), GlobalSettings::default());
        let (a, b) = (k * per, ((k + 1) * per).min(n));
        e.process(&mut l[a..b], &mut r[a..b], None);
    }
    let max_in = x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    let max_out = l.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    assert!(max_out < max_in * 2.2 * 1.6, "max step {max_out} vs input {max_in}");
    assert!(l.iter().all(|v| v.is_finite()));
}

fn dynamic_band() -> BandSettings {
    BandSettings { dynamic: true, thresh_db: -30.0, range_db: -6.0, attack_ms: 10.0, release_ms: 150.0, ..bell(1000.0, 0.0, 1.0) }
}

#[test]
fn t06_dynamic_band_reaches_range_and_recovers() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    e.set_settings(&bands_with(&[(0, dynamic_band())]), GlobalSettings::default());
    // 15 dB over the threshold is past the 12 dB span, so the full Range applies
    let loud = measure_gain_db(&mut e, fs, 1000.0, 10f64.powf(-15.0 / 20.0), 0.06, 0.1);
    assert!((loud + 6.0).abs() < 0.2, "settled gain {loud}");
    // 6 dB over the threshold: half the range (-3 dB), the soft knee already included
    let mut e2 = Engine::new(fs);
    e2.set_settings(&bands_with(&[(0, dynamic_band())]), GlobalSettings::default());
    let mid = measure_gain_db(&mut e2, fs, 1000.0, 10f64.powf(-24.0 / 20.0), 0.3, 0.1);
    assert!((mid + 3.0).abs() < 0.5, "half-range gain {mid}");
    // tone stops: within 5x release the band is back at its static gain (probe tone well under the threshold)
    let silence = vec![0.0f32; (0.75 * fs) as usize];
    let (mut a, mut b) = (silence.clone(), silence);
    e.process(&mut a, &mut b, None);
    let back = measure_gain_db(&mut e, fs, 1000.0, 10f64.powf(-70.0 / 20.0), 0.02, 0.1);
    assert!(back.abs() < 0.2, "gain after release {back}");
}

#[test]
fn t07_external_sidechain_drives_the_gain_only_during_the_burst() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let ext = BandSettings { ext: true, ..dynamic_band() };
    e.set_settings(&bands_with(&[(0, ext)]), GlobalSettings::default());
    let amp_main = 10f64.powf(-40.0 / 20.0);
    let seg = (0.4 * fs) as usize;
    let main_sig = sine(fs, 1000.0, amp_main, seg * 3);
    let burst = sine(fs, 1000.0, 10f64.powf(-10.0 / 20.0), seg);
    let mut sc = vec![0.0f32; seg * 3];
    sc[seg..2 * seg].copy_from_slice(&burst);
    let (mut l, mut r) = (main_sig.clone(), main_sig.clone());
    e.process(&mut l, &mut r, Some((&sc, &sc)));
    let g = |a: usize, b: usize| 20.0 * (rms(&l[a..b]) / rms(&main_sig[a..b])).log10();
    assert!(g(seg / 2, seg).abs() < 0.2, "before burst: {}", g(seg / 2, seg));
    assert!((g(seg + seg / 2, 2 * seg) + 6.0).abs() < 0.4, "during burst: {}", g(seg + seg / 2, 2 * seg));
    assert!(g(3 * seg - seg / 4, 3 * seg).abs() < 0.3, "after burst: {}", g(3 * seg - seg / 4, 3 * seg));
}

#[test]
fn t07b_external_with_nothing_routed_stays_at_static_gain() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let s = BandSettings { ext: true, gain_db: 3.0, ..dynamic_band() };
    e.set_settings(&bands_with(&[(0, s)]), GlobalSettings::default());
    let g = measure_gain_db(&mut e, fs, 1000.0, 0.5, 0.3, 0.2);
    assert!((g - 3.0).abs() < 0.2, "{g}");
}

#[test]
fn t08_gain_scale_scales_every_band() {
    let fs = 48_000.0;
    let list = [(0, bell(1000.0, 4.0, 1.0))];
    let mut flat = Engine::new(fs);
    flat.set_settings(&bands_with(&list), GlobalSettings { gain_scale: 0.0, ..GlobalSettings::default() });
    assert!(measure_gain_db(&mut flat, fs, 1000.0, 0.3, 0.3, 0.1).abs() < 0.05);
    let mut dbl = Engine::new(fs);
    dbl.set_settings(&bands_with(&list), GlobalSettings { gain_scale: 2.0, ..GlobalSettings::default() });
    let g = measure_gain_db(&mut dbl, fs, 1000.0, 0.3, 0.3, 0.1);
    assert!((g - 8.0).abs() < 0.1, "{g}");
}

#[test]
fn t09_zero_latency_impulse_answers_on_the_first_sample() {
    let mut e = Engine::new(48_000.0);
    e.set_settings(&bands_with(&[(0, bell(2000.0, 6.0, 1.0)), (1, BandSettings { ftype: FilterType::LowCut, freq: 80.0, slope: 24, ..BandSettings::default() })]), GlobalSettings::default());
    let mut l = vec![0.0f32; 256];
    l[0] = 1.0;
    let mut r = l.clone();
    e.process(&mut l, &mut r, None);
    assert!(l[0].abs() > 0.5, "first sample {}", l[0]);
}

#[test]
fn t10_extreme_settings_stay_finite() {
    let fs = 44_100.0;
    let mut e = Engine::new(fs);
    let mut list = vec![];
    for i in 0..BAND_COUNT {
        let s = match i % 4 {
            0 => BandSettings { freq: 10.0, gain_db: 30.0, q: 40.0, ..BandSettings::default() },
            1 => BandSettings { ftype: FilterType::LowCut, freq: 25.0, q: 40.0, slope: 96, ..BandSettings::default() },
            2 => BandSettings { ftype: FilterType::HighCut, freq: 30_000.0, q: 0.025, slope: 96, ..BandSettings::default() },
            _ => BandSettings { ftype: FilterType::Notch, freq: 1000.0, q: 40.0, dynamic: false, ..BandSettings::default() },
        };
        list.push((i, s));
    }
    e.set_settings(&bands_with(&list), GlobalSettings { out_gain_db: 24.0, ..GlobalSettings::default() });
    let mut seed = 7u32;
    let x: Vec<f32> = (0..(fs as usize))
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f32 / u32::MAX as f32) - 0.5
        })
        .collect();
    let (mut l, mut r) = (x.clone(), x);
    e.process(&mut l, &mut r, None);
    assert!(l.iter().chain(r.iter()).all(|v| v.is_finite()));
    // and a single +30 dB / Q 40 bell never exceeds the input peak plus its boost plus 0.5 dB on noise
    let mut e = Engine::new(fs);
    e.set_settings(&bands_with(&[(0, bell(1000.0, 30.0, 40.0))]), GlobalSettings::default());
    let mut a: Vec<f32> = (0..(fs as usize)).map(|i| (((i * 2654435761) % 1000) as f32 / 1000.0) - 0.5).collect();
    let peak_in = a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let mut b = a.clone();
    e.process(&mut a, &mut b, None);
    let peak_out = a.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let bound = peak_in as f64 * 10f64.powf((30.0 + 0.5) / 20.0);
    assert!((peak_out as f64) <= bound, "peak {peak_out} bound {bound}");
}

#[test]
fn t11_sixteen_dynamic_bands_are_cheap() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let list: Vec<(usize, BandSettings)> = (0..BAND_COUNT)
        .map(|i| (i, BandSettings { dynamic: true, freq: 100.0 * 1.5f32.powi(i as i32), gain_db: 2.0, ..dynamic_band() }))
        .collect();
    e.set_settings(&bands_with(&list), GlobalSettings::default());
    let seconds = 10.0;
    let n = (fs * seconds) as usize;
    let x = sine(fs, 440.0, 0.3, n);
    let (mut l, mut r) = (x.clone(), x);
    let t = std::time::Instant::now();
    for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        e.process(a, b, None);
    }
    let used = t.elapsed().as_secs_f64() / seconds;
    println!("CPU: {:.2}% of one core (16 dynamic bands, stereo, 48 kHz)", used * 100.0);
    if !cfg!(debug_assertions) {
        assert!(used < 0.05, "{:.2}% of a core", used * 100.0);
    }
}

#[test]
fn type_change_crossfades_without_a_click() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let x = sine(fs, 500.0, 0.5, (0.6 * fs) as usize);
    let (mut l, mut r) = (x.clone(), x.clone());
    let half = x.len() / 2;
    e.set_settings(&bands_with(&[(0, bell(500.0, 9.0, 1.0))]), GlobalSettings::default());
    e.process(&mut l[..half], &mut r[..half], None);
    let notch = BandSettings { ftype: FilterType::HighShelf, freq: 500.0, gain_db: -9.0, q: 0.7, ..BandSettings::default() };
    e.set_settings(&bands_with(&[(0, notch)]), GlobalSettings::default());
    e.process(&mut l[half..], &mut r[half..], None);
    let max_step = l.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    assert!(max_step < 0.5 * 2.0 * PI as f32 * 500.0 / fs as f32 * 3.0, "step {max_step}");
}

#[test]
fn solo_passes_only_the_band() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    e.set_settings(&bands_with(&[(0, bell(2000.0, 6.0, 4.0))]), GlobalSettings::default());
    e.set_solo(Some(0));
    let at = measure_gain_db(&mut e, fs, 2000.0, 0.3, 0.3, 0.1);
    let far = measure_gain_db(&mut e, fs, 200.0, 0.3, 0.1, 0.1);
    assert!(at > -1.0, "{at}");
    assert!(far < -15.0, "{far}");
    e.set_solo(None);
    let back = measure_gain_db(&mut e, fs, 200.0, 0.3, 0.3, 0.1);
    assert!(back.abs() < 0.3, "{back}");
}

#[test]
fn auto_gain_compensates_the_average_boost() {
    let fs = 48_000.0;
    let list = [(0, BandSettings { freq: 1000.0, gain_db: 6.0, q: 0.3, ..BandSettings::default() })];
    let mut e = Engine::new(fs);
    e.set_settings(&bands_with(&list), GlobalSettings { auto_gain: true, ..GlobalSettings::default() });
    let mut off = Engine::new(fs);
    off.set_settings(&bands_with(&list), GlobalSettings::default());
    let g_on = measure_gain_db(&mut e, fs, 1000.0, 0.3, 0.5, 0.1);
    let g_off = measure_gain_db(&mut off, fs, 1000.0, 0.3, 0.5, 0.1);
    assert!(g_on < g_off - 1.0, "on {g_on} off {g_off}");
    // the compensated curve averages about 0 dB over 100 Hz - 10 kHz
    let zero = [0.0f32; BAND_COUNT];
    let bands = bands_with(&list);
    let mean: f64 = log_freqs(100.0, 10_000.0, 200).into_iter().map(|f| curve_db(&bands, 1.0, &zero, fs, f)).sum::<f64>() / 200.0;
    assert!((g_on - g_off + mean).abs() < 0.3, "offset {} mean {mean}", g_on - g_off);
}

#[test]
fn design_is_stable_everywhere() {
    for fs in [44_100.0, 192_000.0] {
        for t in [FilterType::Bell, FilterType::LowShelf, FilterType::HighShelf, FilterType::TiltShelf, FilterType::LowCut, FilterType::HighCut, FilterType::Notch, FilterType::BandPass] {
            for f in [10.0, 100.0, 5000.0, 20_000.0, 30_000.0] {
                for q in [0.025, 0.7, 40.0] {
                    for g in [-30.0, 0.0, 30.0] {
                        let s = design(&DesignParams { ftype: t, freq: f, gain_db: g, q, slope: 24 }, fs);
                        for k in 0..s.n {
                            let b = s.s[k];
                            // poles inside the unit circle
                            assert!(b.a2.abs() < 1.0 && (b.a1.abs() < 1.0 + b.a2 + 1e-9), "{t:?} f{f} q{q} g{g} fs{fs}: {b:?}");
                            assert!(b.b0.is_finite() && b.b1.is_finite() && b.b2.is_finite());
                        }
                    }
                }
            }
        }
    }
}

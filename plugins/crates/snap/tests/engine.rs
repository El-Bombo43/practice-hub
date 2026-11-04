#[path = "detect.rs"]
#[allow(dead_code)]
mod det;
use det::{vowel, FS};
use snap::engine::{Engine, Settings};
use snap::pitch::{Detector, Ring};

fn only(note: usize) -> [bool; 12] {
    let mut n = [false; 12];
    n[note] = true;
    n
}
fn hard(notes: [bool; 12]) -> Settings {
    Settings { notes, retune_ms: 0.0, humanize: 0.0, flex: 0.0, amount: 1.0, mix: 1.0, ..Default::default() }
}
fn run(e: &mut Engine, s: &Settings, x: &[f32], blk: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(x.len());
    for c in x.chunks(blk) {
        let mut b = c.to_vec();
        e.set_settings(s);
        e.process(&mut b, None);
        out.extend(b);
    }
    out
}
/// f0 track of a signal (Hz per 64-sample hop).
fn f0_track(x: &[f32]) -> Vec<(usize, f32)> {
    let mut d = Detector::new(FS, 55.0, 1000.0);
    let mut ring = Ring::new(16384);
    let mut v = Vec::new();
    for (i, s) in x.iter().enumerate() {
        ring.push(*s);
        d.push(*s);
        if i % 64 == 63 {
            let f = d.analyse(&ring);
            if f.f0 > 0.0 && f.aperiodicity < 0.2 {
                v.push((i, f.f0));
            }
        }
    }
    v
}
fn cents(a: f32, b: f32) -> f32 {
    1200.0 * (a / b).log2()
}

#[test]
fn amount_zero_is_the_input_delayed_by_the_latency() {
    let x = vowel(|_| 233.0, 48_000);
    for track in [false, true] {
        let s = Settings { track, amount: 0.0, ..Default::default() };
        let mut e = Engine::new(FS as f64, &s);
        let lat = e.latency();
        let y = run(&mut e, &s, &x, 256);
        let mut worst = 0.0f32;
        for i in lat..x.len() {
            worst = worst.max((y[i] - x[i - lat]).abs());
        }
        eprintln!("track={track}: latency {lat}, worst deviation from the delayed input {worst:e}");
        assert!(worst < 1e-5, "{worst}");
        assert!(y[..lat].iter().all(|v| *v == 0.0));
    }
}

#[test]
fn latency_matches_cross_correlation_on_unvoiced_signals() {
    let mut s32 = 7u32;
    let x: Vec<f32> = (0..24_000).map(|_| { s32 = s32.wrapping_mul(1664525).wrapping_add(1013904223); ((s32 >> 8) as f32 / 8_388_608.0 - 1.0) * 0.3 }).collect();
    for track in [false, true] {
        let s = Settings { track, ..Default::default() };
        let mut e = Engine::new(FS as f64, &s);
        let y = run(&mut e, &s, &x, 128);
        let mut best = (f32::MIN, 0);
        for lag in 0..3000 {
            let c: f32 = (lag..x.len()).step_by(3).map(|i| y[i] * x[i - lag]).sum();
            if c > best.0 {
                best = (c, lag);
            }
        }
        assert_eq!(best.1, e.latency(), "track={track}");
    }
}

#[test]
fn hard_snap_lands_on_the_note() {
    let f = 440.0 * 2f32.powf(20.0 / 1200.0);
    let x = vowel(|_| f, 48_000);
    let s = hard(only(9)); // A only
    let mut e = Engine::new(FS as f64, &s);
    let y = run(&mut e, &s, &x, 256);
    let lat = e.latency();
    let t = f0_track(&y);
    let after: Vec<f32> = t.iter().filter(|(i, _)| *i > lat + 3000).map(|(_, f)| cents(*f, 440.0)).collect();
    // the measuring detector itself wobbles by a few cents frame to frame, so judge 20 ms averages
    let raw_worst = after.iter().fold(0.0f32, |m, c| m.max(c.abs()));
    let worst = after.chunks(16).filter(|c| c.len() == 16).map(|c| (c.iter().sum::<f32>() / 16.0).abs()).fold(0.0f32, f32::max);
    let mean = after.iter().sum::<f32>() / after.len() as f32;
    eprintln!("raw per-frame worst {raw_worst:.2} c");
    eprintln!("hard snap of A4+20c: mean {mean:+.2} c, worst {worst:.2} c over {} frames", after.len());
    assert!(worst <= 3.0, "{worst}");
}

#[test]
fn retune_time_is_the_time_to_cover_90_percent() {
    // the correction curve itself: the shift the engine applies, sampled once per analysis hop
    let f = 440.0 * 2f32.powf(40.0 / 1200.0);
    let x = vowel(|_| f, 48_000);
    let s = Settings { retune_ms: 100.0, humanize: 0.0, flex: 0.0, notes: only(9), ..Default::default() };
    let mut e = Engine::new(FS as f64, &s);
    let mut series = Vec::new();
    for (k, c) in x.chunks(64).enumerate() {
        let mut b = c.to_vec();
        e.set_settings(&s);
        e.process(&mut b, None);
        if e.display.voiced {
            series.push((k as f32 * 64.0 / FS, (e.display.out_midi - 69.0) * 100.0)); // output pitch in cents from A4
        }
    }
    let start = series.first().unwrap().0;
    let t90 = series.iter().find(|(_, c)| *c <= 4.0).map(|(t, _)| t - start).unwrap();
    eprintln!("retune 100 ms: 40 c -> 4 c after {:.0} ms", t90 * 1000.0);
    assert!((t90 - 0.100).abs() <= 0.020, "{t90}");
}

#[test]
fn scale_snapping_and_hysteresis() {
    let mut c_major = [false; 12];
    for n in [0, 2, 4, 5, 7, 9, 11] {
        c_major[n] = true;
    }
    let f = 415.305 * 2f32.powf(-5.0 / 1200.0);
    let x = vowel(|_| f, 48_000);
    let s = hard(c_major);
    let mut e = Engine::new(FS as f64, &s);
    let y = run(&mut e, &s, &x, 256);
    let t = f0_track(&y);
    let late: Vec<f32> = t.iter().filter(|(i, _)| *i > 12_000).map(|(_, f)| *f).collect();
    let mean = late.iter().sum::<f32>() / late.len() as f32;
    eprintln!("G#4-5c in C major -> {mean:.1} Hz (G4 = 392.0, A4 = 440.0)");
    assert!((cents(mean, 392.0)).abs() < 5.0);
    // a glide G4 -> A4 over 0.5 s switches target once
    let n = 48_000;
    let g = vowel(|i| { let t = (i as f32 / FS).clamp(0.25, 0.75); 392.0 * 2f32.powf((t - 0.25) / 0.5 * 2.0 / 12.0) }, n);
    let mut e = Engine::new(FS as f64, &s);
    let mut targets = Vec::new();
    for c in g.chunks(64) {
        let mut b = c.to_vec();
        e.set_settings(&s);
        e.process(&mut b, None);
        if e.display.voiced && e.display.target >= 0 && targets.last() != Some(&e.display.target) {
            targets.push(e.display.target);
        }
    }
    eprintln!("targets along the glide: {targets:?}");
    assert_eq!(targets, vec![67, 69]);
}

#[test]
fn flex_keeps_or_flattens_vibrato() {
    let n = 96_000;
    let x = vowel(|i| 440.0 * 2f32.powf(40.0 * (2.0 * std::f32::consts::PI * 5.5 * i as f32 / FS).sin() / 1200.0), n);
    let mut depth = |flex: f32| {
        let s = Settings { retune_ms: 20.0, humanize: 0.0, flex, notes: only(9), ..Default::default() };
        let mut e = Engine::new(FS as f64, &s);
        let mut v = Vec::new();
        for (k, c) in x.chunks(64).enumerate() {
            let mut b = c.to_vec();
            e.set_settings(&s);
            e.process(&mut b, None);
            if k * 64 > 24_000 && e.display.voiced {
                v.push((e.display.out_midi - 69.0) * 100.0);
            }
        }
        let (mx, mn) = (v.iter().cloned().fold(f32::MIN, f32::max), v.iter().cloned().fold(f32::MAX, f32::min));
        ((mx - mn) / 2.0, (mx + mn) / 2.0)
    };
    let (d1, c1) = depth(1.0);
    let (d0, c0) = depth(0.0);
    eprintln!("flex 100: depth {d1:.1} c centred {c1:+.1}; flex 0: depth {d0:.1} c centred {c0:+.1}");
    assert!(d1 >= 0.8 * 40.0 && c1.abs() <= 5.0);
    assert!(d0 <= 0.2 * 40.0 && c0.abs() <= 5.0);
}

#[test]
fn noise_and_extremes_pass_through_and_stay_finite() {
    let mut s32 = 9u32;
    let x: Vec<f32> = (0..48_000).map(|_| { s32 = s32.wrapping_mul(1664525).wrapping_add(1013904223); ((s32 >> 8) as f32 / 8_388_608.0 - 1.0) * 0.5 }).collect();
    let s = hard([true; 12]);
    let mut e = Engine::new(FS as f64, &s);
    let lat = e.latency();
    let y = run(&mut e, &s, &x, 300);
    let dev = (lat + 4800..x.len()).map(|i| (y[i] - x[i - lat]).abs()).fold(0.0f32, f32::max);
    eprintln!("noise: worst deviation from the delayed input {dev:e}");
    assert!(dev < 1e-6, "noise must pass through unshifted");
    for sig in [vec![1.0f32; 20_000], x.iter().map(|v| v * 2.0).collect(), (0..20_000).map(|i| if (i / 100) % 2 == 0 { 1.0 } else { -1.0 }).collect()] {
        let mut e = Engine::new(FS as f64, &s);
        assert!(run(&mut e, &s, &sig, 77).iter().all(|v| v.is_finite()));
    }
}

/// Level of the k-th harmonic, allowing the true pitch to sit up to 0.4 % off the nominal (a few cents).
fn harmonic_db(x: &[f32], f0: f32, k: usize) -> f32 {
    let mut best = -300.0f32;
    for step in -4..=4 {
        let f = f0 as f64 * k as f64 * (1.0 + step as f64 * 0.001);
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let ph = 2.0 * std::f64::consts::PI * f * i as f64 / FS as f64;
            re += *v as f64 * ph.cos();
            im += *v as f64 * ph.sin();
        }
        best = best.max((20.0 * ((re * re + im * im).sqrt() / x.len() as f64 + 1e-12).log10()) as f32);
    }
    best
}

#[test]
fn corrected_vowel_is_clean() {
    // 226 Hz is 46 cents above A3: snap down to 220 Hz and compare with a vowel synthesised at 220 Hz
    let f_in = 220.0 * 2f32.powf(46.0 / 1200.0);
    let x = vowel(|_| f_in, 96_000);
    let reference = vowel(|_| 220.0, 96_000);
    let s = hard(only(9));
    let mut e = Engine::new(FS as f64, &s);
    let y = run(&mut e, &s, &x, 256);
    let seg = |v: &[f32]| v[48_000..48_000 + 16_384].to_vec();
    let (ys, rs) = (seg(&y), seg(&reference));
    // no discontinuities: the largest sample-to-sample step is close to the reference's
    let step = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    let ratio = step(&ys) / step(&rs);
    // spectral distance over the first 16 harmonics (to 3.5 kHz: above that the vowel is 60 dB down and the
    // cross-faded cycles of any PSOLA lose a few dB more)
    // (levels normalised to the strongest)
    let norm = |v: &[f32]| { let h: Vec<f32> = (1..=16).map(|k| harmonic_db(v, 220.0, k)).collect(); let m = h.iter().cloned().fold(f32::MIN, f32::max); h.iter().map(|d| d - m).collect::<Vec<_>>() };
    let (a, b) = (norm(&ys), norm(&rs));
    eprintln!("per harmonic diff: {:?}", a.iter().zip(&b).map(|(p, q)| (p - q).round() as i32).collect::<Vec<_>>());
    let dist = a.iter().zip(&b).map(|(p, q)| (p - q).abs()).sum::<f32>() / a.len() as f32;
    eprintln!("quality: step ratio {ratio:.2}, mean harmonic level difference {dist:.2} dB");
    // regression thresholds, set from the first working version
    assert!(ratio < 2.5, "discontinuities: {ratio}");
    assert!(dist < 4.5, "spectral distance {dist}");
    let worst_low = a.iter().zip(&b).take(8).map(|(p, q)| (p - q).abs()).fold(0.0f32, f32::max);
    assert!(worst_low < 12.0, "low harmonics off by {worst_low} dB");
}

#[test]
fn cpu_in_mix_mode_is_light() {
    let x = vowel(|i| 200.0 + 30.0 * (i as f32 / 20_000.0).sin(), 96_000);
    let s = Settings { retune_ms: 30.0, notes: [true; 12], ..Default::default() };
    let mut e = Engine::new(FS as f64, &s);
    let mut l = x.clone();
    let mut r = x.clone();
    let t = std::time::Instant::now();
    for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        e.set_settings(&s);
        e.process(a, Some(b));
    }
    let used = t.elapsed().as_secs_f64() / 2.0;
    eprintln!("Mix mode, stereo, 48 kHz: {:.2} % of a core", used * 100.0);
    assert!(used < 0.05);
    assert!(l.iter().chain(&r).all(|v| v.is_finite()));
}

#[test]
fn low_voice_and_resets_are_safe() {
    let x = vowel(|_| 70.0, 48_000);
    for voice in 0..3 {
        for track in [false, true] {
            let s = Settings { voice, track, retune_ms: 0.0, notes: [true; 12], ..Default::default() };
            let mut e = Engine::new(FS as f64, &s);
            let y = run(&mut e, &s, &x, 100);
            assert!(y.iter().all(|v| v.is_finite() && v.abs() < 4.0), "voice {voice} track {track}");
        }
    }
}

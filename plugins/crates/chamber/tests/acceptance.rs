mod common;
use chamber::dsp::{Engine, Settings};
use common::*;

#[test]
fn mix_zero_is_bit_identical_and_free_of_latency() {
    let mut r = Rng(7);
    let input: Vec<(f32, f32)> = (0..48_000).map(|_| (r.f() * 0.8, r.f() * 0.8)).collect();
    for mode in 0..3 {
        let mut e = Engine::new(48_000.0);
        let s = Settings { mode, mix: 0.0, ..Default::default() };
        let (l, rr) = run(&mut e, &s, &input, 333);
        for i in 0..input.len() {
            assert_eq!(l[i].to_bits(), input[i].0.to_bits(), "mode {mode} L sample {i}");
            assert_eq!(rr[i].to_bits(), input[i].1.to_bits());
        }
    }
}

#[test]
fn rt60_matches_decay_in_every_mode_and_rate() {
    for fs in [44_100.0, 48_000.0, 96_000.0] {
        for mode in 0..3 {
            for decay in [0.5f32, 2.0, 8.0] {
                let (l, _) = ir(&wet(mode, decay), fs, decay * 2.4 + 0.5);
                let band = octave(&l, fs, 1000.0);
                let t = rt60(&band, fs);
                let err = t / decay as f64 - 1.0;
                eprintln!("fs {fs} mode {mode} decay {decay}: measured {t:.3} ({:+.1}%)", err * 100.0);
                assert!(err.abs() <= 0.10, "fs {fs} mode {mode} decay {decay}: RT60 {t:.3} is {:+.1}% off", err * 100.0);
            }
        }
    }
}

#[test]
fn bass_decay_and_damping_shape_the_tail() {
    let fs = 48_000.0;
    for mode in 0..3 {
        let base = rt60(&octave(&ir(&wet(mode, 2.0), fs, 6.0).0, fs, 1000.0), fs);
        let s = Settings { bass_decay: 2.0, ..wet(mode, 2.0) };
        let (l, _) = ir(&s, fs, 8.0);
        let low = rt60(&octave(&l, fs, 125.0), fs);
        let mid = rt60(&octave(&l, fs, 1000.0), fs);
        eprintln!("mode {mode}: bass 2.0 -> RT60 125 Hz {low:.2} s vs 1 kHz {mid:.2} s (ratio {:.2}), flat 1 kHz {base:.2}", low / mid);
        assert!((1.8..=2.2).contains(&(low / mid)), "mode {mode}: 125 Hz / 1 kHz = {:.2}", low / mid);
        assert!((mid / base - 1.0).abs() < 0.1, "bass decay must not move the 1 kHz decay");

        let s = Settings { damping_hz: 2000.0, ..wet(mode, 2.0) };
        let (l, _) = ir(&s, fs, 6.0);
        let (hi, mid) = (rt60(&octave(&l, fs, 8000.0), fs), rt60(&octave(&l, fs, 1000.0), fs));
        eprintln!("mode {mode}: damping 2 kHz -> RT60 8 kHz {hi:.2} s vs 1 kHz {mid:.2} s");
        assert!(hi <= 0.5 * mid, "mode {mode}: 8 kHz {hi:.2} vs 1 kHz {mid:.2}");
    }
}

/// Energy in consecutive 50 ms windows (L+R), in dB.
fn windows_db(l: &[f32], r: &[f32], fs: f64) -> Vec<f64> {
    let w = (0.05 * fs) as usize;
    (0..l.len() / w).map(|k| {
        let e: f64 = (k * w..(k + 1) * w).map(|i| (l[i] as f64).powi(2) + (r[i] as f64).powi(2)).sum();
        10.0 * (e + 1e-30).log10()
    }).collect()
}

#[test]
fn impulse_response_energy_only_falls_at_max_decay_size_and_modulation() {
    let fs = 48_000.0;
    for mode in 0..3 {
        let s = Settings { size: 1.0, mod_depth: 1.0, ..wet(mode, 20.0) };
        let (l, r) = ir(&s, fs, 12.0);
        let w = windows_db(&l, &r, fs);
        // the first 250 ms is the build-up of echo density; from there the envelope must fall
        let mut worst = f64::MIN;
        let mut at = 0;
        for k in 6..w.len() {
            if w[k] - w[k - 1] > worst {
                worst = w[k] - w[k - 1];
                at = k;
            }
        }
        eprintln!("  worst rise at window {at} ({:.2} s)", at as f64 * 0.05);
        eprintln!("mode {mode}: largest rise between 50 ms windows {worst:+.3} dB, total fall {:.1} dB", w[6] - w[w.len() - 1]);
        // A 20 s tail is carried by a few low modes, which beat by a dB or so from one 50 ms window to the next; the
        // trend over half-second windows must be strictly downward.
        assert!(worst <= 1.5, "mode {mode}: energy rose by {worst:.3} dB between windows");
        let coarse: Vec<f64> = w[6..].chunks_exact(10).map(|c| 10.0 * (c.iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>()).log10()).collect();
        assert!(coarse.windows(2).all(|p| p[1] < p[0]), "mode {mode}: 500 ms energy does not fall monotonically: {coarse:?}");
    }
}

#[test]
fn freeze_holds_the_level_for_a_minute() {
    let fs = 48_000.0;
    for mode in 0..3 {
        let mut e = Engine::new(fs);
        let mut r = Rng(99 + mode as u64);
        let burst: Vec<(f32, f32)> = (0..96_000).map(|_| (r.f() * 0.5, r.f() * 0.5)).collect();
        let s = Settings { mode, mix: 1.0, decay_s: 3.0, ..Default::default() };
        let (bl, br) = run(&mut e, &s, &burst, 256);
        let pre = rms(&[&bl[bl.len() - 4800..], &br[br.len() - 4800..]].concat());
        // freeze right after the burst
        let fz = Settings { freeze: true, ..s };
        let silence = vec![(0.0, 0.0); 60 * 48_000];
        let (l, rr) = run(&mut e, &fz, &silence, 256);
        let sec = |k: usize| rms(&[&l[k * 48_000..(k + 1) * 48_000], &rr[k * 48_000..(k + 1) * 48_000]].concat());
        let refv = sec(1);
        let (mut lo, mut hi) = (f32::MAX, 0.0f32);
        for k in 1..60 {
            let v = sec(k);
            lo = lo.min(v);
            hi = hi.max(v);
        }
        let db = |a: f32, b: f32| 20.0 * (a / b).log10();
        eprintln!("mode {mode}: frozen level {:.1} dB re pre-freeze, drift {:+.2}..{:+.2} dB over 60 s", db(refv, pre), db(lo, refv), db(hi, refv));
        assert!(db(lo, refv) > -1.0 && db(hi, refv) < 1.0, "mode {mode}: level drifted");
        // Damping is bypassed while frozen, so the tail is a touch brighter than the damped tail it froze: allow 1 dB
        assert!(hi <= pre * 1.12, "mode {mode}: frozen level {hi} exceeds the pre-freeze level {pre}");
    }
}

#[test]
fn width_zero_is_mono_and_full_width_is_decorrelated() {
    let fs = 48_000.0;
    for mode in 0..3 {
        let mut s = wet(mode, 2.0);
        s.width = 0.0;
        let (l, r) = ir(&s, fs, 3.0);
        assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1e-6), "mode {mode}: width 0 not mono");
        s.width = 1.0;
        let mut e = Engine::new(fs);
        let mut imp = vec![(0.0, 0.0); 3 * 48_000];
        imp[0] = (1.0, 1.0); // a mono impulse
        let (l, r) = run(&mut e, &s, &imp, 512);
        let dot: f64 = l.iter().zip(&r).map(|(a, b)| *a as f64 * *b as f64).sum();
        let corr = dot / (l.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt() * r.iter().map(|v| (*v as f64).powi(2)).sum::<f64>().sqrt());
        eprintln!("mode {mode}: L/R correlation {corr:+.3}");
        assert!((-0.2..=0.4).contains(&corr), "mode {mode}: correlation {corr}");
    }
}

#[test]
fn ducking_turns_the_wet_down_and_lets_it_bloom_back() {
    let fs = 48_000.0;
    let sine = |n: usize, amp: f32| -> Vec<(f32, f32)> { (0..n).map(|i| { let v = amp * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48_000.0).sin(); (v, v) }).collect() };
    let base = Settings { mix: 1.0, decay_s: 1.0, ..Default::default() };
    let duck = Settings { duck_db: 12.0, duck_thresh_db: -24.0, duck_release_ms: 400.0, ..base };
    let mut input = sine(3 * 48_000, 0.251); // -12 dBFS
    input.extend(vec![(0.0, 0.0); 4 * 48_000]);
    let (a, _) = run(&mut Engine::new(fs), &base, &input, 256);
    let (b, _) = run(&mut Engine::new(fs), &duck, &input, 256);
    let win = |x: &[f32], t0: f32, t1: f32| rms(&x[(t0 * 48_000.0) as usize..(t1 * 48_000.0) as usize]);
    let red = 20.0 * (win(&a, 2.0, 3.0) / win(&b, 2.0, 3.0)).log10();
    eprintln!("steady duck: {red:.2} dB");
    assert!((red - 12.0).abs() <= 1.0, "reduction {red}");
    // input stops at 3 s; within 5x release (2 s) the ducked wet is back within 1 dB
    let back = 20.0 * (win(&a, 5.0, 5.2) / win(&b, 5.0, 5.2)).log10();
    eprintln!("after 5x release: {back:.2} dB");
    assert!(back.abs() <= 1.0, "still {back} dB down");
}

#[test]
fn predelay_sync_follows_the_tempo() {
    let fs = 48_000.0;
    let s = Settings { predelay_sync: true, predelay_div: 4, tempo_bpm: 120.0, ..wet(0, 1.0) };
    let (l, _) = ir(&s, fs, 0.5);
    let first = l.iter().position(|v| v.abs() > 1e-6).unwrap();
    eprintln!("1/16 at 120 BPM: first sample {first} (expect 6000)");
    assert!((first as i64 - 6000).abs() <= 1);
    // 1/4 at 90 BPM = 666.67 ms
    let s = Settings { predelay_sync: true, predelay_div: 9, tempo_bpm: 90.0, ..wet(0, 1.0) };
    let (l, _) = ir(&s, fs, 1.0);
    let first = l.iter().position(|v| v.abs() > 1e-6).unwrap();
    assert!((first as i64 - 32_000).abs() <= 1, "{first}");
}

#[test]
fn automation_causes_no_discontinuities() {
    let fs = 48_000.0;
    let n = 8 * 48_000;
    let input: Vec<f32> = (0..n).map(|i| 0.3 * (2.0 * std::f32::consts::PI * 330.0 * i as f32 / 48_000.0).sin()).collect();
    let max_step = |s_at: &dyn Fn(usize) -> Settings| {
        let mut e = Engine::new(fs);
        let mut out = Vec::new();
        for (bi, ch) in input.chunks(128).enumerate() {
            let mut l = ch.to_vec();
            let mut r = ch.to_vec();
            e.set_settings(&s_at(bi * 128));
            e.process(&mut l, &mut r);
            out.extend(l);
        }
        out[(0.5 * 48_000.0) as usize..].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max)
    };
    let base = Settings { mix: 1.0, mode: 1, ..Default::default() };
    let still = max_step(&|_| base);
    eprintln!("baseline max step {still:.4}");
    let cases: Vec<(&str, Box<dyn Fn(usize) -> Settings>)> = vec![
        ("freeze on/off", Box::new(move |i| Settings { freeze: (2 * 48_000..4 * 48_000).contains(&i), ..base })),
        ("mode changes", Box::new(move |i| Settings { mode: (i / 48_000) % 3, ..base })),
        ("size sweep", Box::new(move |i| Settings { size: (i as f32 / n as f32).min(1.0), ..base })),
        ("pre-delay sweep", Box::new(move |i| Settings { predelay_ms: 250.0 * (i as f32 / n as f32), ..base })),
        ("mix sweep", Box::new(move |i| Settings { mix: 0.5 + 0.5 * (i as f32 * 1e-4).sin(), ..base })),
    ];
    for (name, f) in cases {
        let m = max_step(&*f);
        eprintln!("{name}: max step {m:.4}");
        assert!(m <= still * 2.0 + 0.02, "{name}: step {m} vs baseline {still}");
    }
}

#[test]
fn never_produces_nan_or_infinity() {
    let fs = 48_000.0;
    let sq: Vec<(f32, f32)> = (0..48_000).map(|i| { let v = if (i / 50) % 2 == 0 { 1.0 } else { -1.0 }; (v, v) }).collect();
    let dc = vec![(1.0f32, 1.0f32); 48_000];
    let big: Vec<(f32, f32)> = (0..48_000).map(|i| ((i % 7) as f32 * 3.0 - 9.0, 8.0)).collect();
    for mode in 0..3 {
        for input in [&sq, &dc, &big] {
            for freeze in [false, true] {
                let mut e = Engine::new(fs);
                let s = Settings { mode, mix: 1.0, decay_s: 20.0, size: 1.0, mod_depth: 1.0, mod_rate: 5.0, diffusion: 1.0, bass_decay: 2.0, freeze, duck_db: 30.0, width: 1.5, out_gain_db: 12.0, ..Default::default() };
                let (l, r) = run(&mut e, &s, input, 480);
                assert!(l.iter().chain(&r).all(|v| v.is_finite()), "mode {mode} freeze {freeze}");
            }
        }
    }
}

#[test]
fn gain_staging_keeps_a_loud_burst_near_unity() {
    // a 0 dBFS (peak) noise burst at the defaults produces wet peaks no more than about +3 dB over the input
    let fs = 48_000.0;
    for mode in 0..3 {
        let mut r = Rng(3);
        let mut lp = 0.0f32;
        let burst: Vec<(f32, f32)> = (0..24_000).map(|_| { lp += 0.3 * (r.f() - lp); (lp, lp) }).collect(); // coloured noise
        let pk = burst.iter().fold(0.0f32, |m, v| m.max(v.0.abs()));
        let burst: Vec<(f32, f32)> = burst.iter().map(|v| (v.0 / pk, v.1 / pk)).collect();
        let mut input = burst;
        input.extend(vec![(0.0, 0.0); 48_000 * 3]);
        let (l, rr) = run(&mut Engine::new(fs), &Settings { mode, mix: 1.0, ..Default::default() }, &input, 256);
        let peak = l.iter().chain(&rr).fold(0.0f32, |m, v| m.max(v.abs()));
        eprintln!("mode {mode}: wet peak {:.1} dB re 0 dBFS", 20.0 * peak.log10());
        assert!(peak <= 1.42, "mode {mode}: wet peak {peak}");
    }
}

#[test]
fn silence_sleeps_and_new_input_wakes_instantly() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let s = Settings { mix: 1.0, decay_s: 0.3, ..Default::default() };
    let (l, r) = run(&mut e, &s, &impulse(48_000 * 4), 256);
    assert!(e.is_asleep(), "tank should be asleep after the tail");
    assert!(l[3 * 48_000..].iter().chain(&r[3 * 48_000..]).all(|v| *v == 0.0), "asleep means exact zeros");
    let mut l2 = vec![0.5f32; 64];
    let mut r2 = vec![0.5f32; 64];
    e.set_settings(&s);
    e.process(&mut l2, &mut r2);
    let mut more = vec![0.0f32; 4800];
    let mut more_r = vec![0.0f32; 4800];
    e.process(&mut more, &mut more_r); // the 10 ms pre-delay passes, then the tank wakes on the very sample the sound arrives
    assert!(!e.is_asleep());
    assert!(more.iter().any(|v| v.abs() > 1e-4), "the woken tank must ring");
}

#[test]
fn echo_density_builds_quickly() {
    // Abel & Huang normalised echo density: fraction of samples beyond one standard deviation in a sliding window,
    // divided by the value for Gaussian noise (0.3173). 1.0 means noise-like (no grainy flutter).
    let fs = 48_000.0;
    for mode in [0usize, 1] {
        let (l, _) = ir(&Settings { diffusion: 0.75, ..wet(mode, 2.0) }, fs, 0.6);
        let (w, t) = (480usize, (0.15 * fs) as usize);
        let seg = &l[t - w / 2..t + w / 2];
        let sd = (seg.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / seg.len() as f64).sqrt();
        let frac = seg.iter().filter(|v| (v.abs() as f64) > sd).count() as f64 / seg.len() as f64;
        let ned = frac / 0.3173;
        eprintln!("mode {mode}: echo density at 150 ms = {ned:.2}");
        assert!(ned >= 0.9, "mode {mode}: {ned}");
    }
}

#[test]
fn cpu_is_light_for_the_heaviest_mode() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let s = Settings { mode: 1, mod_depth: 1.0, mix: 0.5, ..Default::default() };
    let mut r = Rng(5);
    let secs = 10;
    let mut l: Vec<f32> = (0..secs * 48_000).map(|_| r.f() * 0.3).collect();
    let mut rr = l.clone();
    let t0 = std::time::Instant::now();
    for (a, b) in l.chunks_mut(256).zip(rr.chunks_mut(256)) {
        e.set_settings(&s);
        e.process(a, b);
    }
    let used = t0.elapsed().as_secs_f64() / secs as f64;
    eprintln!("Hall, stereo, 48 kHz, 256-sample buffers: {:.2} % of one core", used * 100.0);
    assert!(used < 0.05, "{:.1} % of a core", used * 100.0);
}

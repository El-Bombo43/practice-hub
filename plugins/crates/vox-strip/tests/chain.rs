use phub_common::dsp::gain_to_db;
use vox_strip::dsp::{comp_reduction, Engine, Settings, COMP, DESS, GATE, HPF, SAT};

const FS: f64 = 48_000.0;
fn sine(f: f32, amp: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * f * i as f32 / FS as f32).sin()).collect()
}
fn off() -> Settings {
    Settings { on: [false; 5], ..Default::default() }
}
fn run(s: &Settings, x: &[f32]) -> Vec<f32> {
    let mut e = Engine::new(FS);
    let mut out = x.to_vec();
    for ch in out.chunks_mut(256) {
        e.set_settings(s);
        e.process(ch, None);
    }
    out
}
fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt() as f32
}
fn tail_db(x: &[f32]) -> f32 {
    gain_to_db(rms(&x[x.len() / 2..]))
}

#[test]
fn all_off_is_bit_identical() {
    let x: Vec<f32> = (0..20_000).map(|i| ((i * 7919) % 1000) as f32 / 500.0 - 1.0).collect();
    let y = run(&off(), &x);
    assert!(x.iter().zip(&y).all(|(a, b)| a.to_bits() == b.to_bits()));
    let m0 = Settings { mix: 0.0, on: [true; 5], sat_drive: 0.5, ..Default::default() };
    assert!(run(&m0, &x).iter().zip(&x).all(|(a, b)| a.to_bits() == b.to_bits()), "mix 0 nulls");
}

#[test]
fn hpf_corner_and_slope() {
    for hp24 in [false, true] {
        let s = Settings { on: [false, true, false, false, false], hpf_24: hp24, hpf_freq: 80.0, ..Default::default() };
        let at = |f: f32| tail_db(&run(&s, &sine(f, 0.5, 96_000))) - tail_db(&sine(f, 0.5, 96_000));
        let c = at(80.0);
        assert!((c + 3.01).abs() < 0.25, "24={hp24}: corner {c}");
        let slope = at(40.0) - at(20.0); // one octave below, should be ~ -(12|24) + small corner effects
        eprintln!("hp24={hp24} corner {c:.2} dB, octave step {slope:.1}");
        let want = if hp24 { 24.0 } else { 12.0 };
        assert!((slope.abs() - want).abs() < 3.0);
    }
}

#[test]
fn deesser_split_band() {
    let s = Settings { on: [false, false, true, false, false], ds_freq: 6500.0, ds_thresh: -30.0, ds_range: 6.0, ..Default::default() };
    let x = sine(6500.0, 0.316, 48_000);
    let d = tail_db(&run(&s, &x)) - tail_db(&x);
    assert!((d + 6.0).abs() <= 0.5, "6.5k change {d}");
    let low = sine(500.0, 0.05, 48_000);
    let both: Vec<f32> = low.iter().zip(&x).map(|(a, b)| a + b).collect();
    let y = run(&s, &both);
    // 500 Hz component: correlate
    let comp = |v: &[f32]| (v.iter().zip(&sine(500.0, 1.0, 48_000)).skip(24_000).map(|(a, b)| a * b).sum::<f32>() / 12_000.0).abs();
    let ch = 20.0 * (comp(&y) / comp(&both)).log10();
    assert!(ch.abs() < 0.2, "500 Hz changed {ch}");
}

#[test]
fn compressor_static_curve_and_time_constants() {
    let (t, r, k) = (-18.0, 4.0, 6.0);
    for lvl in [-60.0f32, -30.0, -21.0, -18.0, -15.0, -6.0, 0.0] {
        let s = Settings { on: [false, false, false, true, false], comp_thresh: t, comp_ratio: r, comp_knee: k, comp_attack_ms: 0.1, comp_release_ms: 10.0, ..Default::default() };
        let x = sine(1000.0, 10f32.powf(lvl / 20.0), 48_000);
        let d = tail_db(&run(&s, &x)) - tail_db(&x);
        let want = -comp_reduction(lvl, t, r, k);
        assert!((d - want).abs() <= 0.5, "level {lvl}: got {d:.2}, want {want:.2}");
    }
    // attack: step from silence to a loud sine; gain reduction reaches 63% of its final value at the attack time
    let s = Settings { on: [false, false, false, true, false], comp_thresh: -30.0, comp_ratio: 20.0, comp_knee: 0.0, comp_attack_ms: 20.0, comp_release_ms: 100.0, ..Default::default() };
    let mut e = Engine::new(FS);
    e.set_settings(&s);
    let mut x = sine(1000.0, 0.5, 48_000);
    let mut gr = Vec::new();
    for ch in x.chunks_mut(48) {
        e.process(ch, None);
        gr.push(e.take_meters().comp_gr);
    }
    let fin = *gr.last().unwrap();
    let t63 = gr.iter().position(|g| *g >= 0.632 * fin).unwrap() as f32; // ms
    assert!((t63 - 20.0).abs() <= 2.5, "attack 63% at {t63} ms");
}

#[test]
fn opto_release_depends_on_the_program() {
    let s = Settings { on: [false, false, false, true, false], comp_opto: true, comp_thresh: -30.0, comp_ratio: 6.0, comp_release_ms: 200.0, ..Default::default() };
    let rec = |burst_ms: usize| -> f32 {
        let mut e = Engine::new(FS);
        e.set_settings(&s);
        let mut x = sine(1000.0, 0.5, burst_ms * 48);
        x.extend(sine(1000.0, 0.01, 48_000));
        let mut gr = Vec::new();
        let mut i = 0;
        for ch in x.chunks_mut(48) {
            e.process(ch, None);
            let g = e.take_meters().comp_gr;
            if i >= burst_ms { gr.push(g) }
            i += 1;
        }
        let peak = gr[0];
        gr.iter().position(|g| *g <= 0.368 * peak).unwrap_or(gr.len()) as f32
    };
    let (short, long) = (rec(50), rec(2000));
    eprintln!("opto recovery to 63%: after 50 ms burst {short} ms, after 2 s burst {long} ms");
    assert!(long >= 2.0 * short);
}

#[test]
fn gate_opens_fast_and_does_not_chatter() {
    let s = Settings { on: [true, false, false, false, false], gate_thresh: -40.0, gate_range: 40.0, ..Default::default() };
    let mut e = Engine::new(FS);
    e.set_settings(&s);
    let mut quiet = vec![0.0001f32; 4800];
    e.process(&mut quiet, None);
    assert!(!e.gate_is_open());
    // a step to a loud tone: must open within 1 ms (48 samples)
    let tone = sine(220.0, 0.3, 4800);
    let mut opened = None;
    for (k, ch) in tone.chunks(8).enumerate() {
        let mut c = ch.to_vec();
        e.process(&mut c, None);
        if e.gate_is_open() && opened.is_none() {
            opened = Some((k + 1) * 8);
        }
    }
    assert!(opened.unwrap() <= 48, "opened after {:?} samples", opened);
    // a decaying 220 Hz tone toggles at most once
    let mut e = Engine::new(FS);
    e.set_settings(&s);
    let n = 48_000;
    let decay: Vec<f32> = (0..n).map(|i| 0.5 * (-(i as f32) / (0.15 * 48_000.0)).exp() * (2.0 * std::f32::consts::PI * 220.0 * i as f32 / 48_000.0).sin()).collect();
    let mut toggles = 0;
    let mut state = false;
    for ch in decay.chunks(16) {
        let mut c = ch.to_vec();
        e.process(&mut c, None);
        if e.gate_is_open() != state {
            toggles += 1;
            state = e.gate_is_open();
        }
    }
    assert!(toggles <= 2, "toggled {toggles} times (open then close = 2)");
}

fn thd_db(x: &[f32], f: f32) -> f32 {
    // harmonics 2..8 relative to the fundamental over the second half
    let n = x.len() / 2;
    let seg = &x[x.len() - n..];
    let mag = |hz: f32| {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, v) in seg.iter().enumerate() {
            let ph = 2.0 * std::f64::consts::PI * hz as f64 * (i + x.len() - n) as f64 / FS;
            re += *v as f64 * ph.cos();
            im += *v as f64 * ph.sin();
        }
        (re * re + im * im).sqrt()
    };
    let fund = mag(f);
    let h: f64 = (2..9).map(|k| mag(f * k as f32).powi(2)).sum::<f64>().sqrt();
    (20.0 * (h / fund).log10()) as f32
}

#[test]
fn saturation_thd_level_and_aliasing() {
    for tube in [false, true] {
        let mut last = -200.0f32;
        for d in [0.0f32, 0.1, 0.3, 0.6, 1.0] {
            let s = Settings { on: [false, false, false, false, true], sat_tube: tube, sat_drive: d, ..Default::default() };
            let x = sine(1000.0, 0.126, 96_000);
            let y = run(&s, &x);
            let thd = thd_db(&y, 1000.0);
            let lvl = tail_db(&y) - tail_db(&x);
            eprintln!("tube={tube} drive {d}: THD {thd:.1} dB, level {lvl:+.2} dB");
            if d == 0.0 {
                assert!(thd < -80.0);
            }
            assert!(thd >= last - 0.5, "THD must rise with Drive");
            last = thd;
            assert!(lvl.abs() <= 1.5, "level {lvl}");
        }
        // aliasing: 5 kHz at full drive; anything that is not a harmonic of 5 kHz is alias
        let s = Settings { on: [false, false, false, false, true], sat_tube: tube, sat_drive: 1.0, ..Default::default() };
        let x = sine(5000.0, 0.126, 96_000);
        let y = run(&s, &x);
        let seg = &y[48_000..];
        let mag = |hz: f32| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, v) in seg.iter().enumerate() {
                let ph = 2.0 * std::f64::consts::PI * hz as f64 * i as f64 / FS;
                re += *v as f64 * ph.cos();
                im += *v as f64 * ph.sin();
            }
            (re * re + im * im).sqrt()
        };
        let fund = mag(5000.0);
        // 48 kHz: 5 kHz harmonics land on 10, 15, 20 kHz; aliases of the 5th+ harmonics land at 23, 22, 17... probe non-harmonic bins
        let worst = (1..24).map(|k| k as f32 * 1000.0 + 500.0).map(|hz| (20.0 * (mag(hz) / fund).log10()) as f32).fold(-300.0f32, f32::max);
        eprintln!("tube={tube}: worst non-harmonic bin {worst:.1} dB re fundamental");
        assert!(worst < -60.0);
    }
}

#[test]
fn module_switching_never_clicks_and_silence_is_silent() {
    let x = sine(300.0, 0.4, 96_000);
    let mut e = Engine::new(FS);
    let mut out = x.clone();
    for (b, ch) in out.chunks_mut(100).enumerate() {
        let mut s = Settings::default();
        s.on = [(b / 7) % 2 == 0, (b / 5) % 2 == 0, (b / 11) % 2 == 0, (b / 3) % 2 == 0, (b / 13) % 2 == 0];
        e.set_settings(&s);
        e.process(ch, None);
    }
    let step = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
    assert!(step(&out) < 3.0 * step(&x) + 0.05, "max step {} vs input {}", step(&out), step(&x));
    assert!(out.iter().all(|v| v.is_finite()));
    let z = run(&Settings { on: [true; 5], ..Default::default() }, &vec![0.0; 10_000]);
    assert!(z.iter().all(|v| *v == 0.0));
    let _ = (COMP, DESS, GATE, HPF, SAT);
}

#[test]
fn stereo_and_extreme_input_stay_finite_and_cpu_is_light() {
    let mut e = Engine::new(FS);
    let s = Settings { on: [true; 5], sat_drive: 1.0, comp_ratio: 20.0, ..Default::default() };
    e.set_settings(&s);
    let mut l: Vec<f32> = (0..48_000 * 10).map(|i| ((i * 2654435761u64 as usize) % 2000) as f32 / 1000.0 - 1.0).collect();
    let mut r = l.clone();
    let t = std::time::Instant::now();
    for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        e.process(a, Some(b));
    }
    let used = t.elapsed().as_secs_f64() / 10.0;
    eprintln!("all modules, stereo, 48 kHz: {:.2} % of a core", used * 100.0);
    assert!(l.iter().chain(&r).all(|v| v.is_finite()));
    assert!(used < 0.03);
    let mut dc = vec![1.0f32; 5000];
    e.process(&mut dc, None);
    assert!(dc.iter().all(|v| v.is_finite()));
}

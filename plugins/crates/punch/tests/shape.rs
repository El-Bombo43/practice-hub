use punch::dsp::{Engine, Settings};

const FS: f64 = 48_000.0;
fn sine(f: f32, amp: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| amp * (2.0 * std::f32::consts::PI * f * i as f32 / FS as f32).sin()).collect()
}
fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt() as f32
}
fn db(a: f32, b: f32) -> f32 {
    20.0 * (a / b).log10()
}
fn run(s: &Settings, x: &[f32]) -> Vec<f32> {
    let mut e = Engine::new(FS);
    e.set_settings(s);
    let mut y = x.to_vec();
    for c in y.chunks_mut(64) {
        e.process(c, None);
    }
    y
}
/// gain change (dB) recorded every 24 samples
fn gain_curve(s: &Settings, x: &[f32]) -> Vec<f32> {
    let mut e = Engine::new(FS);
    e.set_settings(s);
    let mut g = Vec::new();
    for c in x.chunks(24) {
        let mut c = c.to_vec();
        e.process(&mut c, None);
        g.push(e.take_meters().gain_db);
    }
    g
}
fn burst(f: f32, amp: f32, lead: usize, total: usize) -> Vec<f32> {
    let mut x = vec![0.0; lead];
    x.extend(sine(f, amp, total - lead));
    x
}
fn drum_loop(amp: f32) -> Vec<f32> {
    let mut seed = 1u32;
    let mut x = vec![0.0f32; 96_000];
    for hit in 0..8 {
        let t0 = hit * 12_000;
        for k in 0..9000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let n = (seed >> 8) as f32 / 8_388_608.0 - 1.0;
            let env = (-(k as f32) / 1500.0).exp();
            x[t0 + k] += amp * env * (0.6 * n + 0.4 * (2.0 * std::f32::consts::PI * (180.0 + 60.0 * env) * k as f32 / 48_000.0).sin());
        }
    }
    x
}

#[test]
fn neutral_is_bit_identical() {
    let x = drum_loop(0.7);
    let y = run(&Settings::default(), &x);
    assert!(x.iter().zip(&y).all(|(a, b)| a.to_bits() == b.to_bits()));
}

#[test]
fn level_independent() {
    let s = Settings { attack: 0.8, sustain: -0.6, focus: 20.0, ..Default::default() };
    let loud = gain_curve(&s, &drum_loop(0.5));
    let quiet = gain_curve(&s, &drum_loop(0.05));
    // compare while the quiet loop is still above -60 dBFS (below that the -80 dB silence floor takes over)
    let worst = loud.iter().zip(&quiet).enumerate().filter(|(k, _)| (k * 24) % 12_000 < 5000).map(|(_, (a, b))| (a - b).abs()).fold(0.0f32, f32::max);
    eprintln!("-6 dB vs -26 dB gain curves differ by at most {worst:.3} dB");
    assert!(worst <= 0.5);
    assert!(loud.iter().fold(0.0f32, |m, g| m.max(g.abs())) > 6.0, "the curve must actually do something");
}

#[test]
fn attack_swings_about_12_db() {
    for sign in [1.0f32, -1.0] {
        let s = Settings { attack: sign, ..Default::default() };
        let x = burst(1000.0, 0.3, 4800, 48_000);
        let y = run(&s, &x);
        let first5 = db(rms(&y[4800..4800 + 240]), rms(&x[4800..4800 + 240]));
        eprintln!("attack {sign:+}: first 5 ms {first5:+.1} dB");
        assert!((first5 - 12.0 * sign).abs() <= 1.5, "{first5}");
    }
}

#[test]
fn sustain_shapes_a_decaying_tail() {
    let n = 48_000;
    let x: Vec<f32> = (0..n).map(|i| 0.5 * (-(i as f32) / (0.3 * 48_000.0)).exp() * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48_000.0).sin()).collect();
    let at = (0.1 * 48_000.0) as usize;
    for (sus, want) in [(-1.0f32, -12.0f32), (1.0, 12.0)] {
        let y = run(&Settings { sustain: sus, ..Default::default() }, &x);
        let d = db(rms(&y[at..at + 480]), rms(&x[at..at + 480]));
        eprintln!("sustain {sus:+}: tail at 100 ms {d:+.1} dB");
        assert!(if sus < 0.0 { d <= want } else { d >= want });
    }
}

#[test]
fn steady_tone_settles_to_unity() {
    let s = Settings { attack: 1.0, sustain: 1.0, ..Default::default() };
    let x = sine(1000.0, 0.3, 48_000);
    let y = run(&s, &x);
    let late = db(rms(&y[(2.0 * 0.150 * 48_000.0) as usize..]), rms(&x[(2.0 * 0.150 * 48_000.0) as usize..]));
    assert!(late.abs() <= 0.2, "settled at {late} dB");
    // silence: gain eases to 0 dB
    let z = run(&s, &vec![0.00001; 20_000]);
    assert!(z.iter().zip(vec![0.00001f32; 20_000]).all(|(a, b)| (a - b).abs() < 1e-9));
}

#[test]
fn focus_steers_the_detector() {
    let s = Settings { attack: 1.0, focus: 500.0, ..Default::default() };
    // bursts with a 2 ms fade-in: an abrupt start is broadband and would tell the detector nothing about the tone
    let faded = |f: f32| {
        let mut x = burst(f, 0.3, 4800, 12_000);
        for k in 0..96 {
            x[4800 + k] *= k as f32 / 96.0;
        }
        x
    };
    let peak = |f: f32| gain_curve(&s, &faded(f)).iter().fold(0.0f32, |m, g| m.max(*g));
    let (low, high) = (peak(60.0), peak(3000.0));
    eprintln!("focus 500: 60 Hz burst {low:.1} dB, 3 kHz burst {high:.1} dB");
    assert!(low < 0.5 * high);
}

#[test]
fn clip_guard_never_exceeds_full_scale() {
    let s = Settings { attack: 1.0, clip: true, ..Default::default() };
    let y = run(&s, &burst(300.0, 1.0, 2400, 24_000));
    assert!(y.iter().all(|v| v.abs() <= 1.0));
    let unclipped = run(&Settings { attack: 1.0, ..Default::default() }, &burst(300.0, 1.0, 2400, 24_000));
    assert!(unclipped.iter().any(|v| v.abs() > 1.0), "the test needs a boost that would clip");
}

#[test]
fn stereo_link_keeps_the_image_and_everything_stays_finite() {
    let mut e = Engine::new(FS);
    e.set_settings(&Settings { attack: 1.0, sustain: -1.0, focus: 200.0, ..Default::default() });
    let mut l = drum_loop(0.6);
    let mut r: Vec<f32> = l.iter().map(|v| v * 0.25).collect();
    let (l0, r0) = (l.clone(), r.clone());
    let t = std::time::Instant::now();
    for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        e.process(a, Some(b));
    }
    eprintln!("{:.3} % of a core", t.elapsed().as_secs_f64() / 2.0 * 100.0);
    for i in 0..l.len() {
        if l0[i].abs() > 1e-3 {
            assert!((l[i] / l0[i] - r[i] / r0[i]).abs() < 1e-3, "linked channels share one gain");
        }
    }
    let mut dc = vec![1.0f32; 4000];
    e.process(&mut dc, None);
    assert!(dc.iter().all(|v| v.is_finite()));
}

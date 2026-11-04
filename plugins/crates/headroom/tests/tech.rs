use headroom::meter::{Meter, Transport};

fn sine(fs: f64, f: f64, db: f64, secs: f64, start: f64) -> Vec<f32> {
    let a = 10f64.powf(db / 20.0);
    (0..(secs * fs) as usize).map(|i| (a * (2.0 * std::f64::consts::PI * f * (i as f64 / fs + start)).sin()) as f32).collect()
}
fn run(fs: f64, l: &[f32], r: &[f32]) -> Meter {
    let mut m = Meter::new(fs);
    let t = Transport { playing: true, beats: Some(0.0), seconds: 0.0, tempo: 120.0 };
    for (a, b) in l.chunks(480).zip(r.chunks(480)) {
        m.process(a, Some(b), -1.0, &t);
    }
    m
}
fn tone(fs: f64, segs: &[(f64, f64)]) -> Vec<f32> {
    let mut v = Vec::new();
    for (db, secs) in segs {
        let n = v.len() as f64 / fs;
        v.extend(sine(fs, 997.0, *db, *secs, n));
    }
    v
}

#[test]
fn ebu_3341_loudness_cases() {
    for fs in [44_100.0, 48_000.0, 96_000.0] {
        // case 1/2: stereo 997 Hz at -23 and -33 LUFS (peak level = LUFS for a stereo sine)
        for lvl in [-23.0, -33.0] {
            let x = tone(fs, &[(lvl, 20.0)]);
            let m = run(fs, &x, &x);
            let s = m.snapshot();
            eprintln!("fs {fs} {lvl}: M {:.2} S {:.2} I {:.2}", s.momentary, s.short_term, s.integrated);
            assert!((s.momentary - lvl as f32).abs() <= 0.1 && (s.short_term - lvl as f32).abs() <= 0.1 && (s.integrated - lvl as f32).abs() <= 0.1);
        }
        // case 3: 10 s at -36, 10 s at -23, 10 s at -36 -> relative gate: integrated -23
        let x = tone(fs, &[(-36.0, 10.0), (-23.0, 60.0), (-36.0, 10.0)]);
        assert!((run(fs, &x, &x).snapshot().integrated + 23.0).abs() <= 0.1);
        // case 4: absolute gate: -72 (below -70 LUFS) / -36 / -23 / -36 / -72 -> -23
        let x = tone(fs, &[(-72.0, 5.0), (-36.0, 1.0), (-23.0, 40.0), (-36.0, 1.0), (-72.0, 5.0)]);
        let s = run(fs, &x, &x);
        eprintln!("gating case: {:.2}", s.snapshot().integrated);
        assert!((s.snapshot().integrated + 23.0).abs() <= 0.1);
    }
}

#[test]
fn true_peak_sees_inter_sample_overs() {
    let fs = 48_000.0;
    // fs/4 sine at 45 degrees: samples sit at +-0.707 of the peak, so the true peak is 3 dB above the sample peak
    let a = 0.5f64;
    let x: Vec<f32> = (0..48_000).map(|i| (a * (std::f64::consts::PI / 2.0 * i as f64 + std::f64::consts::PI / 4.0).sin() / std::f64::consts::FRAC_1_SQRT_2 * std::f64::consts::FRAC_1_SQRT_2) as f32).collect();
    let m = run(fs, &x, &x);
    let s = m.snapshot();
    eprintln!("sample peak {:.2}, true peak {:.2}", s.sample_peak, s.tp_max[0]);
    assert!(s.tp_max[0] > s.sample_peak + 2.5, "inter-sample peak must exceed the sample peak");
    // 997 Hz at -6 dBFS: true peak -6 within the Tech 3341 tolerance
    let y = tone(fs, &[(-6.0, 5.0)]);
    let s = run(fs, &y, &y);
    assert!((s.snapshot().tp_max[0] + 6.0).abs() <= 0.3, "{}", s.snapshot().tp_max[0]);
}

#[test]
fn lra_cases() {
    let fs = 48_000.0;
    for (segs, want) in [(vec![(-20.0, 20.0), (-30.0, 20.0)], 10.0), (vec![(-20.0, 20.0), (-15.0, 20.0)], 5.0), (vec![(-40.0, 20.0), (-20.0, 20.0)], 20.0)] {
        let x = tone(fs, &segs);
        let m = run(fs, &x, &x);
        eprintln!("LRA {:.2} (want {want})", m.snapshot().lra);
        assert!((m.snapshot().lra - want).abs() <= 1.0);
    }
}

#[test]
fn overs_are_counted_as_events() {
    let fs = 48_000.0;
    let mut x = vec![0.0f32; 48_000 * 4];
    for k in 0..5 {
        x[10_000 + k * 30_000] = 1.0; // 0 dBFS pulses, 625 ms apart
    }
    let m = run(fs, &x, &x);
    assert_eq!(m.snapshot().overs, 5);
    let mut y = vec![0.0f32; 48_000];
    y[1000] = 1.0;
    y[1000 + (0.030 * fs) as usize] = 1.0; // 30 ms apart: one event
    assert_eq!(run(fs, &y, &y).snapshot().overs, 1);
    assert_eq!(run(fs, &y, &y).snapshot().clips, 1);
}

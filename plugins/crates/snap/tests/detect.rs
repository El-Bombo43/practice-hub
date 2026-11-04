use snap::pitch::{Detector, Ring};

pub const FS: f32 = 48_000.0;

/// A synthetic vowel: an impulse train through three formant resonators (about 700 / 1200 / 2600 Hz), computed in
/// continuous time so pulses land between samples exactly (no period jitter from rounding).
pub fn vowel(f0: impl Fn(usize) -> f32, n: usize) -> Vec<f32> {
    let formants = [(700.0f64, 90.0f64, 1.0f64), (1200.0, 110.0, 0.6), (2600.0, 160.0, 0.3)];
    let mut pulses: Vec<f64> = Vec::new();
    let mut phase = 0.0f64;
    for i in 0..n {
        let inc = f0(i) as f64 / FS as f64;
        phase += inc;
        if phase >= 1.0 {
            phase -= 1.0;
            pulses.push((i as f64 - phase / inc) / FS as f64);
        }
    }
    let mut y = vec![0.0f32; n];
    let mut first = 0;
    for i in 0..n {
        let t = i as f64 / FS as f64;
        while first < pulses.len() && t - pulses[first] > 0.05 {
            first += 1;
        }
        let mut acc = 0.0;
        for p in &pulses[first..] {
            let d = t - p;
            if d < 0.0 {
                break;
            }
            for (f, bw, a) in formants {
                acc += a * (-std::f64::consts::PI * bw * d).exp() * (2.0 * std::f64::consts::PI * f * d).sin();
            }
        }
        y[i] = acc as f32;
    }
    let pk = y.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    y.iter().map(|v| 0.4 * v / pk).collect()
}

pub fn track(x: &[f32], fmin: f32, fmax: f32) -> Vec<(usize, f32, f32)> {
    let mut det = Detector::new(FS, fmin, fmax);
    let mut ring = Ring::new(8192);
    let mut out = Vec::new();
    for (i, s) in x.iter().enumerate() {
        ring.push(*s);
        det.push(*s);
        if i % 64 == 63 {
            let f = det.analyse(&ring);
            out.push((i, f.f0, f.aperiodicity));
        }
    }
    out
}

fn cents(a: f32, b: f32) -> f32 {
    1200.0 * (a / b).log2()
}

#[test]
fn sines_and_vowels_are_tracked_to_a_few_cents() {
    for f in [82.4f32, 110.0, 196.0, 261.6, 440.0, 622.0, 784.0] {
        for vow in [false, true] {
            let x: Vec<f32> = if vow { vowel(|_| f, 24_000) } else { (0..24_000).map(|i| 0.4 * (2.0 * std::f32::consts::PI * f * i as f32 / FS).sin()).collect() };
            let t = track(&x, 55.0, 1000.0);
            let late: Vec<_> = t.iter().filter(|(i, _, _)| *i > 12_000).collect();
            let worst = late.iter().map(|(_, g, _)| cents(*g, f).abs()).fold(0.0f32, f32::max);
            let octave = late.iter().filter(|(_, g, _)| cents(*g, f).abs() > 600.0).count();
            eprintln!("{f} Hz vowel={vow}: worst {worst:.1} cents, octave errors {octave}");
            assert!(worst < 5.0 && octave == 0, "{f} Hz vowel={vow}: {worst}");
        }
    }
}

#[test]
fn white_noise_is_unvoiced() {
    let mut s = 12345u32;
    let x: Vec<f32> = (0..96_000).map(|_| { s = s.wrapping_mul(1664525).wrapping_add(1013904223); ((s >> 8) as f32 / 8_388_608.0 - 1.0) * 0.3 }).collect();
    let t = track(&x, 80.0, 800.0);
    let voiced = t.iter().skip(20).filter(|(_, f, a)| *f > 0.0 && *a < 0.18).count();
    let total = t.len() - 20;
    eprintln!("noise: {voiced}/{total} frames look voiced");
    assert!((voiced as f32) < 0.05 * total as f32);
}

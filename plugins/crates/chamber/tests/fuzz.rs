//! Random settings every block, odd block sizes, every sample rate, resets: nothing may panic or go non-finite, and the
//! engine must settle once the abuse stops.
mod common;
use chamber::dsp::{Engine, Settings};
use common::*;

fn rnd(r: &mut Rng, lo: f32, hi: f32) -> f32 {
    lo + (hi - lo) * (r.f() * 0.5 + 0.5)
}
fn rnd_log(r: &mut Rng, lo: f32, hi: f32) -> f32 {
    (lo.ln() + (hi.ln() - lo.ln()) * (r.f() * 0.5 + 0.5)).exp()
}

fn random_settings(r: &mut Rng) -> Settings {
    Settings {
        mode: (r.next() % 3) as usize,
        mix: rnd(r, 0.0, 1.0),
        predelay_ms: rnd(r, 0.0, 250.0),
        predelay_sync: r.next() % 4 == 0,
        predelay_div: (r.next() % 10) as usize,
        size: rnd(r, 0.0, 1.0),
        decay_s: rnd_log(r, 0.1, 20.0),
        diffusion: rnd(r, 0.0, 1.0),
        early_db: rnd(r, -24.0, 6.0),
        low_cut: rnd_log(r, 20.0, 1000.0),
        high_cut: rnd_log(r, 1000.0, 20_000.0),
        damping_hz: rnd_log(r, 1000.0, 20_000.0),
        bass_decay: rnd_log(r, 0.25, 2.0),
        mod_rate: rnd_log(r, 0.05, 5.0),
        mod_depth: rnd(r, 0.0, 1.0),
        width: rnd(r, 0.0, 1.5),
        duck_db: if r.next() % 2 == 0 { 0.0 } else { rnd(r, 0.0, 30.0) },
        duck_thresh_db: rnd(r, -60.0, 0.0),
        duck_release_ms: rnd_log(r, 50.0, 2000.0),
        freeze: r.next() % 5 == 0,
        out_gain_db: rnd(r, -24.0, 12.0),
        tempo_bpm: rnd(r, 20.0, 300.0) as f64,
    }
}

#[test]
fn random_settings_never_panic_or_go_non_finite() {
    for (k, fs) in [22_050.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0].into_iter().enumerate() {
        for seed in 0..4u64 {
            let mut r = Rng(seed * 7919 + k as u64 * 104_729 + 1);
            let mut e = Engine::new(fs);
            let mut s = random_settings(&mut r);
            let mut peak = 0.0f32;
            for blk in 0..200 {
                if blk % 3 == 0 {
                    s = random_settings(&mut r);
                }
                e.set_settings(&s);
                if r.next() % 50 == 0 {
                    e.reset(fs);
                }
                let n = [0usize, 1, 2, 31, 32, 33, 64, 100, 256, 513, 1024, 4096][(r.next() % 12) as usize];
                let amp = rnd(&mut r, 0.0, 1.0) * if r.next() % 10 == 0 { 8.0 } else { 1.0 };
                let mut l: Vec<f32> = (0..n).map(|_| r.f() * amp).collect();
                let mut rr: Vec<f32> = (0..n).map(|_| r.f() * amp).collect();
                e.process(&mut l, &mut rr);
                assert!(l.iter().chain(&rr).all(|v| v.is_finite()), "seed {seed} fs {fs} block {blk}");
                peak = peak.max(l.iter().chain(&rr).fold(0.0, |m, v| m.max(v.abs())));
            }
            // abuse over: a benign setting must settle back to silence (no frozen or runaway loop left behind)
            let calm = Settings { mix: 1.0, decay_s: 1.0, ..Default::default() };
            let (l, rr) = run(&mut e, &calm, &vec![(0.0, 0.0); (fs as usize) * 20], 512);
            let tail = rms(&l[l.len() - fs as usize..]).max(rms(&rr[rr.len() - fs as usize..]));
            assert!(tail < 1e-3, "seed {seed} fs {fs}: still ringing at {tail} (earlier peak {peak})");
        }
    }
}

#[test]
fn empty_and_single_sample_blocks_are_fine() {
    let mut e = Engine::new(48_000.0);
    let s = Settings { mix: 0.5, duck_db: 12.0, ..Default::default() };
    e.set_settings(&s);
    e.process(&mut [], &mut []);
    for i in 0..5000 {
        let mut l = [(i as f32 * 0.01).sin()];
        let mut r = [0.2f32];
        e.process(&mut l, &mut r);
        assert!(l[0].is_finite() && r[0].is_finite());
    }
}

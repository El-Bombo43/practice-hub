//! Randomised stress: random settings every block, odd block sizes, every sample rate. Nothing may panic, and nothing
//! may come out as NaN, infinity or an absurd level. A panic on the audio thread would take the host down with it.
use contour::dsp::response::band_db;
use contour::dsp::{BandSettings, Engine, FilterType, GlobalSettings, BAND_COUNT};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f()
    }
    fn log(&mut self, lo: f32, hi: f32) -> f32 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.f()).exp()
    }
    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
}

const TYPES: [FilterType; 8] = [
    FilterType::Bell, FilterType::LowShelf, FilterType::HighShelf, FilterType::TiltShelf,
    FilterType::LowCut, FilterType::HighCut, FilterType::Notch, FilterType::BandPass,
];
const SLOPES: [u32; 8] = [6, 12, 18, 24, 36, 48, 72, 96];

fn random_band(r: &mut Rng) -> BandSettings {
    BandSettings {
        used: r.chance(0.8),
        enabled: r.chance(0.85),
        ftype: TYPES[r.pick(8)],
        freq: r.log(10.0, 30_000.0),
        gain_db: r.range(-30.0, 30.0),
        q: r.log(0.025, 40.0),
        slope: SLOPES[r.pick(8)],
        dynamic: r.chance(0.5),
        thresh_db: r.range(-60.0, 0.0),
        range_db: r.range(-30.0, 30.0),
        attack_ms: r.log(0.5, 200.0),
        release_ms: r.log(10.0, 2000.0),
        ext: r.chance(0.3),
    }
}

fn noise(r: &mut Rng, n: usize, amp: f32) -> Vec<f32> {
    (0..n).map(|_| r.range(-amp, amp)).collect()
}

fn run(seed: u64, fs: f64, blocks: usize, stereo: bool, change_every_block: bool) {
    let mut r = Rng(seed | 1);
    let mut e = Engine::new(fs);
    let mut bands = [BandSettings::default(); BAND_COUNT];
    for b in bands.iter_mut() {
        *b = random_band(&mut r);
    }
    let mut global = GlobalSettings { out_gain_db: r.range(-24.0, 24.0), auto_gain: r.chance(0.5), gain_scale: r.range(0.0, 2.0) };
    let mut prev_bound_db = f64::MIN;
    for blk in 0..blocks {
        if change_every_block || blk % 7 == 0 {
            // change a few bands, sometimes everything, to provoke crossfades, redesigns and resets
            for _ in 0..(1 + r.pick(5)) {
                let i = r.pick(BAND_COUNT);
                bands[i] = random_band(&mut r);
            }
            if r.chance(0.1) {
                global = GlobalSettings { out_gain_db: r.range(-24.0, 24.0), auto_gain: r.chance(0.5), gain_scale: r.range(0.0, 2.0) };
            }
        }
        e.set_settings(&bands, global);
        e.set_solo(if r.chance(0.1) { Some(r.pick(BAND_COUNT)) } else { None });
        if r.chance(0.01) {
            e.reset(fs); // a host reset in the middle of playback
        }
        let n = [1usize, 2, 31, 32, 33, 64, 100, 256, 513, 1024, 4096][r.pick(11)];
        let amp = r.range(0.0, 1.0);
        let loud = r.chance(0.1);
        let mut l = noise(&mut r, n, if loud { 4.0 } else { amp });
        let mut rr = noise(&mut r, n, amp);
        let sc_l = noise(&mut r, n, 0.5);
        let sc_r = noise(&mut r, n, 0.5);
        let peak_in = l.iter().chain(rr.iter()).fold(0.0f32, |m, v| m.max(v.abs()));
        let use_sc = r.chance(0.5);
        if stereo {
            e.process(&mut l, &mut rr, use_sc.then_some((&sc_l[..], &sc_r[..])));
        } else {
            e.process_mono(&mut l, use_sc.then_some(&sc_l[..]));
        }
        let out: Vec<f32> = if stereo { l.iter().chain(rr.iter()).copied().collect() } else { l.clone() };
        assert!(out.iter().all(|v| v.is_finite()), "seed {seed} fs {fs} block {blk}: non-finite output");
        // Stacked boosts legitimately reach >150 dB, so bound against the summed per-band peak gain (an upper bound
        // on the combined curve) plus the output gain, with 24 dB of margin for transients and crossfades.
        let peak_out = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let gs = global.gain_scale as f64;
        let mut bound_db = global.out_gain_db as f64 + 24.0;
        for b in bands.iter().filter(|b| b.used && b.enabled) {
            let mut worst = f64::MIN;
            for k in 0..200 {
                let f = 5.0 * (fs * 0.5 / 5.0).powf(k as f64 / 199.0);
                worst = worst.max(band_db(b, b.gain_db as f64 * gs, fs, f));
                if b.dynamic {
                    worst = worst.max(band_db(b, (b.gain_db + b.range_db) as f64 * gs, fs, f));
                }
            }
            bound_db += worst.max(0.0);
        }
        // high-gain resonances ring out slowly, so keep the worst curve seen so far
        prev_bound_db = prev_bound_db.max(bound_db);
        let worst_db = prev_bound_db;
        let bound = peak_in.max(1e-3) as f64 * 10f64.powf(worst_db / 20.0) + 1.0;
        assert!((peak_out as f64) <= bound, "seed {seed} fs {fs} block {blk}: level blow-up {peak_out} from {peak_in}, bound {bound}");
        for i in 0..BAND_COUNT {
            assert!(e.dyn_db(i).is_finite());
        }
    }
    // Recovery: after the abuse, a benign flat setting must settle to unity (no latched state, no runaway feedback).
    let flat = [BandSettings::default(); BAND_COUNT];
    e.set_settings(&flat, GlobalSettings { out_gain_db: 0.0, auto_gain: false, gain_scale: 1.0 });
    e.set_solo(None);
    let mut last = 0.0f32;
    for _ in 0..((fs * 8.0) as usize / 512) {
        let mut l = vec![0.1f32; 512];
        let mut rr = vec![0.1f32; 512];
        if stereo { e.process(&mut l, &mut rr, None) } else { e.process_mono(&mut l, None) }
        last = l.iter().fold(0.0f32, |m, v| m.max((v - 0.1).abs()));
    }
    assert!(last < 1e-3, "seed {seed} fs {fs}: engine did not settle after abuse (residual {last})");
}

#[test]
fn random_settings_never_panic_or_blow_up_at_every_sample_rate() {
    for (k, fs) in [22_050.0, 44_100.0, 48_000.0, 88_200.0, 96_000.0, 176_400.0, 192_000.0].into_iter().enumerate() {
        for seed in 0..6u64 {
            run(seed * 7919 + k as u64 * 104_729 + 1, fs, 80, true, seed % 2 == 0);
        }
    }
}

#[test]
fn mono_layout_survives_the_same_abuse() {
    for seed in 0..10u64 {
        run(seed * 31 + 5, 48_000.0, 80, false, true);
    }
}

#[test]
fn single_sample_and_empty_blocks_are_fine() {
    let mut e = Engine::new(48_000.0);
    let mut bands = [BandSettings::default(); BAND_COUNT];
    bands[0] = BandSettings { used: true, gain_db: 6.0, dynamic: true, ..Default::default() };
    e.set_settings(&bands, GlobalSettings::default());
    let mut empty_l: Vec<f32> = vec![];
    let mut empty_r: Vec<f32> = vec![];
    e.process(&mut empty_l, &mut empty_r, None);
    for i in 0..2000 {
        let mut l = [(i as f32 * 0.01).sin()];
        let mut r = [0.0f32];
        e.process(&mut l, &mut r, Some((&[0.3], &[0.3])));
        assert!(l[0].is_finite());
    }
}

#[test]
fn a_sidechain_shorter_than_the_block_is_padded_with_silence() {
    let mut e = Engine::new(48_000.0);
    let mut bands = [BandSettings::default(); BAND_COUNT];
    bands[0] = BandSettings { used: true, dynamic: true, ext: true, ..Default::default() };
    e.set_settings(&bands, GlobalSettings::default());
    let mut l = vec![0.1f32; 512];
    let mut r = l.clone();
    let sc = vec![0.5f32; 10]; // host gave a short aux buffer
    e.process(&mut l, &mut r, Some((&sc, &sc)));
    assert!(l.iter().all(|v| v.is_finite()));
}

#[test]
fn long_silence_does_not_spike_the_cpu_with_denormals() {
    let fs = 48_000.0;
    let mut e = Engine::new(fs);
    let mut bands = [BandSettings::default(); BAND_COUNT];
    for (i, b) in bands.iter_mut().enumerate() {
        *b = BandSettings { used: true, ftype: if i % 2 == 0 { FilterType::LowCut } else { FilterType::Bell }, freq: 40.0 + 30.0 * i as f32, gain_db: 12.0, q: 8.0, slope: 96, dynamic: i % 2 == 1, ..Default::default() };
    }
    e.set_settings(&bands, GlobalSettings::default());
    let mut burst = vec![0.5f32; 4800];
    let mut b2 = burst.clone();
    e.process(&mut burst, &mut b2, None);
    // ring out for 20 s of digital silence: the filters decay into denormal territory
    let t = std::time::Instant::now();
    for _ in 0..(20 * 48_000 / 512) {
        let mut a = vec![0.0f32; 512];
        let mut b = vec![0.0f32; 512];
        e.process(&mut a, &mut b, None);
    }
    let per_second = t.elapsed().as_secs_f64() / 20.0;
    if !cfg!(debug_assertions) {
        assert!(per_second < 0.05, "silence cost {:.1}% of real time (denormals?)", per_second * 100.0);
    }
}

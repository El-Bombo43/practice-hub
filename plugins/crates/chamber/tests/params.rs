use chamber::dsp::{Settings, Engine};
use chamber::params::ChamberParams;
use chamber::presets::PRESETS;
use nih_plug::prelude::*;

/// Parameter IDs are permanent once the plugin is used in saved sets: this list is the contract from SPEC.md.
const SPEC_IDS: [&str; 21] = [
    "mode", "mix", "predelay", "predelay_sync", "predelay_div", "size", "decay", "diffusion", "early", "low_cut", "high_cut", "damping",
    "bass_decay", "mod_rate", "mod_depth", "width", "duck_amt", "duck_thresh", "duck_release", "freeze", "out_gain",
];

#[test]
fn parameter_ids_match_the_spec() {
    let p = ChamberParams::default();
    let mut ids: Vec<String> = p.param_map().into_iter().map(|(id, _, _)| id).collect();
    ids.sort();
    let mut want: Vec<String> = SPEC_IDS.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(ids, want);
}

fn shown(p: &ChamberParams, id: &str) -> String {
    let map = p.param_map();
    let (_, ptr, _) = map.iter().find(|(i, _, _)| i == id).unwrap();
    unsafe { ptr.normalized_value_to_string(ptr.default_normalized_value(), true) }
}

#[test]
fn defaults_and_display_formats_follow_the_spec() {
    let p = ChamberParams::default();
    let want = [
        ("mode", "Plate"), ("mix", "30 %"), ("predelay", "10.0 ms"), ("predelay_sync", "Off"), ("predelay_div", "1/32"), ("size", "50 %"), ("decay", "2.00 s"),
        ("diffusion", "75 %"), ("early", "0.0 dB"), ("low_cut", "120 Hz"), ("high_cut", "12.0 kHz"), ("damping", "7.00 kHz"), ("bass_decay", "1.00×"),
        ("mod_rate", "0.60 Hz"), ("mod_depth", "25 %"), ("width", "100 %"), ("duck_amt", "Off"), ("duck_thresh", "-30.0 dB"), ("duck_release", "400 ms"),
        ("freeze", "Off"), ("out_gain", "0.0 dB"),
    ];
    for (id, text) in want {
        assert_eq!(shown(&p, id), text, "{id}");
    }
}

#[test]
fn typed_values_parse_with_or_without_units() {
    let p = ChamberParams::default();
    assert_eq!(p.high_cut.string_to_normalized_value("9k").map(|n| p.high_cut.preview_plain(n).round()), Some(9000.0));
    assert_eq!(p.decay.string_to_normalized_value("350 ms").map(|n| (p.decay.preview_plain(n) * 100.0).round()), Some(35.0));
    assert_eq!(p.duck_amt.string_to_normalized_value("off").map(|n| p.duck_amt.preview_plain(n)), Some(0.0));
    assert_eq!(p.mix.string_to_normalized_value("45%").map(|n| p.mix.preview_plain(n).round()), Some(45.0));
}

#[test]
fn defaults_equal_the_engine_defaults_and_the_init_preset() {
    let a = ChamberParams::default().settings(120.0);
    let b = Settings::default();
    assert_eq!((a.mode, a.mix, a.predelay_ms, a.size, a.decay_s, a.low_cut, a.high_cut, a.damping_hz, a.bass_decay, a.width), (b.mode, b.mix, b.predelay_ms, b.size, b.decay_s, b.low_cut, b.high_cut, b.damping_hz, b.bass_decay, b.width));
    let init = (PRESETS[0].settings)();
    assert_eq!((init.mix, init.decay_s, init.mode), (b.mix, b.decay_s, b.mode));
}

#[test]
fn every_preset_runs_cleanly() {
    for preset in PRESETS {
        let s = (preset.settings)();
        let mut e = Engine::new(48_000.0);
        let mut l: Vec<f32> = (0..48_000).map(|i| 0.5 * (i as f32 * 0.05).sin()).collect();
        let mut r = l.clone();
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            e.set_settings(&s);
            e.process(a, b);
        }
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 4.0), "{}", preset.name);
    }
}

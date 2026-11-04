//! The parameter IDs and names are a contract with saved Ableton sets: they must never change.
use contour::params::ContourParams;
use nih_plug::prelude::*;

fn map() -> Vec<(String, String)> {
    let p = ContourParams::default();
    p.param_map().into_iter().map(|(id, ptr, _)| (id, unsafe { ptr.name().to_string() })).collect()
}

#[test]
fn ids_and_names_match_the_spec() {
    let m = map();
    let has = |id: &str, name: &str| assert!(m.iter().any(|(i, n)| i == id && n == name), "missing {id} / {name}");
    for n in [1usize, 2, 9, 16] {
        has(&format!("b{n}_used"), &format!("{n} Used"));
        has(&format!("b{n}_on"), &format!("{n} Enabled"));
        has(&format!("b{n}_type"), &format!("{n} Type"));
        has(&format!("b{n}_freq"), &format!("{n} Freq"));
        has(&format!("b{n}_gain"), &format!("{n} Gain"));
        has(&format!("b{n}_q"), &format!("{n} Q"));
        has(&format!("b{n}_slope"), &format!("{n} Slope"));
        has(&format!("b{n}_dyn"), &format!("{n} Dynamic"));
        has(&format!("b{n}_thresh"), &format!("{n} Threshold"));
        has(&format!("b{n}_range"), &format!("{n} Range"));
        has(&format!("b{n}_attack"), &format!("{n} Attack"));
        has(&format!("b{n}_release"), &format!("{n} Release"));
        has(&format!("b{n}_ext"), &format!("{n} Ext Sidechain"));
    }
    has("out_gain", "Output");
    has("auto_gain", "Auto Gain");
    has("gain_scale", "Gain Scale");
    assert_eq!(m.len(), 16 * 13 + 3);
    let mut ids: Vec<_> = m.iter().map(|(i, _)| i.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 16 * 13 + 3, "duplicate IDs");
}

#[test]
fn defaults_and_ranges_match_the_spec() {
    let p = ContourParams::default();
    let b = p.band(0);
    let s = b.settings();
    assert!(!s.used && s.enabled && !s.dynamic && !s.ext);
    assert_eq!((s.freq, s.gain_db, s.q, s.slope), (1000.0, 0.0, 1.0, 12));
    assert_eq!((s.thresh_db, s.range_db, s.attack_ms, s.release_ms), (-20.0, -6.0, 10.0, 150.0));
    // ends of the ranges
    assert_eq!(b.freq.preview_plain(0.0), 10.0);
    assert!((b.freq.preview_plain(1.0) - 30_000.0).abs() < 1.0);
    assert!((b.q.preview_plain(0.0) - 0.025).abs() < 1e-6 && (b.q.preview_plain(1.0) - 40.0).abs() < 1e-3);
    assert!((b.freq.preview_plain(0.5) - 547.7).abs() < 15.0, "log-ish mid {}", b.freq.preview_plain(0.5));
    assert_eq!(p.out_gain.value(), 0.0);
    assert_eq!(p.global_settings().gain_scale, 1.0);
    // display strings from the spec
    assert_eq!(b.freq.normalized_value_to_string(b.freq.preview_normalized(82.4), true), "82.4 Hz");
    assert_eq!(b.freq.normalized_value_to_string(b.freq.preview_normalized(1250.0), true), "1.25 kHz");
    assert_eq!(b.gain.normalized_value_to_string(b.gain.preview_normalized(-3.5), true), "-3.50 dB");
    assert_eq!(b.q.normalized_value_to_string(b.q.preview_normalized(1.0), true), "Q 1.00");
    assert_eq!(b.slope.value().db_per_oct(), 12);
}

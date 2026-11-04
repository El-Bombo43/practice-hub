use nih_plug::prelude::*;
use snap::params::{KeyParam, ScaleParam, SnapParams};
use snap::scale::{decide, mask_for, nearest_enabled};

#[test]
fn ids_and_defaults_follow_the_spec() {
    let p = SnapParams::default();
    let map = p.param_map();
    let mut ids: Vec<String> = map.iter().map(|(i, _, _)| i.clone()).collect();
    ids.sort();
    let mut want: Vec<String> = ["mode", "voice", "key", "scale", "n_c", "n_cs", "n_d", "n_ds", "n_e", "n_f", "n_fs", "n_g", "n_gs", "n_a", "n_as", "n_b", "retune", "humanize", "flex", "amount", "mix", "out_gain"].iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(ids, want);
    let shown = |id: &str| {
        let (_, ptr, _) = map.iter().find(|(i, _, _)| i == id).unwrap();
        unsafe { ptr.normalized_value_to_string(ptr.default_normalized_value(), true) }
    };
    for (id, t) in [("mode", "Mix"), ("voice", "Mid"), ("key", "C"), ("scale", "Major"), ("retune", "50 ms"), ("humanize", "20 %"), ("flex", "50 %"), ("amount", "100 %"), ("mix", "100 %"), ("out_gain", "0.0 dB")] {
        assert_eq!(shown(id), t, "{id}");
    }
    // default toggles are C major
    assert_eq!(p.toggles(), mask_for(0, 1));
    let mode = map.iter().find(|(i, _, _)| i == "mode").unwrap().1;
    assert!(unsafe { mode.flags() }.contains(ParamFlags::NON_AUTOMATABLE), "Mode is not automatable");
}

#[test]
fn a_named_scale_wins_and_custom_uses_the_toggles() {
    let e_minor = SnapParams::from_settings(&Default::default(), KeyParam::E, ScaleParam::NaturalMinor);
    assert_eq!(e_minor.notes(), mask_for(4, 2));
    assert_eq!(e_minor.notes().iter().filter(|b| **b).count(), 7);
    // Custom: the toggles (here C major, the default) are the truth whatever the key says
    let custom = SnapParams::from_settings(&Default::default(), KeyParam::E, ScaleParam::Custom);
    assert_eq!(custom.notes(), mask_for(0, 1));
    assert_eq!(mask_for(3, 0), [true; 12]);
}

#[test]
fn nearest_note_and_hysteresis() {
    let c_major = mask_for(0, 1);
    assert_eq!(nearest_enabled(67.95, &c_major), Some(67));
    assert_eq!(nearest_enabled(68.2, &c_major), Some(69));
    assert_eq!(nearest_enabled(97.9, &c_major), Some(98)); // any octave
    assert_eq!(nearest_enabled(60.0, &[false; 12]), None);
    // holding G (67): A (69) is chosen only once the pitch is 15 cents past the 68.0 midpoint
    assert_eq!(decide(68.10, Some(67), &c_major, 0.15), Some(67));
    assert_eq!(decide(68.14, Some(67), &c_major, 0.15), Some(67));
    assert_eq!(decide(68.40, Some(67), &c_major, 0.15), Some(69));
}

use nih_plug::prelude::*;
use vox_strip::params::VoxParams;

const IDS: [&str; 30] = [
    "in_gain", "gate_on", "gate_thresh", "gate_range", "gate_attack", "gate_release", "hpf_on", "hpf_freq", "hpf_slope", "ds_on", "ds_freq", "ds_thresh", "ds_range", "ds_mode",
    "comp_on", "comp_style", "comp_thresh", "comp_ratio", "comp_attack", "comp_release", "comp_knee", "comp_makeup", "comp_auto", "sat_on", "sat_type", "sat_drive", "sat_mix",
    "out_gain", "mix", "xx",
];

#[test]
fn ids_and_default_displays_follow_the_spec() {
    let p = VoxParams::default();
    let map = p.param_map();
    let mut ids: Vec<String> = map.iter().map(|(i, _, _)| i.clone()).collect();
    ids.sort();
    let mut want: Vec<String> = IDS[..29].iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(ids, want);
    let shown = |id: &str| {
        let (_, ptr, _) = map.iter().find(|(i, _, _)| i == id).unwrap();
        unsafe { ptr.normalized_value_to_string(ptr.default_normalized_value(), true) }
    };
    for (id, text) in [
        ("in_gain", "0.0 dB"), ("gate_thresh", "-50.0 dB"), ("gate_range", "20.0 dB"), ("gate_attack", "1.0 ms"), ("gate_release", "120 ms"), ("hpf_freq", "80 Hz"), ("hpf_slope", "12 dB/oct"),
        ("ds_freq", "6.50 kHz"), ("ds_thresh", "-30.0 dB"), ("ds_range", "6.0 dB"), ("ds_mode", "Split-band"), ("comp_style", "Clean"), ("comp_thresh", "-18.0 dB"), ("comp_ratio", "3.0:1"),
        ("comp_attack", "10.0 ms"), ("comp_release", "120 ms"), ("comp_knee", "6.0 dB"), ("comp_makeup", "0.0 dB"), ("sat_type", "Tape"), ("sat_drive", "20 %"), ("sat_mix", "100 %"), ("out_gain", "0.0 dB"), ("mix", "100 %"),
    ] {
        assert_eq!(shown(id), text, "{id}");
    }
}

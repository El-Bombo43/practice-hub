use nih_plug::prelude::*;
use punch::params::PunchParams;

#[test]
fn ids_and_default_displays_follow_the_spec() {
    let p = PunchParams::default();
    let map = p.param_map();
    let mut ids: Vec<&str> = map.iter().map(|(i, _, _)| i.as_str()).collect();
    ids.sort();
    let mut want = vec!["attack", "sustain", "attack_len", "sustain_len", "focus", "link", "clip", "out_gain", "mix"];
    want.sort();
    assert_eq!(ids, want);
    let shown = |id: &str| {
        let (_, ptr, _) = map.iter().find(|(i, _, _)| i == id).unwrap();
        unsafe { ptr.normalized_value_to_string(ptr.default_normalized_value(), true) }
    };
    for (id, t) in [("attack", "+0 %"), ("attack_len", "10.0 ms"), ("sustain_len", "150 ms"), ("focus", "Off"), ("link", "On"), ("clip", "Off"), ("out_gain", "0.0 dB"), ("mix", "100 %")] {
        assert_eq!(shown(id), t, "{id}");
    }
}

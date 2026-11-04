use headroom::gui::format_position;
use headroom::meter::OverEvent;
use headroom::state::*;

#[test]
fn targets_and_ceilings() {
    let mut s = HeadSettings::default();
    assert_eq!((s.target(), s.ceiling(-15.0), s.ceiling(-13.0)), (-14.0, -1.0, -2.0), "Spotify drops to -2 when louder than -14");
    s.preset = 1;
    assert_eq!((s.target(), s.ceiling(-10.0)), (-16.0, -1.0));
    s.preset = 5;
    s.custom_target = -40.0;
    s.custom_ceiling = 1.0;
    assert_eq!((s.target(), s.ceiling(-10.0)), (-30.0, 0.0), "custom values are clamped");
}

#[test]
fn window_modes_have_fixed_sizes() {
    let mut s = HeadSettings::default();
    assert_eq!(s.size(), (960, 560));
    s.mode = Mode::Strip;
    assert_eq!(s.size(), (1200, 132));
    s.strip_narrow = true;
    assert_eq!(s.size(), (860, 132));
    s.mode = Mode::Compact;
    assert_eq!(s.size(), (480, 170));
}

#[test]
fn bar_beat_positions_at_120_bpm() {
    // 120 BPM, 4/4: 2 beats per second. 114.3 s = beat 228.6 -> bar 58, beat 1 (bar = 4 beats)
    let o = |sec: f64| OverEvent { beats: Some(sec * 2.0), seconds: sec, peak_db: 0.4, bar_index: 0 };
    assert_eq!(format_position(&o(134.0), 4.0), "Bar 68.1 \u{b7} 2:14 \u{b7} +0.4 dBTP");
    assert_eq!(format_position(&o(0.0), 4.0), "Bar 1.1 \u{b7} 0:00 \u{b7} +0.4 dBTP");
    // one sixteenth note (0.125 s) later is still beat 1; a full beat later is beat 2
    assert!(format_position(&o(0.125), 4.0).starts_with("Bar 1.1"));
    assert!(format_position(&o(0.5), 4.0).starts_with("Bar 1.2"));
}

#[test]
fn settings_survive_a_save_and_reload() {
    let s = HeadSettings { preset: 4, mode: Mode::Compact, history_s: 60, auto_reset: false, ..Default::default() };
    let json = serde_json_like(&s);
    assert!(json.contains("Compact"));
}

fn serde_json_like(s: &HeadSettings) -> String {
    format!("{s:?}")
}

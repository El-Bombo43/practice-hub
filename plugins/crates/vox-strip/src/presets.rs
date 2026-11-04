use crate::dsp::Settings;

pub struct Preset {
    pub name: &'static str,
    pub settings: fn() -> Settings,
}

fn d() -> Settings {
    Settings::default()
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Init", settings: d },
    Preset { name: "Rock Lead", settings: || Settings { on: [false, true, true, true, true], hpf_freq: 100.0, ds_freq: 6500.0, ds_thresh: -30.0, ds_range: 6.0, comp_opto: true, comp_thresh: -20.0, comp_ratio: 4.0, sat_tube: false, sat_drive: 0.25, ..d() } },
    Preset { name: "Screams", settings: || Settings { on: [true, true, true, true, true], gate_thresh: -45.0, gate_range: 30.0, hpf_freq: 120.0, ds_freq: 5500.0, ds_thresh: -28.0, ds_range: 8.0, comp_opto: false, comp_thresh: -24.0, comp_ratio: 8.0, comp_attack_ms: 2.0, comp_release_ms: 60.0, sat_tube: true, sat_drive: 0.45, mix: 0.85, ..d() } },
    Preset { name: "Clean Ballad", settings: || Settings { on: [false, true, true, true, false], hpf_freq: 80.0, ds_freq: 7000.0, ds_thresh: -32.0, ds_range: 5.0, comp_opto: true, comp_thresh: -16.0, comp_ratio: 2.5, ..d() } },
    Preset { name: "Backing Vocals", settings: || Settings { on: [false, true, true, true, true], hpf_freq: 150.0, ds_freq: 6500.0, ds_thresh: -34.0, ds_range: 10.0, comp_opto: false, comp_thresh: -22.0, comp_ratio: 6.0, sat_tube: false, sat_drive: 0.15, ..d() } },
];

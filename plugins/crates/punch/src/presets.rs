use crate::dsp::Settings;

pub struct Preset {
    pub name: &'static str,
    pub settings: fn() -> Settings,
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Init", settings: Settings::default },
    Preset { name: "Snappy Kick", settings: || Settings { attack: 0.60, sustain: -0.20, attack_len_ms: 8.0, focus: 80.0, ..Settings::default() } },
    Preset { name: "Tight Palm Mutes", settings: || Settings { attack: 0.25, sustain: -0.45, sustain_len_ms: 120.0, focus: 150.0, ..Settings::default() } },
    Preset { name: "Room Killer (drums)", settings: || Settings { attack: 0.10, sustain: -0.70, sustain_len_ms: 200.0, ..Settings::default() } },
    Preset { name: "Fat Snare", settings: || Settings { attack: 0.30, sustain: 0.40, sustain_len_ms: 250.0, ..Settings::default() } },
    Preset { name: "Soften Pick", settings: || Settings { attack: -0.40, sustain: 0.0, focus: 300.0, ..Settings::default() } },
];

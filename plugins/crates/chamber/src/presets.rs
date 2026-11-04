//! Factory presets, as engine settings (everything not listed is the default).
use crate::dsp::Settings;

pub struct Preset {
    pub name: &'static str,
    pub settings: fn() -> Settings,
}

fn init() -> Settings {
    Settings::default()
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Init", settings: init },
    Preset { name: "Vocal Plate", settings: || Settings { mode: 0, mix: 0.25, predelay_sync: true, predelay_div: 2, decay_s: 1.8, low_cut: 200.0, high_cut: 9000.0, damping_hz: 6000.0, mod_depth: 0.20, duck_db: 6.0, duck_thresh_db: -30.0, duck_release_ms: 350.0, ..init() } },
    Preset { name: "Lead Hall (ducked)", settings: || Settings { mode: 1, mix: 0.35, predelay_ms: 40.0, size: 0.75, decay_s: 3.5, low_cut: 250.0, high_cut: 8000.0, damping_hz: 5000.0, bass_decay: 0.8, mod_depth: 0.35, duck_db: 12.0, duck_thresh_db: -36.0, duck_release_ms: 600.0, ..init() } },
    Preset { name: "Drum Room", settings: || Settings { mode: 2, mix: 0.20, predelay_ms: 0.0, size: 0.30, decay_s: 0.7, early_db: 2.0, low_cut: 80.0, high_cut: 14_000.0, damping_hz: 9000.0, mod_depth: 0.10, width: 1.10, ..init() } },
    Preset { name: "Clean Guitar Wash", settings: || Settings { mode: 1, mix: 0.45, predelay_ms: 20.0, size: 0.90, decay_s: 6.0, low_cut: 300.0, high_cut: 7000.0, damping_hz: 4500.0, mod_rate: 0.3, mod_depth: 0.60, width: 1.30, ..init() } },
    Preset { name: "Return 100%", settings: || Settings { mode: 0, mix: 1.0, ..init() } },
];

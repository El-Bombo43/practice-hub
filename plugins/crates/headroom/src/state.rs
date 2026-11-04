//! Saved settings (non-automatable state, so nothing clutters Ableton's automation list) and the targets table.
use serde::{Deserialize, Serialize};

/// (name, target LUFS, true-peak ceiling dBTP) as published by the services, Nov 2026. Edit here when they change.
pub const TARGETS: [(&str, f32, f32); 6] = [
    ("Spotify", -14.0, -1.0),
    ("Apple Music", -16.0, -1.0),
    ("YouTube", -14.0, -1.0),
    ("Tidal / Amazon", -14.0, -1.0),
    ("Deezer", -15.0, -1.0),
    ("Custom", -14.0, -1.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Full,
    Strip,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadSettings {
    pub preset: usize,
    pub custom_target: f32,
    pub custom_ceiling: f32,
    pub mode: Mode,
    pub strip_narrow: bool,
    pub history_s: u32,
    pub auto_reset: bool,
    pub measure_stopped: bool,
}

impl Default for HeadSettings {
    fn default() -> Self {
        Self { preset: 0, custom_target: -14.0, custom_ceiling: -1.0, mode: Mode::Full, strip_narrow: false, history_s: 30, auto_reset: true, measure_stopped: false }
    }
}

impl HeadSettings {
    pub fn target(&self) -> f32 {
        if self.preset >= 5 { self.custom_target.clamp(-30.0, -6.0) } else { TARGETS[self.preset].1 }
    }
    /// The true-peak ceiling in force. Spotify asks for -2 dBTP when the master is louder than its -14 target.
    pub fn ceiling(&self, integrated: f32) -> f32 {
        if self.preset >= 5 {
            self.custom_ceiling.clamp(-3.0, 0.0)
        } else if self.preset == 0 && integrated.is_finite() && integrated > -14.0 {
            -2.0
        } else {
            TARGETS[self.preset].2
        }
    }
    pub fn name(&self) -> &'static str {
        TARGETS[self.preset.min(5)].0
    }
    pub fn size(&self) -> (u32, u32) {
        match self.mode {
            Mode::Full => (960, 560),
            Mode::Strip => if self.strip_narrow { (860, 132) } else { (1200, 132) },
            Mode::Compact => (480, 170),
        }
    }
}

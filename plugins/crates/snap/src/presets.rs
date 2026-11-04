use crate::params::{KeyParam, ScaleParam};

pub struct Preset {
    pub name: &'static str,
    pub key: KeyParam,
    pub scale: ScaleParam,
    pub retune: f32,
    pub humanize: f32,
    pub flex: f32,
    pub amount: f32,
    /// `Some(true)` forces Track, `Some(false)` Mix, `None` leaves the mode alone
    pub track: Option<bool>,
}

pub const PRESETS: &[Preset] = &[
    Preset { name: "Natural - E minor", key: KeyParam::E, scale: ScaleParam::NaturalMinor, retune: 80.0, humanize: 40.0, flex: 70.0, amount: 100.0, track: None },
    Preset { name: "Natural - Chromatic cleanup", key: KeyParam::C, scale: ScaleParam::Chromatic, retune: 120.0, humanize: 50.0, flex: 80.0, amount: 70.0, track: None },
    Preset { name: "Hard Snap - E minor", key: KeyParam::E, scale: ScaleParam::NaturalMinor, retune: 0.0, humanize: 0.0, flex: 0.0, amount: 100.0, track: None },
    Preset { name: "Track Monitor", key: KeyParam::C, scale: ScaleParam::Chromatic, retune: 60.0, humanize: 20.0, flex: 50.0, amount: 100.0, track: Some(true) },
];

//! Parameters. IDs are final (see the spec): `b{n}_used`, `b{n}_on`, ... and the three globals.

use crate::dsp::{BandSettings, FilterType, GlobalSettings, BAND_COUNT};
use crate::fmt::*;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeParam {
    #[id = "bell"]
    #[name = "Bell"]
    Bell,
    #[id = "low_shelf"]
    #[name = "Low Shelf"]
    LowShelf,
    #[id = "high_shelf"]
    #[name = "High Shelf"]
    HighShelf,
    #[id = "tilt_shelf"]
    #[name = "Tilt Shelf"]
    TiltShelf,
    #[id = "low_cut"]
    #[name = "Low Cut"]
    LowCut,
    #[id = "high_cut"]
    #[name = "High Cut"]
    HighCut,
    #[id = "notch"]
    #[name = "Notch"]
    Notch,
    #[id = "band_pass"]
    #[name = "Band Pass"]
    BandPass,
}

impl TypeParam {
    pub const ALL: [TypeParam; 8] = [
        Self::Bell, Self::LowShelf, Self::HighShelf, Self::TiltShelf, Self::LowCut, Self::HighCut, Self::Notch, Self::BandPass,
    ];
    pub fn filter(self) -> FilterType {
        match self {
            Self::Bell => FilterType::Bell,
            Self::LowShelf => FilterType::LowShelf,
            Self::HighShelf => FilterType::HighShelf,
            Self::TiltShelf => FilterType::TiltShelf,
            Self::LowCut => FilterType::LowCut,
            Self::HighCut => FilterType::HighCut,
            Self::Notch => FilterType::Notch,
            Self::BandPass => FilterType::BandPass,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Bell => "Bell",
            Self::LowShelf => "Low Shelf",
            Self::HighShelf => "High Shelf",
            Self::TiltShelf => "Tilt Shelf",
            Self::LowCut => "Low Cut",
            Self::HighCut => "High Cut",
            Self::Notch => "Notch",
            Self::BandPass => "Band Pass",
        }
    }
}

#[derive(Enum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlopeParam {
    #[id = "s6"]
    #[name = "6 dB/oct"]
    S6,
    #[id = "s12"]
    #[name = "12 dB/oct"]
    S12,
    #[id = "s18"]
    #[name = "18 dB/oct"]
    S18,
    #[id = "s24"]
    #[name = "24 dB/oct"]
    S24,
    #[id = "s36"]
    #[name = "36 dB/oct"]
    S36,
    #[id = "s48"]
    #[name = "48 dB/oct"]
    S48,
    #[id = "s72"]
    #[name = "72 dB/oct"]
    S72,
    #[id = "s96"]
    #[name = "96 dB/oct"]
    S96,
}

impl SlopeParam {
    pub const ALL: [SlopeParam; 8] = [Self::S6, Self::S12, Self::S18, Self::S24, Self::S36, Self::S48, Self::S72, Self::S96];
    pub fn db_per_oct(self) -> u32 {
        match self {
            Self::S6 => 6,
            Self::S12 => 12,
            Self::S18 => 18,
            Self::S24 => 24,
            Self::S36 => 36,
            Self::S48 => 48,
            Self::S72 => 72,
            Self::S96 => 96,
        }
    }
    pub fn from_db_per_oct(v: u32) -> Self {
        Self::ALL.into_iter().find(|s| s.db_per_oct() == v).unwrap_or(Self::S12)
    }
}

/// A skewed range whose midpoint is the geometric mean of the ends, which is close to a logarithmic scale.
fn log_range(min: f32, max: f32) -> FloatRange {
    let mid = (min * max).sqrt();
    let k = ((mid - min) / (max - min)).ln() / 0.5f32.ln(); // 0.5^k = normalised midpoint
    FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-(k.log2())) }
}

#[derive(Params)]
pub struct BandParams {
    #[id = "used"]
    pub used: BoolParam,
    #[id = "on"]
    pub on: BoolParam,
    #[id = "type"]
    pub ftype: EnumParam<TypeParam>,
    #[id = "freq"]
    pub freq: FloatParam,
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "q"]
    pub q: FloatParam,
    #[id = "slope"]
    pub slope: EnumParam<SlopeParam>,
    #[id = "dyn"]
    pub dynamic: BoolParam,
    #[id = "thresh"]
    pub thresh: FloatParam,
    #[id = "range"]
    pub range: FloatParam,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
    #[id = "ext"]
    pub ext: BoolParam,
}

impl BandParams {
    pub fn new(n: usize) -> Self {
        let db = |name: &str, default: f32, min: f32, max: f32| {
            FloatParam::new(format!("{n} {name}"), default, FloatRange::Linear { min, max })
                .with_step_size(0.01)
                .with_value_to_string(Arc::new(|v| fmt_db(v, 2)))
                .with_string_to_value(Arc::new(|s| parse_db(s)))
        };
        Self {
            used: BoolParam::new(format!("{n} Used"), false),
            on: BoolParam::new(format!("{n} Enabled"), true),
            ftype: EnumParam::new(format!("{n} Type"), TypeParam::Bell),
            freq: FloatParam::new(format!("{n} Freq"), 1000.0, log_range(10.0, 30_000.0))
                .with_value_to_string(Arc::new(fmt_freq))
                .with_string_to_value(Arc::new(parse_freq)),
            gain: db("Gain", 0.0, -30.0, 30.0),
            q: FloatParam::new(format!("{n} Q"), 1.0, log_range(0.025, 40.0))
                .with_value_to_string(Arc::new(fmt_q))
                .with_string_to_value(Arc::new(parse_q)),
            slope: EnumParam::new(format!("{n} Slope"), SlopeParam::S12),
            dynamic: BoolParam::new(format!("{n} Dynamic"), false),
            thresh: FloatParam::new(format!("{n} Threshold"), -20.0, FloatRange::Linear { min: -60.0, max: 0.0 })
                .with_value_to_string(Arc::new(|v| fmt_db(v, 1)))
                .with_string_to_value(Arc::new(|s| parse_db(s))),
            range: FloatParam::new(format!("{n} Range"), -6.0, FloatRange::Linear { min: -30.0, max: 30.0 })
                .with_value_to_string(Arc::new(|v| fmt_db(v, 1)))
                .with_string_to_value(Arc::new(|s| parse_db(s))),
            attack: FloatParam::new(format!("{n} Attack"), 10.0, log_range(0.5, 200.0))
                .with_value_to_string(Arc::new(fmt_ms))
                .with_string_to_value(Arc::new(parse_ms)),
            release: FloatParam::new(format!("{n} Release"), 150.0, log_range(10.0, 2000.0))
                .with_value_to_string(Arc::new(fmt_ms))
                .with_string_to_value(Arc::new(parse_ms)),
            ext: BoolParam::new(format!("{n} Ext Sidechain"), false),
        }
    }

    pub fn settings(&self) -> BandSettings {
        BandSettings {
            used: self.used.value(),
            enabled: self.on.value(),
            ftype: self.ftype.value().filter(),
            freq: self.freq.value(),
            gain_db: self.gain.value(),
            q: self.q.value(),
            slope: self.slope.value().db_per_oct(),
            dynamic: self.dynamic.value(),
            thresh_db: self.thresh.value(),
            range_db: self.range.value(),
            attack_ms: self.attack.value(),
            release_ms: self.release.value(),
            ext: self.ext.value(),
        }
    }
}

/// Editor-only state saved with the project (not automatable). Band solo is deliberately not here.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuiPrefs {
    pub selected: Option<usize>,
    /// 0 = +-3 dB, 1 = +-6, 2 = +-12, 3 = +-30
    pub range_idx: u8,
    /// 0 = Off, 1 = Post, 2 = Pre, 3 = Pre+Post
    pub analyzer_mode: u8,
    /// 0 = Slow, 1 = Medium, 2 = Fast
    pub analyzer_speed: u8,
}

impl Default for GuiPrefs {
    fn default() -> Self {
        Self { selected: None, range_idx: 2, analyzer_mode: 1, analyzer_speed: 1 }
    }
}

#[derive(Params)]
pub struct ContourParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[persist = "gui-prefs"]
    pub prefs: Arc<RwLock<GuiPrefs>>,

    #[id = "out_gain"]
    pub out_gain: FloatParam,
    #[id = "auto_gain"]
    pub auto_gain: BoolParam,
    #[id = "gain_scale"]
    pub gain_scale: FloatParam,

    #[nested(id_prefix = "b1", group = "Band 1")]
    pub b1: BandParams,
    #[nested(id_prefix = "b2", group = "Band 2")]
    pub b2: BandParams,
    #[nested(id_prefix = "b3", group = "Band 3")]
    pub b3: BandParams,
    #[nested(id_prefix = "b4", group = "Band 4")]
    pub b4: BandParams,
    #[nested(id_prefix = "b5", group = "Band 5")]
    pub b5: BandParams,
    #[nested(id_prefix = "b6", group = "Band 6")]
    pub b6: BandParams,
    #[nested(id_prefix = "b7", group = "Band 7")]
    pub b7: BandParams,
    #[nested(id_prefix = "b8", group = "Band 8")]
    pub b8: BandParams,
    #[nested(id_prefix = "b9", group = "Band 9")]
    pub b9: BandParams,
    #[nested(id_prefix = "b10", group = "Band 10")]
    pub b10: BandParams,
    #[nested(id_prefix = "b11", group = "Band 11")]
    pub b11: BandParams,
    #[nested(id_prefix = "b12", group = "Band 12")]
    pub b12: BandParams,
    #[nested(id_prefix = "b13", group = "Band 13")]
    pub b13: BandParams,
    #[nested(id_prefix = "b14", group = "Band 14")]
    pub b14: BandParams,
    #[nested(id_prefix = "b15", group = "Band 15")]
    pub b15: BandParams,
    #[nested(id_prefix = "b16", group = "Band 16")]
    pub b16: BandParams,
}

impl Default for ContourParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(960, 560),
            prefs: Arc::new(RwLock::new(GuiPrefs::default())),
            out_gain: FloatParam::new("Output", 0.0, FloatRange::Linear { min: -24.0, max: 24.0 })
                .with_step_size(0.01)
                .with_value_to_string(Arc::new(|v| fmt_db(v, 1)))
                .with_string_to_value(Arc::new(|s| parse_db(s))),
            auto_gain: BoolParam::new("Auto Gain", false),
            gain_scale: FloatParam::new("Gain Scale", 100.0, FloatRange::Linear { min: 0.0, max: 200.0 })
                .with_value_to_string(Arc::new(fmt_percent))
                .with_string_to_value(Arc::new(parse_percent)),
            b1: BandParams::new(1),
            b2: BandParams::new(2),
            b3: BandParams::new(3),
            b4: BandParams::new(4),
            b5: BandParams::new(5),
            b6: BandParams::new(6),
            b7: BandParams::new(7),
            b8: BandParams::new(8),
            b9: BandParams::new(9),
            b10: BandParams::new(10),
            b11: BandParams::new(11),
            b12: BandParams::new(12),
            b13: BandParams::new(13),
            b14: BandParams::new(14),
            b15: BandParams::new(15),
            b16: BandParams::new(16),
        }
    }
}

impl ContourParams {
    pub fn band(&self, i: usize) -> &BandParams {
        self.bands()[i]
    }

    pub fn bands(&self) -> [&BandParams; BAND_COUNT] {
        [&self.b1, &self.b2, &self.b3, &self.b4, &self.b5, &self.b6, &self.b7, &self.b8, &self.b9, &self.b10, &self.b11, &self.b12, &self.b13, &self.b14, &self.b15, &self.b16]
    }

    pub fn band_settings(&self) -> [BandSettings; BAND_COUNT] {
        let b = self.bands();
        std::array::from_fn(|i| b[i].settings())
    }

    pub fn global_settings(&self) -> GlobalSettings {
        GlobalSettings { out_gain_db: self.out_gain.value(), auto_gain: self.auto_gain.value(), gain_scale: self.gain_scale.value() / 100.0 }
    }
}

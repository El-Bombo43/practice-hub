use crate::fmt::*;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeParam {
    #[id = "plate"]
    #[name = "Plate"]
    Plate,
    #[id = "hall"]
    #[name = "Hall"]
    Hall,
    #[id = "room"]
    #[name = "Room"]
    Room,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DivParam {
    #[id = "d64"]
    #[name = "1/64"]
    D64,
    #[id = "d32t"]
    #[name = "1/32 T"]
    D32T,
    #[id = "d32"]
    #[name = "1/32"]
    D32,
    #[id = "d16t"]
    #[name = "1/16 T"]
    D16T,
    #[id = "d16"]
    #[name = "1/16"]
    D16,
    #[id = "d16d"]
    #[name = "1/16 D"]
    D16D,
    #[id = "d8t"]
    #[name = "1/8 T"]
    D8T,
    #[id = "d8"]
    #[name = "1/8"]
    D8,
    #[id = "d8d"]
    #[name = "1/8 D"]
    D8D,
    #[id = "d4"]
    #[name = "1/4"]
    D4,
}

pub const SIZES: [(u32, u32); 3] = [(640, 400), (820, 512), (1100, 690)];

/// A skewed range whose midpoint is the geometric mean of the ends (close to a log scale).
pub fn log_range(min: f32, max: f32) -> FloatRange {
    let mid = (min * max).sqrt();
    let k = ((mid - min) / (max - min)).ln() / 0.5f32.ln();
    FloatRange::Skewed { min, max, factor: FloatRange::skew_factor(-(k.log2())) }
}

#[derive(Params)]
pub struct ChamberParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "predelay"]
    pub predelay: FloatParam,
    #[id = "predelay_sync"]
    pub predelay_sync: BoolParam,
    #[id = "predelay_div"]
    pub predelay_div: EnumParam<DivParam>,
    #[id = "size"]
    pub size: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "diffusion"]
    pub diffusion: FloatParam,
    #[id = "early"]
    pub early: FloatParam,
    #[id = "low_cut"]
    pub low_cut: FloatParam,
    #[id = "high_cut"]
    pub high_cut: FloatParam,
    #[id = "damping"]
    pub damping: FloatParam,
    #[id = "bass_decay"]
    pub bass_decay: FloatParam,
    #[id = "mod_rate"]
    pub mod_rate: FloatParam,
    #[id = "mod_depth"]
    pub mod_depth: FloatParam,
    #[id = "width"]
    pub width: FloatParam,
    #[id = "duck_amt"]
    pub duck_amt: FloatParam,
    #[id = "duck_thresh"]
    pub duck_thresh: FloatParam,
    #[id = "duck_release"]
    pub duck_release: FloatParam,
    #[id = "freeze"]
    pub freeze: BoolParam,
    #[id = "out_gain"]
    pub out_gain: FloatParam,
}

fn percent(name: &'static str, default: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max })
        .with_value_to_string(Arc::new(fmt_percent))
        .with_string_to_value(Arc::new(|t| parse_percent(t)))
}

impl Default for ChamberParams {
    fn default() -> Self {
        Self::from_settings(&crate::dsp::Settings::default())
    }
}

impl ChamberParams {
    /// Parameters whose defaults are the given settings (used by the offscreen renderer and tests).
    pub fn from_settings(s: &crate::dsp::Settings) -> Self {
        let div = [DivParam::D64, DivParam::D32T, DivParam::D32, DivParam::D16T, DivParam::D16, DivParam::D16D, DivParam::D8T, DivParam::D8, DivParam::D8D, DivParam::D4];
        Self {
            editor_state: EguiState::from_size(SIZES[1].0, SIZES[1].1),
            mode: EnumParam::new("Mode", [ModeParam::Plate, ModeParam::Hall, ModeParam::Room][s.mode.min(2)]),
            mix: percent("Mix", s.mix * 100.0, 100.0),
            predelay: FloatParam::new("Pre-delay", s.predelay_ms, FloatRange::Skewed { min: 0.0, max: 250.0, factor: FloatRange::skew_factor(-1.4) })
                .with_value_to_string(Arc::new(fmt_ms))
                .with_string_to_value(Arc::new(|t| parse_ms(t))),
            predelay_sync: BoolParam::new("Pre-delay Sync", s.predelay_sync),
            predelay_div: EnumParam::new("Pre-delay Note", div[s.predelay_div.min(9)]),
            size: percent("Size", s.size * 100.0, 100.0),
            decay: FloatParam::new("Decay", s.decay_s, log_range(0.1, 20.0))
                .with_value_to_string(Arc::new(fmt_s))
                .with_string_to_value(Arc::new(|t| parse_s(t))),
            diffusion: percent("Diffusion", s.diffusion * 100.0, 100.0),
            early: FloatParam::new("Early", s.early_db, FloatRange::Linear { min: -24.0, max: 6.0 })
                .with_value_to_string(Arc::new(fmt_db))
                .with_string_to_value(Arc::new(|t| parse_db(t))),
            low_cut: FloatParam::new("Low Cut", s.low_cut, log_range(20.0, 1000.0))
                .with_value_to_string(Arc::new(fmt_hz))
                .with_string_to_value(Arc::new(|t| parse_hz(t))),
            high_cut: FloatParam::new("High Cut", s.high_cut, log_range(1000.0, 20_000.0))
                .with_value_to_string(Arc::new(fmt_hz))
                .with_string_to_value(Arc::new(|t| parse_hz(t))),
            damping: FloatParam::new("Damping", s.damping_hz, log_range(1000.0, 20_000.0))
                .with_value_to_string(Arc::new(fmt_khz2))
                .with_string_to_value(Arc::new(|t| parse_hz(t))),
            bass_decay: FloatParam::new("Bass Decay", s.bass_decay, log_range(0.25, 2.0))
                .with_value_to_string(Arc::new(fmt_x))
                .with_string_to_value(Arc::new(|t| parse_x(t))),
            mod_rate: FloatParam::new("Mod Rate", s.mod_rate, log_range(0.05, 5.0))
                .with_value_to_string(Arc::new(fmt_rate))
                .with_string_to_value(Arc::new(|t| parse_hz(t))),
            mod_depth: percent("Mod Depth", s.mod_depth * 100.0, 100.0),
            width: percent("Width", s.width * 100.0, 150.0),
            duck_amt: FloatParam::new("Duck", s.duck_db, FloatRange::Linear { min: 0.0, max: 30.0 })
                .with_value_to_string(Arc::new(fmt_duck))
                .with_string_to_value(Arc::new(|t| parse_duck(t))),
            duck_thresh: FloatParam::new("Duck Threshold", s.duck_thresh_db, FloatRange::Linear { min: -60.0, max: 0.0 })
                .with_value_to_string(Arc::new(fmt_db))
                .with_string_to_value(Arc::new(|t| parse_db(t))),
            duck_release: FloatParam::new("Duck Release", s.duck_release_ms, log_range(50.0, 2000.0))
                .with_value_to_string(Arc::new(fmt_ms0))
                .with_string_to_value(Arc::new(|t| parse_ms(t))),
            freeze: BoolParam::new("Freeze", s.freeze),
            out_gain: FloatParam::new("Output", s.out_gain_db, FloatRange::Linear { min: -24.0, max: 12.0 })
                .with_value_to_string(Arc::new(fmt_db))
                .with_string_to_value(Arc::new(|t| parse_db(t))),
        }
    }
}

impl ChamberParams {
    pub fn settings(&self, tempo: f64) -> crate::dsp::Settings {
        crate::dsp::Settings {
            mode: match self.mode.value() {
                ModeParam::Plate => 0,
                ModeParam::Hall => 1,
                ModeParam::Room => 2,
            },
            mix: self.mix.value() / 100.0,
            predelay_ms: self.predelay.value(),
            predelay_sync: self.predelay_sync.value(),
            predelay_div: self.predelay_div.value() as usize,
            size: self.size.value() / 100.0,
            decay_s: self.decay.value(),
            diffusion: self.diffusion.value() / 100.0,
            early_db: self.early.value(),
            low_cut: self.low_cut.value(),
            high_cut: self.high_cut.value(),
            damping_hz: self.damping.value(),
            bass_decay: self.bass_decay.value(),
            mod_rate: self.mod_rate.value(),
            mod_depth: self.mod_depth.value() / 100.0,
            width: self.width.value() / 100.0,
            duck_db: self.duck_amt.value(),
            duck_thresh_db: self.duck_thresh.value(),
            duck_release_ms: self.duck_release.value(),
            freeze: self.freeze.value(),
            out_gain_db: self.out_gain.value(),
            tempo_bpm: tempo,
        }
    }
}

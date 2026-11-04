use crate::dsp::Settings;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use phub_common::fmt::*;
use phub_common::params::*;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlopeParam {
    #[id = "s12"]
    #[name = "12 dB/oct"]
    S12,
    #[id = "s24"]
    #[name = "24 dB/oct"]
    S24,
}
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DsMode {
    #[id = "split"]
    #[name = "Split-band"]
    Split,
    #[id = "wide"]
    #[name = "Wideband"]
    Wide,
}
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompStyle {
    #[id = "clean"]
    #[name = "Clean"]
    Clean,
    #[id = "opto"]
    #[name = "Opto"]
    Opto,
}
#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SatType {
    #[id = "tape"]
    #[name = "Tape"]
    Tape,
    #[id = "tube"]
    #[name = "Tube"]
    Tube,
}

pub const SIZES: [(u32, u32); 3] = [(760, 360), (960, 450), (1280, 600)];

#[derive(Params)]
pub struct VoxParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[id = "in_gain"]
    pub in_gain: FloatParam,
    #[id = "gate_on"]
    pub gate_on: BoolParam,
    #[id = "gate_thresh"]
    pub gate_thresh: FloatParam,
    #[id = "gate_range"]
    pub gate_range: FloatParam,
    #[id = "gate_attack"]
    pub gate_attack: FloatParam,
    #[id = "gate_release"]
    pub gate_release: FloatParam,
    #[id = "hpf_on"]
    pub hpf_on: BoolParam,
    #[id = "hpf_freq"]
    pub hpf_freq: FloatParam,
    #[id = "hpf_slope"]
    pub hpf_slope: EnumParam<SlopeParam>,
    #[id = "ds_on"]
    pub ds_on: BoolParam,
    #[id = "ds_freq"]
    pub ds_freq: FloatParam,
    #[id = "ds_thresh"]
    pub ds_thresh: FloatParam,
    #[id = "ds_range"]
    pub ds_range: FloatParam,
    #[id = "ds_mode"]
    pub ds_mode: EnumParam<DsMode>,
    #[id = "comp_on"]
    pub comp_on: BoolParam,
    #[id = "comp_style"]
    pub comp_style: EnumParam<CompStyle>,
    #[id = "comp_thresh"]
    pub comp_thresh: FloatParam,
    #[id = "comp_ratio"]
    pub comp_ratio: FloatParam,
    #[id = "comp_attack"]
    pub comp_attack: FloatParam,
    #[id = "comp_release"]
    pub comp_release: FloatParam,
    #[id = "comp_knee"]
    pub comp_knee: FloatParam,
    #[id = "comp_makeup"]
    pub comp_makeup: FloatParam,
    #[id = "comp_auto"]
    pub comp_auto: BoolParam,
    #[id = "sat_on"]
    pub sat_on: BoolParam,
    #[id = "sat_type"]
    pub sat_type: EnumParam<SatType>,
    #[id = "sat_drive"]
    pub sat_drive: FloatParam,
    #[id = "sat_mix"]
    pub sat_mix: FloatParam,
    #[id = "out_gain"]
    pub out_gain: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for VoxParams {
    fn default() -> Self {
        Self::from_settings(&Settings::default(), SatType::Tape)
    }
}

impl VoxParams {
    /// Parameters whose defaults are the given settings (presets, offscreen renders, tests).
    pub fn from_settings(s: &Settings, _t: SatType) -> Self {
        Self {
            editor_state: EguiState::from_size(SIZES[1].0, SIZES[1].1),
            in_gain: db_param("Input", s.in_gain_db, -24.0, 24.0),
            gate_on: BoolParam::new("Gate On", s.on[0]),
            gate_thresh: db_param("Gate Threshold", s.gate_thresh, -80.0, 0.0),
            gate_range: db_param("Gate Range", s.gate_range, 0.0, 80.0),
            gate_attack: ms_log_param("Gate Attack", s.gate_attack_ms, 0.1, 50.0),
            gate_release: ms_log_param("Gate Release", s.gate_release_ms, 10.0, 1000.0),
            hpf_on: BoolParam::new("HPF On", s.on[1]),
            hpf_freq: hz_log_param("HPF Freq", s.hpf_freq, 20.0, 400.0),
            hpf_slope: EnumParam::new("HPF Slope", if s.hpf_24 { SlopeParam::S24 } else { SlopeParam::S12 }),
            ds_on: BoolParam::new("De-ess On", s.on[2]),
            ds_freq: FloatParam::new("De-ess Freq", s.ds_freq, log_range(3000.0, 12_000.0)).with_value_to_string(Arc::new(fmt_khz2)).with_string_to_value(Arc::new(|t| parse_hz(t))),
            ds_thresh: db_param("De-ess Threshold", s.ds_thresh, -60.0, 0.0),
            ds_range: db_param("De-ess Range", s.ds_range, 0.0, 20.0),
            ds_mode: EnumParam::new("De-ess Mode", if s.ds_wide { DsMode::Wide } else { DsMode::Split }),
            comp_on: BoolParam::new("Comp On", s.on[3]),
            comp_style: EnumParam::new("Comp Style", if s.comp_opto { CompStyle::Opto } else { CompStyle::Clean }),
            comp_thresh: db_param("Comp Threshold", s.comp_thresh, -60.0, 0.0),
            comp_ratio: FloatParam::new("Comp Ratio", s.comp_ratio, log_range(1.0, 20.0)).with_value_to_string(Arc::new(fmt_ratio)).with_string_to_value(Arc::new(|t| parse_ratio(t))),
            comp_attack: ms_log_param("Comp Attack", s.comp_attack_ms, 0.1, 100.0),
            comp_release: ms_log_param("Comp Release", s.comp_release_ms, 10.0, 1000.0),
            comp_knee: db_param("Comp Knee", s.comp_knee, 0.0, 24.0),
            comp_makeup: db_param("Comp Makeup", s.comp_makeup, -12.0, 24.0),
            comp_auto: BoolParam::new("Auto Makeup", s.comp_auto),
            sat_on: BoolParam::new("Sat On", s.on[4]),
            sat_type: EnumParam::new("Sat Type", if s.sat_tube { SatType::Tube } else { SatType::Tape }),
            sat_drive: pct_param("Drive", s.sat_drive * 100.0, 0.0, 100.0),
            sat_mix: pct_param("Sat Mix", s.sat_mix * 100.0, 0.0, 100.0),
            out_gain: db_param("Output", s.out_gain_db, -24.0, 24.0),
            mix: pct_param("Mix", s.mix * 100.0, 0.0, 100.0),
        }
    }

    pub fn settings(&self, listen: bool) -> Settings {
        Settings {
            in_gain_db: self.in_gain.value(),
            on: [self.gate_on.value(), self.hpf_on.value(), self.ds_on.value(), self.comp_on.value(), self.sat_on.value()],
            gate_thresh: self.gate_thresh.value(),
            gate_range: self.gate_range.value(),
            gate_attack_ms: self.gate_attack.value(),
            gate_release_ms: self.gate_release.value(),
            hpf_freq: self.hpf_freq.value(),
            hpf_24: self.hpf_slope.value() == SlopeParam::S24,
            ds_freq: self.ds_freq.value(),
            ds_thresh: self.ds_thresh.value(),
            ds_range: self.ds_range.value(),
            ds_wide: self.ds_mode.value() == DsMode::Wide,
            ds_listen: listen,
            comp_opto: self.comp_style.value() == CompStyle::Opto,
            comp_thresh: self.comp_thresh.value(),
            comp_ratio: self.comp_ratio.value(),
            comp_attack_ms: self.comp_attack.value(),
            comp_release_ms: self.comp_release.value(),
            comp_knee: self.comp_knee.value(),
            comp_makeup: self.comp_makeup.value(),
            comp_auto: self.comp_auto.value(),
            sat_tube: self.sat_type.value() == SatType::Tube,
            sat_drive: self.sat_drive.value() / 100.0,
            sat_mix: self.sat_mix.value() / 100.0,
            out_gain_db: self.out_gain.value(),
            mix: self.mix.value() / 100.0,
        }
    }
}

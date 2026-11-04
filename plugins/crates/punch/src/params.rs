use crate::dsp::Settings;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use phub_common::fmt::*;
use phub_common::params::*;
use std::sync::Arc;

pub const SIZES: [(u32, u32); 3] = [(520, 320), (660, 400), (880, 540)];

#[derive(Params)]
pub struct PunchParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "attack_len"]
    pub attack_len: FloatParam,
    #[id = "sustain_len"]
    pub sustain_len: FloatParam,
    #[id = "focus"]
    pub focus: FloatParam,
    #[id = "link"]
    pub link: BoolParam,
    #[id = "clip"]
    pub clip: BoolParam,
    #[id = "out_gain"]
    pub out_gain: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for PunchParams {
    fn default() -> Self {
        Self::from_settings(&Settings::default())
    }
}

fn signed_pct(name: &'static str, v: f32) -> FloatParam {
    FloatParam::new(name, v, FloatRange::Linear { min: -100.0, max: 100.0 })
        .with_value_to_string(Arc::new(fmt_signed_pct))
        .with_string_to_value(Arc::new(|t| parse_percent(t)))
}

impl PunchParams {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            editor_state: EguiState::from_size(SIZES[1].0, SIZES[1].1),
            attack: signed_pct("Attack", s.attack * 100.0),
            sustain: signed_pct("Sustain", s.sustain * 100.0),
            attack_len: ms_log_param("Attack Length", s.attack_len_ms, 1.0, 50.0),
            sustain_len: ms_log_param("Sustain Length", s.sustain_len_ms, 20.0, 1000.0),
            focus: FloatParam::new("Focus", s.focus, log_range(20.0, 2000.0))
                .with_value_to_string(Arc::new(|v| if v <= 20.5 { "Off".to_string() } else { fmt_hz0(v) }))
                .with_string_to_value(Arc::new(|t| if t.trim().eq_ignore_ascii_case("off") { Some(20.0) } else { parse_hz(t) })),
            link: BoolParam::new("Stereo Link", s.link),
            clip: BoolParam::new("Clip Guard", s.clip),
            out_gain: db_param("Output", s.out_gain_db, -24.0, 12.0),
            mix: pct_param("Mix", s.mix * 100.0, 0.0, 100.0),
        }
    }
    pub fn settings(&self) -> Settings {
        Settings {
            attack: self.attack.value() / 100.0,
            sustain: self.sustain.value() / 100.0,
            attack_len_ms: self.attack_len.value(),
            sustain_len_ms: self.sustain_len.value(),
            focus: self.focus.value(),
            link: self.link.value(),
            clip: self.clip.value(),
            out_gain_db: self.out_gain.value(),
            mix: self.mix.value() / 100.0,
        }
    }
}

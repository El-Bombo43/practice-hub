use crate::engine::Settings;
use crate::scale::{mask_for, NOTE_NAMES, SCALES};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use phub_common::fmt::*;
use phub_common::params::*;
use std::sync::Arc;

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeParam {
    #[id = "track"]
    #[name = "Track"]
    Track,
    #[id = "mix"]
    #[name = "Mix"]
    Mix,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoiceParam {
    #[id = "high"]
    #[name = "High"]
    High,
    #[id = "mid"]
    #[name = "Mid"]
    Mid,
    #[id = "low"]
    #[name = "Low"]
    Low,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyParam {
    #[id = "c"]
    #[name = "C"]
    C,
    #[id = "cs"]
    #[name = "C#"]
    Cs,
    #[id = "d"]
    #[name = "D"]
    D,
    #[id = "ds"]
    #[name = "D#"]
    Ds,
    #[id = "e"]
    #[name = "E"]
    E,
    #[id = "f"]
    #[name = "F"]
    F,
    #[id = "fs"]
    #[name = "F#"]
    Fs,
    #[id = "g"]
    #[name = "G"]
    G,
    #[id = "gs"]
    #[name = "G#"]
    Gs,
    #[id = "a"]
    #[name = "A"]
    A,
    #[id = "as"]
    #[name = "A#"]
    As,
    #[id = "b"]
    #[name = "B"]
    B,
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScaleParam {
    #[id = "chromatic"]
    #[name = "Chromatic"]
    Chromatic,
    #[id = "major"]
    #[name = "Major"]
    Major,
    #[id = "minor"]
    #[name = "Natural Minor"]
    NaturalMinor,
    #[id = "harmonic"]
    #[name = "Harmonic Minor"]
    HarmonicMinor,
    #[id = "melodic"]
    #[name = "Melodic Minor"]
    MelodicMinor,
    #[id = "majpent"]
    #[name = "Major Pentatonic"]
    MajorPentatonic,
    #[id = "minpent"]
    #[name = "Minor Pentatonic"]
    MinorPentatonic,
    #[id = "blues"]
    #[name = "Blues"]
    Blues,
    #[id = "dorian"]
    #[name = "Dorian"]
    Dorian,
    #[id = "phrygian"]
    #[name = "Phrygian"]
    Phrygian,
    #[id = "lydian"]
    #[name = "Lydian"]
    Lydian,
    #[id = "mixolydian"]
    #[name = "Mixolydian"]
    Mixolydian,
    #[id = "custom"]
    #[name = "Custom"]
    Custom,
}

pub const KEYS: [KeyParam; 12] = [KeyParam::C, KeyParam::Cs, KeyParam::D, KeyParam::Ds, KeyParam::E, KeyParam::F, KeyParam::Fs, KeyParam::G, KeyParam::Gs, KeyParam::A, KeyParam::As, KeyParam::B];
pub const SCALE_LIST: [ScaleParam; 13] = [
    ScaleParam::Chromatic, ScaleParam::Major, ScaleParam::NaturalMinor, ScaleParam::HarmonicMinor, ScaleParam::MelodicMinor, ScaleParam::MajorPentatonic, ScaleParam::MinorPentatonic,
    ScaleParam::Blues, ScaleParam::Dorian, ScaleParam::Phrygian, ScaleParam::Lydian, ScaleParam::Mixolydian, ScaleParam::Custom,
];

pub const SIZES: [(u32, u32); 3] = [(720, 420), (920, 540), (1240, 720)];

#[derive(Params)]
pub struct SnapParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[id = "mode"]
    pub mode: EnumParam<ModeParam>,
    #[id = "voice"]
    pub voice: EnumParam<VoiceParam>,
    #[id = "key"]
    pub key: EnumParam<KeyParam>,
    #[id = "scale"]
    pub scale: EnumParam<ScaleParam>,
    #[id = "n_c"]
    pub n_c: BoolParam,
    #[id = "n_cs"]
    pub n_cs: BoolParam,
    #[id = "n_d"]
    pub n_d: BoolParam,
    #[id = "n_ds"]
    pub n_ds: BoolParam,
    #[id = "n_e"]
    pub n_e: BoolParam,
    #[id = "n_f"]
    pub n_f: BoolParam,
    #[id = "n_fs"]
    pub n_fs: BoolParam,
    #[id = "n_g"]
    pub n_g: BoolParam,
    #[id = "n_gs"]
    pub n_gs: BoolParam,
    #[id = "n_a"]
    pub n_a: BoolParam,
    #[id = "n_as"]
    pub n_as: BoolParam,
    #[id = "n_b"]
    pub n_b: BoolParam,
    #[id = "retune"]
    pub retune: FloatParam,
    #[id = "humanize"]
    pub humanize: FloatParam,
    #[id = "flex"]
    pub flex: FloatParam,
    #[id = "amount"]
    pub amount: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
    #[id = "out_gain"]
    pub out_gain: FloatParam,
}

impl Default for SnapParams {
    fn default() -> Self {
        Self::from_settings(&Settings::default(), KeyParam::C, ScaleParam::Major)
    }
}

impl SnapParams {
    pub fn from_settings(s: &Settings, key: KeyParam, scale: ScaleParam) -> Self {
        let n = |i: usize, name: &'static str| BoolParam::new(name, s.notes[i]);
        Self {
            editor_state: EguiState::from_size(SIZES[1].0, SIZES[1].1),
            mode: EnumParam::new("Mode", if s.track { ModeParam::Track } else { ModeParam::Mix }).non_automatable(),
            voice: EnumParam::new("Voice", [VoiceParam::High, VoiceParam::Mid, VoiceParam::Low][s.voice.min(2)]),
            key: EnumParam::new("Key", key),
            scale: EnumParam::new("Scale", scale),
            n_c: n(0, "C"),
            n_cs: n(1, "C#"),
            n_d: n(2, "D"),
            n_ds: n(3, "D#"),
            n_e: n(4, "E"),
            n_f: n(5, "F"),
            n_fs: n(6, "F#"),
            n_g: n(7, "G"),
            n_gs: n(8, "G#"),
            n_a: n(9, "A"),
            n_as: n(10, "A#"),
            n_b: n(11, "B"),
            retune: FloatParam::new("Retune", s.retune_ms, FloatRange::Skewed { min: 0.0, max: 400.0, factor: FloatRange::skew_factor(-1.0) })
                .with_value_to_string(Arc::new(|v| if v < 0.5 { "Snap".to_string() } else { fmt_ms0(v) }))
                .with_string_to_value(Arc::new(|t| if t.trim().eq_ignore_ascii_case("snap") { Some(0.0) } else { parse_ms(t) })),
            humanize: pct_param("Humanize", s.humanize * 100.0, 0.0, 100.0),
            flex: pct_param("Flex", s.flex * 100.0, 0.0, 100.0),
            amount: pct_param("Amount", s.amount * 100.0, 0.0, 100.0),
            mix: pct_param("Mix", s.mix * 100.0, 0.0, 100.0),
            out_gain: db_param("Output", s.out_gain_db, -24.0, 12.0),
        }
    }

    pub fn toggles(&self) -> [bool; 12] {
        [self.n_c.value(), self.n_cs.value(), self.n_d.value(), self.n_ds.value(), self.n_e.value(), self.n_f.value(), self.n_fs.value(), self.n_g.value(), self.n_gs.value(), self.n_a.value(), self.n_as.value(), self.n_b.value()]
    }
    pub fn toggle_params(&self) -> [&BoolParam; 12] {
        [&self.n_c, &self.n_cs, &self.n_d, &self.n_ds, &self.n_e, &self.n_f, &self.n_fs, &self.n_g, &self.n_gs, &self.n_a, &self.n_as, &self.n_b]
    }

    /// The notes in force. A named scale (Key + Scale) wins, so a host-automated Key or Scale change takes effect
    /// even though only the editor can rewrite the toggles; "Custom" means the toggles are the truth.
    pub fn notes(&self) -> [bool; 12] {
        let sc = self.scale.value();
        if sc == ScaleParam::Custom {
            self.toggles()
        } else {
            mask_for(self.key.value() as usize, sc as usize)
        }
    }

    pub fn settings(&self) -> Settings {
        Settings {
            track: self.mode.value() == ModeParam::Track,
            voice: self.voice.value() as usize,
            notes: self.notes(),
            retune_ms: self.retune.value(),
            humanize: self.humanize.value() / 100.0,
            flex: self.flex.value() / 100.0,
            amount: self.amount.value() / 100.0,
            mix: self.mix.value() / 100.0,
            out_gain_db: self.out_gain.value(),
        }
    }
}

pub fn scale_name(i: usize) -> &'static str {
    if i < SCALES.len() {
        SCALES[i].0
    } else {
        "Custom"
    }
}
pub fn key_name(i: usize) -> &'static str {
    NOTE_NAMES[i % 12]
}

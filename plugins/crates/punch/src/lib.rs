//! Punch: a level-independent transient shaper. See `SPEC.md`.
pub mod dsp;
pub mod gui;
pub mod params;
pub mod presets;

use dsp::Engine;
use nih_plug::prelude::*;
use params::PunchParams;
use phub_common::dsp::PeakCell;
use std::sync::Arc;

#[derive(Default)]
pub struct Shared {
    pub in_peak: PeakCell,
    pub out_peak: PeakCell,
    pub gain_up: PeakCell,
    pub gain_down: PeakCell,
}

pub struct Punch {
    params: Arc<PunchParams>,
    shared: Arc<Shared>,
    engine: Engine,
}

impl Default for Punch {
    fn default() -> Self {
        Self { params: Arc::new(PunchParams::default()), shared: Arc::new(Shared::default()), engine: Engine::new(48_000.0) }
    }
}

impl Plugin for Punch {
    const NAME: &'static str = "Punch";
    const VENDOR: &'static str = "Practice Hub";
    const URL: &'static str = "https://github.com/el-bombo43/practice-hub";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), names: PortNames { layout: Some("Stereo"), ..PortNames::const_default() }, ..AudioIOLayout::const_default() },
        AudioIOLayout { main_input_channels: NonZeroU32::new(1), main_output_channels: NonZeroU32::new(1), names: PortNames { layout: Some("Mono"), ..PortNames::const_default() }, ..AudioIOLayout::const_default() },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _e: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        gui::create(self.params.clone(), self.shared.clone())
    }
    fn initialize(&mut self, _l: &AudioIOLayout, config: &BufferConfig, _c: &mut impl InitContext<Self>) -> bool {
        self.engine = Engine::new(config.sample_rate as f64);
        true
    }
    fn reset(&mut self) {
        let fs = self.engine.sample_rate() as f64;
        self.engine.reset(fs);
    }
    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, _ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        self.engine.set_settings(&self.params.settings());
        let ch = buffer.as_slice();
        match ch.len() {
            0 => {}
            1 => self.engine.process(&mut ch[0][..], None),
            _ => {
                let (a, b) = ch.split_at_mut(1);
                self.engine.process(&mut a[0][..], Some(&mut b[0][..]));
            }
        }
        let m = self.engine.take_meters();
        if self.params.editor_state.is_open() {
            self.shared.in_peak.max(m.in_peak);
            self.shared.out_peak.max(m.out_peak);
            if m.gain_db > 0.0 {
                self.shared.gain_up.max(m.gain_db);
            } else {
                self.shared.gain_down.max(-m.gain_db);
            }
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Punch {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubPunchShaper1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(Punch);

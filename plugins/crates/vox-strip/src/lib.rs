//! Vox Strip: a one-plugin vocal chain. See `SPEC.md`.
pub mod dsp;
pub mod gui;
pub mod params;
pub mod presets;

use dsp::Engine;
use nih_plug::prelude::*;
use params::VoxParams;
use phub_common::dsp::PeakCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Audio thread -> editor.
#[derive(Default)]
pub struct Shared {
    pub in_peak: PeakCell,
    pub out_peak: PeakCell,
    pub ds_gr: PeakCell,
    pub comp_gr: PeakCell,
    pub gate_open: AtomicBool,
    /// De-esser Listen: editor state, not saved with the project.
    pub listen: AtomicBool,
}

pub struct VoxStrip {
    params: Arc<VoxParams>,
    shared: Arc<Shared>,
    engine: Engine,
}

impl Default for VoxStrip {
    fn default() -> Self {
        Self { params: Arc::new(VoxParams::default()), shared: Arc::new(Shared::default()), engine: Engine::new(48_000.0) }
    }
}

impl Plugin for VoxStrip {
    const NAME: &'static str = "Vox Strip";
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
        let open = self.params.editor_state.is_open();
        let listen = open && self.shared.listen.load(Ordering::Relaxed);
        self.engine.set_settings(&self.params.settings(listen));
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
        if open {
            self.shared.in_peak.max(m.in_peak);
            self.shared.out_peak.max(m.out_peak);
            self.shared.ds_gr.max(m.ds_gr);
            self.shared.comp_gr.max(m.comp_gr);
            self.shared.gate_open.store(m.gate_open, Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for VoxStrip {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubVoxStrip0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(VoxStrip);

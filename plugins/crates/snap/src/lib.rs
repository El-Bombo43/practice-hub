//! Snap: real-time vocal pitch correction. See `SPEC.md`. (Never called by anyone else's trademark in the UI or code.)
pub mod engine;
pub mod gui;
pub mod params;
pub mod pitch;
pub mod presets;
pub mod scale;

use engine::Engine;
use nih_plug::prelude::*;
use params::SnapParams;
use phub_common::dsp::PeakCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Audio thread -> editor.
pub struct Shared {
    pub in_peak: PeakCell,
    pub out_peak: PeakCell,
    pub in_midi: AtomicU32,
    pub out_midi: AtomicU32,
    pub target: AtomicU32,
}

impl Default for Shared {
    fn default() -> Self {
        Self { in_peak: PeakCell::new(), out_peak: PeakCell::new(), in_midi: AtomicU32::new(f32::NAN.to_bits()), out_midi: AtomicU32::new(f32::NAN.to_bits()), target: AtomicU32::new(0) }
    }
}

pub struct Snap {
    params: Arc<SnapParams>,
    shared: Arc<Shared>,
    engine: Engine,
    reported_latency: u32,
}

impl Default for Snap {
    fn default() -> Self {
        let p = Arc::new(SnapParams::default());
        let s = p.settings();
        Self { engine: Engine::new(48_000.0, &s), params: p, shared: Arc::new(Shared::default()), reported_latency: 0 }
    }
}

impl Plugin for Snap {
    const NAME: &'static str = "Snap";
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
    fn initialize(&mut self, _l: &AudioIOLayout, config: &BufferConfig, ctx: &mut impl InitContext<Self>) -> bool {
        self.engine = Engine::new(config.sample_rate as f64, &self.params.settings());
        self.reported_latency = self.engine.latency() as u32;
        ctx.set_latency_samples(self.reported_latency);
        true
    }
    fn reset(&mut self) {
        let fs = self.engine.sample_rate() as f64;
        self.engine = Engine::new(fs, &self.params.settings());
    }
    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let s = self.params.settings();
        self.engine.set_settings(&s); // a change of Mode or Voice rebuilds the engine (a brief dropout)
        let lat = self.engine.latency() as u32;
        if lat != self.reported_latency {
            self.reported_latency = lat;
            ctx.set_latency_samples(lat);
        }
        let ch = buffer.as_slice();
        match ch.len() {
            0 => {}
            1 => self.engine.process(&mut ch[0][..], None),
            _ => {
                let (a, b) = ch.split_at_mut(1);
                self.engine.process(&mut a[0][..], Some(&mut b[0][..]));
            }
        }
        let (ip, op) = self.engine.take_peaks();
        if self.params.editor_state.is_open() {
            self.shared.in_peak.max(ip);
            self.shared.out_peak.max(op);
            let d = self.engine.display;
            self.shared.in_midi.store(d.in_midi.to_bits(), Ordering::Relaxed);
            self.shared.out_midi.store(d.out_midi.to_bits(), Ordering::Relaxed);
            self.shared.target.store(d.target as u32, Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Snap {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubSnapPitch001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::PitchShift];
}

nih_export_vst3!(Snap);

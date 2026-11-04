//! Chamber: an algorithmic reverb (Plate / Hall / Room). See `SPEC.md`.
pub mod dsp;
pub mod fmt;
pub mod gui;
pub mod params;
pub mod presets;
pub mod space;

use dsp::Engine;
use nih_plug::prelude::*;
use params::ChamberParams;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Audio thread -> editor: peaks since the editor last looked (non-negative f32 bits order like the numbers).
pub struct Shared {
    pub in_peak: AtomicU32,
    pub wet_peak: AtomicU32,
    pub gr_db: AtomicU32,
    pub wet_level: AtomicU32,
}

impl Shared {
    pub fn new() -> Self {
        Self { in_peak: AtomicU32::new(0), wet_peak: AtomicU32::new(0), gr_db: AtomicU32::new(0), wet_level: AtomicU32::new(0) }
    }
    /// Read and clear a peak.
    pub fn take(a: &AtomicU32) -> f32 {
        f32::from_bits(a.swap(0, Ordering::Relaxed))
    }
    pub fn get(a: &AtomicU32) -> f32 {
        f32::from_bits(a.load(Ordering::Relaxed))
    }
}

impl Default for Shared {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Chamber {
    params: Arc<ChamberParams>,
    shared: Arc<Shared>,
    engine: Engine,
    tempo: f64,
    mono_in: bool,
    scratch: Vec<f32>,
}

impl Default for Chamber {
    fn default() -> Self {
        Self { params: Arc::new(ChamberParams::default()), shared: Arc::new(Shared::new()), engine: Engine::new(48_000.0), tempo: 120.0, mono_in: false, scratch: Vec::new() }
    }
}

impl Plugin for Chamber {
    const NAME: &'static str = "Chamber";
    const VENDOR: &'static str = "Practice Hub";
    const URL: &'static str = "https://github.com/el-bombo43/practice-hub";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout { main_input_channels: NonZeroU32::new(2), main_output_channels: NonZeroU32::new(2), names: PortNames { layout: Some("Stereo"), ..PortNames::const_default() }, ..AudioIOLayout::const_default() },
        AudioIOLayout { main_input_channels: NonZeroU32::new(1), main_output_channels: NonZeroU32::new(2), names: PortNames { layout: Some("Mono to stereo"), ..PortNames::const_default() }, ..AudioIOLayout::const_default() },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        gui::create(self.params.clone(), self.shared.clone())
    }

    fn initialize(&mut self, layout: &AudioIOLayout, config: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        // allocates here, never on the audio thread
        self.engine = Engine::new(config.sample_rate as f64);
        self.mono_in = layout.main_input_channels.map_or(false, |c| c.get() == 1);
        self.scratch = vec![0.0; config.max_buffer_size as usize];
        true
    }

    fn reset(&mut self) {
        let fs = self.engine.sample_rate() as f64;
        self.engine.reset(fs);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        if let Some(t) = ctx.transport().tempo {
            if t.is_finite() && t > 1.0 {
                self.tempo = t;
            }
        }
        let settings = self.params.settings(self.tempo);
        self.engine.set_settings(&settings);
        let n = buffer.samples();
        let channels = buffer.as_slice();
        if channels.len() >= 2 {
            let (a, b) = channels.split_at_mut(1);
            if self.mono_in {
                b[0][..n].copy_from_slice(&a[0][..n]); // mono in: both sides hear the same input
            }
            self.engine.process(&mut a[0][..], &mut b[0][..]);
        } else if channels.len() == 1 && n <= self.scratch.len() {
            self.scratch[..n].copy_from_slice(&channels[0][..n]);
            self.engine.process(&mut channels[0][..], &mut self.scratch[..n]);
        }
        if self.params.editor_state.is_open() {
            let m = self.engine.take_meters();
            self.shared.in_peak.fetch_max(m.in_peak.to_bits(), Ordering::Relaxed);
            self.shared.wet_peak.fetch_max(m.wet_peak.to_bits(), Ordering::Relaxed);
            self.shared.gr_db.fetch_max(m.gr_db.to_bits(), Ordering::Relaxed);
            self.shared.wet_level.store(m.wet_level.to_bits(), Ordering::Relaxed);
        }
        if self.engine.is_asleep() {
            ProcessStatus::Normal
        } else {
            ProcessStatus::KeepAlive
        }
    }
}

impl Vst3Plugin for Chamber {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubChamberRvb01";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Reverb];
}

nih_export_vst3!(Chamber);

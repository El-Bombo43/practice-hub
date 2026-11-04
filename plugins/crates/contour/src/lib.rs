//! Contour: a mixing EQ with a Pro-Q-style workflow and per-band dynamic EQ. See `SPEC.md`.

pub mod analyzer;
pub mod dsp;
pub mod fmt;
pub mod params;

use analyzer::Shared;
use dsp::Engine;
use nih_plug::prelude::*;
use params::ContourParams;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Contour {
    params: Arc<ContourParams>,
    shared: Arc<Shared>,
    engine: Engine,
    pre_scratch: Vec<f32>,
    post_scratch: Vec<f32>,
}

impl Default for Contour {
    fn default() -> Self {
        Self {
            params: Arc::new(ContourParams::default()),
            shared: Arc::new(Shared::new()),
            engine: Engine::new(48_000.0),
            pre_scratch: Vec::new(),
            post_scratch: Vec::new(),
        }
    }
}

impl Plugin for Contour {
    const NAME: &'static str = "Contour";
    const VENDOR: &'static str = "Practice Hub";
    const URL: &'static str = "https://github.com/el-bombo43/practice-hub";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[new_nonzero_u32(2)],
            names: PortNames { layout: Some("Stereo"), main_input: None, main_output: None, aux_inputs: &["Sidechain"], aux_outputs: &[] },
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            aux_input_ports: &[new_nonzero_u32(1)],
            names: PortNames { layout: Some("Mono"), main_input: None, main_output: None, aux_inputs: &["Sidechain"], aux_outputs: &[] },
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(&mut self, _layout: &AudioIOLayout, config: &BufferConfig, _ctx: &mut impl InitContext<Self>) -> bool {
        self.engine.reset(config.sample_rate as f64);
        self.shared.sample_rate.store(config.sample_rate.to_bits(), Ordering::Relaxed);
        let cap = config.max_buffer_size as usize;
        self.pre_scratch = vec![0.0; cap];
        self.post_scratch = vec![0.0; cap];
        true
    }

    fn reset(&mut self) {
        let fs = self.engine.sample_rate();
        self.engine.reset(fs);
    }

    fn process(&mut self, buffer: &mut Buffer, aux: &mut AuxiliaryBuffers, _ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let n = buffer.samples();
        self.engine.set_settings(&self.params.band_settings(), self.params.global_settings());
        let solo = self.shared.solo.load(Ordering::Relaxed);
        self.engine.set_solo((solo >= 0).then_some(solo as usize));

        let analyse = self.params.editor_state.is_open() && n <= self.pre_scratch.len();
        let channels = buffer.as_slice();
        let nch = channels.len();
        if analyse {
            for i in 0..n {
                self.pre_scratch[i] = if nch > 1 { 0.5 * (channels[0][i] + channels[1][i]) } else { channels[0][i] };
            }
        }

        let sc = aux.inputs.first_mut().map(|b| b.as_slice_immutable());
        if nch >= 2 {
            let (a, b) = channels.split_at_mut(1);
            let sc2 = sc.as_ref().and_then(|s| (s.len() >= 2).then(|| (&s[0][..], &s[1][..])));
            self.engine.process(&mut a[0][..], &mut b[0][..], sc2);
        } else if nch == 1 {
            let sc1 = sc.as_ref().and_then(|s| s.first().map(|c| &c[..]));
            self.engine.process_mono(&mut channels[0][..], sc1);
        }

        if analyse {
            for i in 0..n {
                self.post_scratch[i] = if nch > 1 { 0.5 * (channels[0][i] + channels[1][i]) } else { channels[0][i] };
            }
            if let Some(mut ring) = self.shared.ring.try_lock() {
                ring.push(&self.pre_scratch[..n], &self.post_scratch[..n]);
            }
        }
        for i in 0..dsp::BAND_COUNT {
            self.shared.dyn_db[i].store(self.engine.dyn_db(i).to_bits(), Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Contour {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubContourEQ001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Eq];
}

nih_export_vst3!(Contour);

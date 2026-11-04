//! Scaffold plugin: proves the whole chain (build, bundle, Windows CI, load in Ableton, GUI, state).
//! A real plugin copies this layout: `dsp.rs` for testable maths, `lib.rs` for the plugin glue and GUI.

pub mod dsp;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct ScaffoldGain {
    params: Arc<ScaffoldGainParams>,
    /// Output peak since the GUI last read it, as f32 bits.
    peak: Arc<AtomicU32>,
}

#[derive(Params)]
struct ScaffoldGainParams {
    /// Window size and any other GUI state saved with the project.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    /// The parameter ID ("gain") is permanent once a plugin is used in real projects.
    #[id = "gain"]
    gain: FloatParam,
}

impl Default for ScaffoldGain {
    fn default() -> Self {
        Self { params: Arc::new(ScaffoldGainParams::default()), peak: Arc::new(AtomicU32::new(0)) }
    }
}

impl Default for ScaffoldGainParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(420, 220),
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0)) // no zipper noise when automated
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for ScaffoldGain {
    const NAME: &'static str = "Practice Hub Scaffold Gain";
    const VENDOR: &'static str = "Practice Hub";
    const URL: &'static str = "https://github.com/el-bombo43/practice-hub";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let peak = self.peak.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |ctx, _| {
                // dark theme in the Practice Hub palette
                let mut v = egui::Visuals::dark();
                v.panel_fill = egui::Color32::from_rgb(0x14, 0x14, 0x16);
                v.override_text_color = Some(egui::Color32::from_rgb(0xf2, 0xf2, 0xf0));
                ctx.set_visuals(v);
            },
            move |ctx, setter, _state| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Scaffold Gain");
                    ui.label("Pipeline check: if you can see this in Ableton, the build chain works.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.label("Gain");
                        ui.add(widgets::ParamSlider::for_param(&params.gain, setter));
                    });
                    ui.add_space(12.0);
                    // output meter: the audio thread keeps the max, the GUI reads and decays it
                    let p = f32::from_bits(peak.load(Ordering::Relaxed));
                    peak.store((p * 0.85).to_bits(), Ordering::Relaxed);
                    let db = util::gain_to_db(p.max(1e-5)).clamp(-60.0, 6.0);
                    ui.add(egui::ProgressBar::new((db + 60.0) / 66.0).text(format!("{db:.1} dB")));
                });
                ctx.request_repaint_after(std::time::Duration::from_millis(33));
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut block_peak = 0.0_f32;
        for mut frame in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            for sample in frame.iter_mut() {
                *sample *= gain;
                block_peak = block_peak.max(sample.abs());
            }
        }
        // keep the loudest value for the GUI; no allocation, no locks on the audio thread
        let prev = f32::from_bits(self.peak.load(Ordering::Relaxed));
        if block_peak > prev {
            self.peak.store(block_peak.to_bits(), Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for ScaffoldGain {
    const VST3_CLASS_ID: [u8; 16] = *b"PracticeHubGain1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_vst3!(ScaffoldGain);

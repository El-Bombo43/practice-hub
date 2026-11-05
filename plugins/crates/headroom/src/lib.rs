//! Headroom: a mastering meter (LUFS and true peak) that collapses to a small box. See `SPEC.md`.
pub mod gui;
pub mod meter;
pub mod state;

use meter::{Bar, Meter, Snapshot, Transport, MAX_BARS};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use parking_lot::{Mutex, RwLock};
use state::HeadSettings;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Audio thread -> editor.
#[derive(Default)]
pub struct Shared {
    pub snap: Mutex<Snapshot>,
    pub bars: Mutex<BarStore>,
    pub reset: AtomicBool,
    pub playing: AtomicBool,
    pub bar_beats: Mutex<f64>,
}

#[derive(Default)]
pub struct BarStore {
    pub bars: VecDeque<Bar>,
    /// index (since the last reset) of `bars[0]`
    pub first_index: u64,
}

#[derive(Params)]
pub struct HeadParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
    #[persist = "settings"]
    pub settings: Arc<RwLock<HeadSettings>>,
}

impl Default for HeadParams {
    fn default() -> Self {
        Self { editor_state: EguiState::from_size(960, 560), settings: Arc::new(RwLock::new(HeadSettings::default())) }
    }
}

pub struct Headroom {
    params: Arc<HeadParams>,
    shared: Arc<Shared>,
    meter: Meter,
    was_playing: bool,
}

impl Default for Headroom {
    fn default() -> Self {
        Self { params: Arc::new(HeadParams::default()), shared: Arc::new(Shared::default()), meter: Meter::new(48_000.0), was_playing: false }
    }
}

impl Plugin for Headroom {
    const NAME: &'static str = "Headroom";
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
        self.meter = Meter::new(config.sample_rate as f64); // a new rate resets every meter
        true
    }
    fn reset(&mut self) {
        let fs = self.meter.sample_rate();
        self.meter = Meter::new(fs);
    }

    fn process(&mut self, buffer: &mut Buffer, _aux: &mut AuxiliaryBuffers, ctx: &mut impl ProcessContext<Self>) -> ProcessStatus {
        let tr = ctx.transport();
        let t = Transport { playing: tr.playing, beats: tr.pos_beats(), seconds: tr.pos_seconds().unwrap_or(0.0), tempo: tr.tempo.unwrap_or(120.0) };
        let st = *self.params.settings.read();
        if tr.playing && !self.was_playing && st.auto_reset {
            self.meter.reset();
        }
        self.was_playing = tr.playing;
        if self.shared.reset.swap(false, Ordering::Relaxed) {
            self.meter.reset();
        }
        let ceiling = st.ceiling(self.meter.snapshot().integrated);
        if tr.playing || st.measure_stopped {
            let ch = buffer.as_slice_immutable();
            match ch.len() {
                0 => {}
                1 => self.meter.process(&ch[0][..], None, ceiling, &t),
                _ => self.meter.process(&ch[0][..], Some(&ch[1][..]), ceiling, &t),
            }
        }
        // publish; if the editor holds a lock right now, try again next block
        if self.params.editor_state.is_open() {
            if let Some(mut s) = self.shared.snap.try_lock() {
                s.copy_from(self.meter.snapshot());
            }
            if let Some(mut b) = self.shared.bars.try_lock() {
                if self.meter.reset_pending {
                    b.bars.clear();
                    b.first_index = 0;
                    self.meter.reset_pending = false;
                }
                let n = self.meter.bars.len() as u64;
                b.bars.extend(self.meter.bars.drain(..));
                while b.bars.len() > MAX_BARS {
                    b.bars.pop_front();
                    b.first_index += 1;
                }
                let _ = n;
            }
            *self.shared.bar_beats.lock() = tr.time_sig_numerator.map_or(4.0, |n| n as f64 * 4.0 / tr.time_sig_denominator.unwrap_or(4) as f64);
            self.shared.playing.store(tr.playing, Ordering::Relaxed);
        } else {
            self.meter.bars.clear();
            self.meter.reset_pending = false;
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Headroom {
    const VST3_CLASS_ID: [u8; 16] = *b"PHubHeadroomMtr1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Analyzer];
}

nih_export_vst3!(Headroom);

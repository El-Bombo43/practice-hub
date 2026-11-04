use headroom::meter::{Bar, OverEvent, Snapshot};
use headroom::state::{HeadSettings, Mode};
use headroom::{gui, HeadParams, Shared};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use parking_lot::RwLock;
use phub_common::offscreen::{render_frames, Dummy};
use phub_common::ui::setup;
use std::sync::Arc;

fn shot(mode: Mode, narrow: bool, name: &str) {
    let Ok(dir) = std::env::var("HEADROOM_RENDER_DIR") else { return };
    let st = HeadSettings { mode, strip_narrow: narrow, ..Default::default() };
    let (w, h) = st.size();
    let params = Arc::new(HeadParams { editor_state: EguiState::from_size(w, h), settings: Arc::new(RwLock::new(st)) });
    let shared = Arc::new(Shared::default());
    let mut bars = Vec::new();
    for i in 0..6000usize {
        let t = i as f32 * 0.02;
        let env = 0.35 + 0.3 * (t * 0.7).sin().abs() + 0.12 * ((t * 9.0).sin());
        let over = i % 1400 == 700;
        bars.push(Bar { tp: if over { 1.2 } else { env.clamp(0.05, 0.9) }, st: if i < 150 { f32::NEG_INFINITY } else { -13.0 + 2.5 * (t * 0.3).sin() }, over });
    }
    shared.bars.lock().bars = bars.into_iter().collect();
    *shared.snap.lock() = Snapshot {
        momentary: -12.4, short_term: -11.9, integrated: -13.2, lra: 6.4, tp_max: [0.4, -0.2], sample_peak: -0.1, max_momentary: -8.7, max_short: -9.9, overs: 4, clips: 2, correlation: 0.62, tp_now: -3.0,
        over_list: (0..4).map(|k| OverEvent { beats: Some(112.0 + k as f64 * 30.0), seconds: 62.0 + k as f64 * 15.0, peak_db: 0.4 - k as f32 * 0.2, bar_index: 700 + k as u64 * 1400 }).collect(),
    };
    let mut local = gui::new_local();
    local.selected_over = Some(1);
    let dummy = Dummy;
    let setter = ParamSetter::new(&dummy);
    let rgba = render_frames(w as usize, h as usize, 6, setup, |ctx, _| gui::update(ctx, &setter, &mut local, &params, &shared));
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

#[test]
fn render_all_modes() {
    shot(Mode::Full, false, "full");
    shot(Mode::Strip, false, "strip-wide");
    shot(Mode::Strip, true, "strip-narrow");
    shot(Mode::Compact, false, "compact");
}

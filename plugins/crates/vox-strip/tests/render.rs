//! Offscreen screenshots of the editor when VOX_RENDER_DIR is set.
use nih_plug::prelude::*;
use phub_common::offscreen::{render_frames, Dummy};
use phub_common::ui::setup;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use vox_strip::dsp::Settings;
use vox_strip::params::{SatType, VoxParams};
use vox_strip::{gui, Shared};

fn shot(p: &Arc<VoxParams>, name: &str, w: usize, h: usize) {
    let Ok(dir) = std::env::var("VOX_RENDER_DIR") else { return };
    let shared = Arc::new(Shared::default());
    let mut local = gui::new_local();
    local.fixed_dt = Some(1.0 / 60.0);
    let dummy = Dummy;
    let setter = ParamSetter::new(&dummy);
    let rgba = render_frames(w, h, 200, setup, |ctx, f| {
        let t = f as f32 / 60.0;
        let env = (-(t * 2.0 % 1.0) * 5.0).exp();
        shared.in_peak.max(0.5 * env + 0.01);
        shared.out_peak.max(0.3 * env + 0.01);
        shared.comp_gr.max(6.0 * env);
        shared.ds_gr.max(3.0 * env);
        shared.gate_open.store(env > 0.05, Ordering::Relaxed);
        gui::update(ctx, &setter, &mut local, p, &shared);
    });
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

#[test]
fn render_editor_screenshots() {
    shot(&Arc::new(VoxParams::default()), "01-default", 960, 450);
    let s = (vox_strip::presets::PRESETS[2].settings)();
    shot(&Arc::new(VoxParams::from_settings(&s, SatType::Tube)), "02-screams", 960, 450);
    let _ = Settings::default();
}

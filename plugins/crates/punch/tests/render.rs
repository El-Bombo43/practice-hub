use nih_plug::prelude::*;
use phub_common::offscreen::{render_frames, Dummy};
use phub_common::ui::setup;
use punch::params::PunchParams;
use punch::{gui, Shared};
use std::sync::Arc;

#[test]
fn render_editor_screenshots() {
    let Ok(dir) = std::env::var("PUNCH_RENDER_DIR") else { return };
    let s = (punch::presets::PRESETS[1].settings)();
    let p = Arc::new(PunchParams::from_settings(&s));
    let shared = Arc::new(Shared::default());
    let mut local = gui::new_local();
    local.fixed_dt = Some(1.0 / 60.0);
    let dummy = Dummy;
    let setter = ParamSetter::new(&dummy);
    let rgba = render_frames(660, 400, 220, setup, |ctx, f| {
        let t = (f % 30) as f32 / 30.0;
        let env = (-t * 6.0).exp();
        let hit = if f % 60 < 30 { 1.0 } else { 0.5 };
        shared.in_peak.max(0.6 * env * hit + 0.01);
        shared.out_peak.max(0.7 * env * hit + 0.01);
        if t < 0.1 { shared.gain_up.max(10.0 * hit) } else if t > 0.3 { shared.gain_down.max(6.0 * env * 2.0 * hit) }
        gui::update(ctx, &setter, &mut local, &p, &shared);
    });
    let file = std::fs::File::create(format!("{dir}/punch.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 660, 400);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

use nih_plug::prelude::*;
use phub_common::offscreen::{render_frames, Dummy};
use phub_common::ui::setup;
use snap::engine::Settings;
use snap::params::{KeyParam, ScaleParam, SnapParams};
use snap::{gui, Shared};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[test]
fn render_editor_screenshot() {
    let Ok(dir) = std::env::var("SNAP_RENDER_DIR") else { return };
    let mut notes = [false; 12];
    for n in snap::scale::mask_for(4, 2).iter().enumerate().filter(|(_, b)| **b).map(|(i, _)| i) {
        notes[n] = true;
    }
    let p = Arc::new(SnapParams::from_settings(&Settings { notes, retune_ms: 80.0, humanize: 0.4, flex: 0.7, ..Default::default() }, KeyParam::E, ScaleParam::NaturalMinor));
    let shared = Arc::new(Shared::default());
    let mut local = gui::new_local();
    local.fixed_dt = Some(1.0 / 60.0);
    let dummy = Dummy;
    let setter = ParamSetter::new(&dummy);
    let rgba = render_frames(920, 540, 400, setup, |ctx, f| {
        let t = f as f32 / 60.0;
        // a sung slide with vibrato and a breath gap
        let base = 64.0 + 3.0 * (t * 0.8).sin() * 2.0 + 0.3 * (t * 34.0).sin();
        let voiced = (t % 2.6) < 2.2;
        let m = if voiced { base } else { f32::NAN };
        let tgt = base.round();
        shared.in_midi.store(m.to_bits(), Ordering::Relaxed);
        shared.out_midi.store((if voiced { tgt + 0.25 * (base - tgt) } else { f32::NAN }).to_bits(), Ordering::Relaxed);
        shared.target.store(tgt as u32, Ordering::Relaxed);
        shared.in_peak.max(0.4);
        shared.out_peak.max(0.35);
        gui::update(ctx, &setter, &mut local, &p, &shared);
    });
    let file = std::fs::File::create(format!("{dir}/snap.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), 920, 540);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

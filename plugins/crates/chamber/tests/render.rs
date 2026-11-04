//! Offscreen render of the editor to PNG files, for checking the layout without a window or GPU.
//! Runs only when CHAMBER_RENDER_DIR is set (it is not part of the normal test run).
use chamber::gui;
use chamber::params::{ChamberParams, ModeParam};
use chamber::Shared;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, pos2, vec2, Rect};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

struct Dummy;
impl GuiContext for Dummy {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Vst3
    }
    fn request_resize(&self) -> bool {
        true
    }
    unsafe fn raw_begin_set_parameter(&self, _p: ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, p: ParamPtr, n: f32) {
        let _ = n; // the renderer shows static state
        let _ = p;
    }
    unsafe fn raw_end_set_parameter(&self, _p: ParamPtr) {}
    fn get_state(&self) -> PluginState {
        unimplemented!()
    }
    fn set_state(&self, _s: PluginState) {}
}

struct Tex {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

fn apply_textures(delta: &epaint::textures::TexturesDelta, texs: &mut HashMap<egui::TextureId, Tex>) {
    for (id, d) in &delta.set {
        let (w, h) = (d.image.width(), d.image.height());
        let pixels: Vec<[u8; 4]> = match &d.image {
            egui::ImageData::Color(c) => c.pixels.iter().map(|p| p.to_array()).collect(),
            egui::ImageData::Font(f) => f.srgba_pixels(None).map(|p| p.to_array()).collect(),
        };
        match d.pos {
            None => {
                texs.insert(*id, Tex { w, h, px: pixels });
            }
            Some([x0, y0]) => {
                if let Some(t) = texs.get_mut(id) {
                    for yy in 0..h {
                        for xx in 0..w {
                            t.px[(y0 + yy) * t.w + x0 + xx] = pixels[yy * w + xx];
                        }
                    }
                }
            }
        }
    }
}

fn rasterize(prims: &[egui::ClippedPrimitive], texs: &HashMap<egui::TextureId, Tex>, ppp: f32, w: usize, h: usize) -> Vec<u8> {
    let mut img = vec![[10.0f32, 10.0, 11.0, 255.0]; w * h];
    for cp in prims {
        let egui::epaint::Primitive::Mesh(mesh) = &cp.primitive else { continue };
        let Some(tex) = texs.get(&mesh.texture_id) else { continue };
        let clip = cp.clip_rect;
        let (cx0, cy0, cx1, cy1) = ((clip.min.x * ppp).max(0.0) as i32, (clip.min.y * ppp).max(0.0) as i32, ((clip.max.x * ppp) as i32).min(w as i32), ((clip.max.y * ppp) as i32).min(h as i32));
        for tri in mesh.indices.chunks_exact(3) {
            let v = [&mesh.vertices[tri[0] as usize], &mesh.vertices[tri[1] as usize], &mesh.vertices[tri[2] as usize]];
            let p: Vec<(f32, f32)> = v.iter().map(|v| (v.pos.x * ppp, v.pos.y * ppp)).collect();
            let (minx, maxx) = (p.iter().map(|q| q.0).fold(f32::MAX, f32::min), p.iter().map(|q| q.0).fold(f32::MIN, f32::max));
            let (miny, maxy) = (p.iter().map(|q| q.1).fold(f32::MAX, f32::min), p.iter().map(|q| q.1).fold(f32::MIN, f32::max));
            let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
            if area.abs() < 1e-6 {
                continue;
            }
            for y in (miny.floor() as i32).max(cy0)..=(maxy.ceil() as i32).min(cy1 - 1) {
                for x in (minx.floor() as i32).max(cx0)..=(maxx.ceil() as i32).min(cx1 - 1) {
                    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = ((p[1].0 - fx) * (p[2].1 - fy) - (p[2].0 - fx) * (p[1].1 - fy)) / area;
                    let w1 = ((p[2].0 - fx) * (p[0].1 - fy) - (p[0].0 - fx) * (p[2].1 - fy)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                        continue;
                    }
                    let uv = (w0 * v[0].uv.x + w1 * v[1].uv.x + w2 * v[2].uv.x, w0 * v[0].uv.y + w1 * v[1].uv.y + w2 * v[2].uv.y);
                    let (tx, ty) = (((uv.0 * tex.w as f32) as usize).min(tex.w - 1), ((uv.1 * tex.h as f32) as usize).min(tex.h - 1));
                    let t = tex.px[ty * tex.w + tx];
                    let mut src = [0.0f32; 4];
                    for k in 0..4 {
                        let vc = w0 * v[0].color.to_array()[k] as f32 + w1 * v[1].color.to_array()[k] as f32 + w2 * v[2].color.to_array()[k] as f32;
                        src[k] = vc / 255.0 * t[k] as f32;
                    }
                    let d = &mut img[y as usize * w + x as usize];
                    let a = src[3] / 255.0;
                    for k in 0..3 {
                        d[k] = src[k] + d[k] * (1.0 - a);
                    }
                }
            }
        }
    }
    img.iter().flat_map(|p| [p[0].clamp(0.0, 255.0) as u8, p[1].clamp(0.0, 255.0) as u8, p[2].clamp(0.0, 255.0) as u8, 255]).collect()
}

fn render(params: &Arc<ChamberParams>, shared: &Arc<Shared>, w: usize, h: usize, name: &str, frames: usize, feed: &dyn Fn(usize, &Shared)) {
    let dir = std::env::var("CHAMBER_RENDER_DIR").unwrap();
    let ctx = egui::Context::default();
    gui::setup(&ctx);
    let mut local = gui::new_local();
    local.fixed_dt = Some(1.0 / 60.0);
    let dummy = Dummy;
    let setter = ParamSetter::new(&dummy);
    let mut texs: HashMap<egui::TextureId, Tex> = HashMap::new();
    let mut last = None;
    for f in 0..frames {
        feed(f, shared);
        let ppp = ctx.pixels_per_point();
        let mut raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w as f32 / ppp, h as f32 / ppp))), ..Default::default() };
        raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(1.0), ..Default::default() });
        let out = ctx.run(raw, |ctx| gui::update(ctx, &setter, &mut local, params, shared));
        apply_textures(&out.textures_delta, &mut texs);
        last = Some((ctx.tessellate(out.shapes, ctx.pixels_per_point()), ctx.pixels_per_point()));
    }
    let (prims, ppp) = last.unwrap();
    let rgba = rasterize(&prims, &texs, ppp, w, h);
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

/// Transient hits every 14 frames at the given level, with a wet level and some duck reduction.
fn hits(f: usize, s: &Shared) {
    let peak = if f % 14 == 0 { 0.6f32 } else { 0.01 };
    s.in_peak.store(peak.to_bits(), Ordering::Relaxed);
    s.wet_peak.store((0.2f32).to_bits(), Ordering::Relaxed);
    s.wet_level.store((0.12f32).to_bits(), Ordering::Relaxed);
    s.gr_db.store((if f % 14 < 5 { 6.0f32 } else { 0.0 }).to_bits(), Ordering::Relaxed);
}

#[test]
fn render_editor_screenshots() {
    if std::env::var("CHAMBER_RENDER_DIR").is_err() {
        return;
    }
    use chamber::dsp::Settings;
    let shared = Arc::new(Shared::new());
    let p = Arc::new(ChamberParams::default());
    render(&p, &shared, 820, 512, "01-plate", 90, &hits);
    let hall = Arc::new(ChamberParams::from_settings(&Settings { mode: 1, size: 0.75, decay_s: 3.5, ..Default::default() }));
    render(&hall, &shared, 820, 512, "02-hall", 90, &hits);
    let room = Arc::new(ChamberParams::from_settings(&Settings { mode: 2, size: 0.3, decay_s: 0.7, mod_depth: 0.6, ..Default::default() }));
    render(&room, &shared, 640, 400, "03-room-small", 90, &hits);
    let frozen = Arc::new(ChamberParams::from_settings(&Settings { mode: 1, freeze: true, predelay_sync: true, duck_db: 12.0, width: 1.4, ..Default::default() }));
    render(&frozen, &shared, 1100, 690, "04-hall-frozen-large", 120, &|f, s| if f < 60 { hits(f, s) } else { s.in_peak.store(0, Ordering::Relaxed) });
    let _ = ModeParam::Hall;
}

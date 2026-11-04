//! Offscreen rendering of an egui UI to RGBA pixels (a CPU triangle rasteriser), plus a do-nothing `GuiContext`.
//! Used by the plugins' tests to look at the editor and to drive it with simulated input, with no window or GPU.
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, epaint, pos2, vec2, Rect};
use std::collections::HashMap;

pub struct Dummy;
impl GuiContext for Dummy {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Vst3
    }
    fn request_resize(&self) -> bool {
        true
    }
    unsafe fn raw_begin_set_parameter(&self, _p: ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, _p: ParamPtr, _n: f32) {}
    unsafe fn raw_end_set_parameter(&self, _p: ParamPtr) {}
    fn get_state(&self) -> PluginState {
        unimplemented!()
    }
    fn set_state(&self, _s: PluginState) {}
}

pub struct Tex {
    w: usize,
    h: usize,
    px: Vec<[u8; 4]>,
}

pub fn apply_textures(delta: &epaint::textures::TexturesDelta, texs: &mut HashMap<egui::TextureId, Tex>) {
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

pub fn rasterize(prims: &[egui::ClippedPrimitive], texs: &HashMap<egui::TextureId, Tex>, ppp: f32, w: usize, h: usize) -> Vec<u8> {
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


/// Run `frames` frames of `ui` on a w x h screen and return the last frame as RGBA bytes.
pub fn render_frames(w: usize, h: usize, frames: usize, setup: impl Fn(&egui::Context), mut ui: impl FnMut(&egui::Context, usize)) -> Vec<u8> {
    let ctx = egui::Context::default();
    setup(&ctx);
    let mut texs: HashMap<egui::TextureId, Tex> = HashMap::new();
    let mut last = None;
    for f in 0..frames {
        let ppp = ctx.pixels_per_point();
        let mut raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(w as f32 / ppp, h as f32 / ppp))), ..Default::default() };
        raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(1.0), ..Default::default() });
        let out = ctx.run(raw, |ctx| ui(ctx, f));
        apply_textures(&out.textures_delta, &mut texs);
        last = Some((ctx.tessellate(out.shapes, ctx.pixels_per_point()), ctx.pixels_per_point()));
    }
    let (prims, ppp) = last.unwrap();
    rasterize(&prims, &texs, ppp, w, h)
}

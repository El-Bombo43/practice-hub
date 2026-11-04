//! Simulated mouse and keyboard input against the editor: which parameter changes does the host receive?
use contour::analyzer::Shared;
use contour::dsp::{BandSettings, FilterType, BAND_COUNT};
use contour::geom::{freq_to_frac, gain_to_frac};
use contour::gui::{self, Local};
use contour::params::ContourParams;
use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, pos2, vec2, Event, Modifiers, PointerButton, Pos2, Rect};
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Default)]
struct Rec {
    /// (parameter name, normalized value) for every set, in order
    sets: Vec<(String, f32)>,
}

struct Dummy(Arc<Mutex<Rec>>);
impl GuiContext for Dummy {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Vst3
    }
    fn request_resize(&self) -> bool {
        true
    }
    unsafe fn raw_begin_set_parameter(&self, _p: ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, p: ParamPtr, n: f32) {
        self.0.lock().sets.push((p.name().to_string(), n));
    }
    unsafe fn raw_end_set_parameter(&self, _p: ParamPtr) {}
    fn get_state(&self) -> PluginState {
        unimplemented!()
    }
    fn set_state(&self, _s: PluginState) {}
}

struct Harness {
    ctx: egui::Context,
    local: Local,
    params: Arc<ContourParams>,
    shared: Arc<Shared>,
    rec: Arc<Mutex<Rec>>,
    ctx_gui: Dummy,
    t: f64,
    pos: Pos2,
}

const W: f32 = 960.0;
const H: f32 = 560.0;
// graph area at the default (M) size: below the 36 px top bar, left of the 42 px dB strip, above the 72 px bottom bar and 18 px label strip
const GL: f32 = 0.0;
const GT: f32 = 36.0;
const GW: f32 = W - 42.0;
const GH: f32 = H - 36.0 - 72.0 - 18.0;

fn gx(f: f32) -> f32 {
    GL + freq_to_frac(f) * GW
}
fn gy(gain: f32, range: f32) -> f32 {
    GT + gain_to_frac(gain, range) * GH
}

impl Harness {
    fn new(bands: &[(usize, BandSettings)]) -> Self {
        let mut all = [BandSettings::default(); BAND_COUNT];
        for (i, b) in bands {
            all[*i] = BandSettings { used: true, ..*b };
        }
        let params = Arc::new(ContourParams::from_settings(&all));
        let rec = Arc::new(Mutex::new(Rec::default()));
        let ctx = egui::Context::default();
        gui::setup(&ctx);
        let mut h = Harness { ctx, local: gui::new_local(), params, shared: Arc::new(Shared::new()), ctx_gui: Dummy(rec.clone()), rec, t: 0.0, pos: pos2(5.0, 5.0) };
        h.frame(vec![], Modifiers::NONE);
        h.frame(vec![], Modifiers::NONE);
        h
    }

    fn frame(&mut self, events: Vec<Event>, mods: Modifiers) {
        self.t += 0.04;
        let mut raw = egui::RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(W, H))), time: Some(self.t), modifiers: mods, events, ..Default::default() };
        raw.viewports.insert(egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(1.0), ..Default::default() });
        let setter = ParamSetter::new(&self.ctx_gui);
        let (params, shared) = (self.params.clone(), self.shared.clone());
        let local = &mut self.local;
        let _ = self.ctx.run(raw, |ctx| gui::update(ctx, &setter, local, &params, &shared));
    }

    fn mv(&mut self, p: Pos2, mods: Modifiers) {
        self.pos = p;
        self.frame(vec![Event::PointerMoved(p)], mods);
    }
    fn down(&mut self, mods: Modifiers) {
        let p = self.pos;
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: mods }], mods);
    }
    fn up(&mut self, mods: Modifiers) {
        let p = self.pos;
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: mods }], mods);
    }
    fn click_at(&mut self, p: Pos2, mods: Modifiers) {
        self.mv(p, mods);
        self.down(mods);
        self.up(mods);
    }
    fn double_click_at(&mut self, p: Pos2) {
        self.click_at(p, Modifiers::NONE);
        self.down(Modifiers::NONE);
        self.up(Modifiers::NONE);
    }
    fn drag(&mut self, from: Pos2, to: Pos2, mods: Modifiers) {
        self.mv(from, mods);
        self.frame(vec![], mods); // a real hand takes far longer than two frames between aiming and pressing
        self.frame(vec![], mods);
        self.down(mods);
        for i in 1..=6 {
            let f = i as f32 / 6.0;
            self.mv(from + (to - from) * f, mods);
        }
        self.up(mods);
    }
    fn scroll(&mut self, p: Pos2, dy: f32, mods: Modifiers) {
        self.mv(p, mods);
        self.frame(vec![], mods); // let a freshly shown band panel finish its first (sizing) frame
        self.frame(vec![Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: vec2(0.0, dy), modifiers: mods }], mods);
    }
    fn key(&mut self, key: egui::Key) {
        let m = Modifiers::NONE;
        self.frame(vec![Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: m }], m);
        self.frame(vec![Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers: m }], m);
    }

    fn sets(&self, name: &str) -> Vec<f32> {
        self.rec.lock().sets.iter().filter(|(n, _)| n == name).map(|(_, v)| *v).collect()
    }
    fn last(&self, name: &str) -> Option<f32> {
        self.sets(name).last().copied()
    }
    fn clear(&mut self) {
        self.rec.lock().sets.clear();
    }
}

fn bell(f: f32, g: f32) -> BandSettings {
    BandSettings { freq: f, gain_db: g, ..BandSettings::default() }
}

#[test]
fn double_click_on_empty_graph_creates_a_bell_there() {
    let mut h = Harness::new(&[]);
    h.double_click_at(pos2(gx(1000.0), gy(6.0, 12.0)));
    let b = h.params.band(0);
    assert_eq!(h.last("1 Used"), Some(1.0));
    let f = b.freq.preview_plain(h.last("1 Freq").expect("freq set"));
    assert!((f - 1000.0).abs() < 40.0, "freq {f}");
    let g = b.gain.preview_plain(h.last("1 Gain").expect("gain set"));
    assert!((g - 6.0).abs() < 0.5, "gain {g}");
    assert_eq!(h.params.prefs.read().selected, Some(0));
}

#[test]
fn double_click_near_the_edges_creates_cuts() {
    let mut h = Harness::new(&[]);
    h.double_click_at(pos2(GL + 8.0, gy(0.0, 12.0)));
    assert!((h.last("1 Type").unwrap() - h.params.band(0).ftype.preview_normalized(contour::params::TypeParam::LowCut)).abs() < 1e-4);
    let mut h = Harness::new(&[]);
    h.double_click_at(pos2(GL + GW - 8.0, gy(0.0, 12.0)));
    assert!((h.last("1 Type").unwrap() - h.params.band(0).ftype.preview_normalized(contour::params::TypeParam::HighCut)).abs() < 1e-4);
}

#[test]
fn new_bands_take_the_lowest_free_slot() {
    let mut h = Harness::new(&[(0, bell(100.0, 0.0)), (2, bell(5000.0, 0.0))]);
    h.double_click_at(pos2(gx(1000.0), gy(3.0, 12.0)));
    assert_eq!(h.last("2 Used"), Some(1.0));
    assert_eq!(h.last("1 Used"), None);
}

#[test]
fn dragging_a_node_moves_frequency_and_gain() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.drag(pos2(gx(1000.0), gy(0.0, 12.0)), pos2(gx(1000.0) + 80.0, gy(0.0, 12.0) - 40.0), Modifiers::NONE);
    let b = h.params.band(0);
    let f = b.freq.preview_plain(h.last("1 Freq").expect("freq"));
    let g = b.gain.preview_plain(h.last("1 Gain").expect("gain"));
    assert!(f > 1100.0 && f < 2500.0, "freq {f}");
    assert!(g > 2.0 && g < 8.0, "gain {g}");
    assert_eq!(h.params.prefs.read().selected, Some(0));
}

#[test]
fn shift_makes_dragging_ten_times_finer() {
    let mut a = Harness::new(&[(0, bell(1000.0, 0.0))]);
    a.drag(pos2(gx(1000.0), gy(0.0, 12.0)), pos2(gx(1000.0) + 100.0, gy(0.0, 12.0)), Modifiers::NONE);
    let mut b = Harness::new(&[(0, bell(1000.0, 0.0))]);
    b.drag(pos2(gx(1000.0), gy(0.0, 12.0)), pos2(gx(1000.0) + 100.0, gy(0.0, 12.0)), Modifiers::SHIFT);
    let fa = a.params.band(0).freq.preview_plain(a.last("1 Freq").unwrap());
    let fb = b.params.band(0).freq.preview_plain(b.last("1 Freq").unwrap());
    let (da, db) = ((fa / 1000.0).ln(), (fb / 1000.0).ln());
    assert!(da > 0.3 && (da / db - 10.0).abs() < 1.5, "{da} vs {db}");
}

#[test]
fn ctrl_drag_keeps_to_the_first_axis() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    let ctrl = Modifiers { ctrl: true, ..Modifiers::NONE };
    let p = pos2(gx(1000.0), gy(0.0, 12.0));
    h.drag(p, p + vec2(90.0, 12.0), ctrl); // mostly horizontal: gain must not move
    let g = h.params.band(0).gain.preview_plain(h.last("1 Gain").unwrap());
    assert!(g.abs() < 0.05, "gain moved to {g}");
    assert!(h.params.band(0).freq.preview_plain(h.last("1 Freq").unwrap()) > 1200.0);
}

#[test]
fn cut_nodes_move_sideways_and_vertical_drag_sets_q() {
    let cut = BandSettings { ftype: FilterType::LowCut, freq: 80.0, q: 0.71, slope: 24, ..BandSettings::default() };
    let mut h = Harness::new(&[(0, cut)]);
    let p = pos2(gx(80.0), gy(0.0, 12.0));
    h.drag(p, p + vec2(30.0, -40.0), Modifiers::NONE);
    assert!(h.last("1 Freq").is_some());
    assert!(h.last("1 Gain").is_none(), "a cut has no gain");
    let q = h.params.band(0).q.preview_plain(h.last("1 Q").expect("q"));
    assert!(q > 1.0, "dragging up should raise Q, got {q}");
}

#[test]
fn wheel_over_a_node_changes_q_and_selects_it() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.scroll(pos2(gx(1000.0), gy(0.0, 12.0)), 50.0, Modifiers::NONE);
    let q = h.params.band(0).q.preview_plain(h.last("1 Q").expect("q"));
    assert!(q > 1.05 && q < 1.2, "one notch up: Q {q}");
    h.clear();
    h.scroll(pos2(gx(1000.0), gy(0.0, 12.0)), -50.0, Modifiers::NONE);
    assert!(h.params.band(0).q.preview_plain(h.last("1 Q").unwrap()) < 1.0 + 1e-3);
}

#[test]
fn wheel_on_empty_graph_does_nothing() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.scroll(pos2(gx(5000.0), gy(8.0, 12.0)), 100.0, Modifiers::NONE);
    assert!(h.rec.lock().sets.is_empty());
}

#[test]
fn ctrl_wheel_turns_dynamic_on_and_sets_range() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.scroll(pos2(gx(1000.0), gy(0.0, 12.0)), 100.0, Modifiers { ctrl: true, ..Modifiers::NONE });
    assert_eq!(h.last("1 Dynamic"), Some(1.0));
    let r = h.params.band(0).range.preview_plain(h.last("1 Range").expect("range"));
    assert!((r + 5.0).abs() < 0.2, "range {r}"); // -6 default + 2 notches * 0.5 dB
}

#[test]
fn ctrl_wheel_ignores_bands_that_cannot_be_dynamic() {
    let cut = BandSettings { ftype: FilterType::LowCut, freq: 200.0, ..BandSettings::default() };
    let mut h = Harness::new(&[(0, cut)]);
    h.scroll(pos2(gx(200.0), gy(0.0, 12.0)), 100.0, Modifiers { ctrl: true, ..Modifiers::NONE });
    assert!(h.last("1 Dynamic").is_none() && h.last("1 Range").is_none());
}

#[test]
fn double_click_on_a_node_toggles_enabled() {
    let mut h = Harness::new(&[(0, bell(1000.0, 3.0))]);
    h.double_click_at(pos2(gx(1000.0), gy(3.0, 12.0)));
    assert_eq!(h.last("1 Enabled"), Some(0.0));
    assert!(h.last("1 Used").is_none(), "must not create or delete anything");
}

#[test]
fn alt_click_removes_the_band_by_resetting_every_parameter() {
    let mut h = Harness::new(&[(0, bell(1000.0, 3.0))]);
    h.click_at(pos2(gx(1000.0), gy(3.0, 12.0)), Modifiers::ALT);
    let names: Vec<_> = h.rec.lock().sets.iter().map(|(n, _)| n.clone()).collect();
    for n in ["1 Used", "1 Freq", "1 Gain", "1 Q", "1 Type", "1 Dynamic"] {
        assert!(names.iter().any(|x| x == n), "{n} not reset: {names:?}");
    }
    // (the harness builds band 1 with Used = on as its *default*, so "reset" lands on that default)
    assert_eq!(h.last("1 Used"), Some(h.params.band(0).used.default_normalized_value()));
    assert!(h.rec.lock().sets.iter().all(|(n, _)| n.starts_with("1 ")), "only band 1 may change");
}

#[test]
fn delete_key_removes_the_selected_band() {
    let mut h = Harness::new(&[(0, bell(1000.0, 3.0)), (1, bell(4000.0, -2.0))]);
    h.click_at(pos2(gx(4000.0), gy(-2.0, 12.0)), Modifiers::NONE);
    assert_eq!(h.params.prefs.read().selected, Some(1));
    h.clear();
    h.key(egui::Key::Delete);
    assert_eq!(h.last("2 Used"), Some(h.params.band(1).used.default_normalized_value()));
    assert!(h.rec.lock().sets.iter().all(|(n, _)| n.starts_with("2 ")));
    assert_eq!(h.params.prefs.read().selected, None);
}

#[test]
fn clicking_empty_space_deselects() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.click_at(pos2(gx(1000.0), gy(0.0, 12.0)), Modifiers::NONE);
    assert_eq!(h.params.prefs.read().selected, Some(0));
    h.click_at(pos2(gx(300.0), gy(8.0, 12.0)), Modifiers::NONE); // empty space, clear of the band panel
    assert_eq!(h.params.prefs.read().selected, None);
}

#[test]
fn nothing_is_sent_to_the_host_by_just_looking() {
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.mv(pos2(gx(1000.0), gy(0.0, 12.0)), Modifiers::NONE);
    h.mv(pos2(gx(2000.0), gy(4.0, 12.0)), Modifiers::NONE);
    assert!(h.rec.lock().sets.is_empty());
}

#[test]
fn shift_wheel_is_a_fine_adjustment() {
    let mut a = Harness::new(&[(0, bell(1000.0, 0.0))]);
    a.scroll(pos2(gx(1000.0), gy(0.0, 12.0)), 50.0, Modifiers::NONE);
    let mut b = Harness::new(&[(0, bell(1000.0, 0.0))]);
    b.scroll(pos2(gx(1000.0), gy(0.0, 12.0)), 50.0, Modifiers::SHIFT);
    let qa = a.params.band(0).q.preview_plain(a.last("1 Q").unwrap()).ln();
    let qb = b.params.band(0).q.preview_plain(b.last("1 Q").expect("shift+wheel must still change Q")).ln();
    assert!(qb > 0.0 && (qa / qb - 4.0).abs() < 0.8, "{qa} vs {qb}");
}

fn tone_ring(h: &Harness, hz: f32) {
    let mut ring = contour::analyzer::Ring::new();
    let x: Vec<f32> = (0..contour::analyzer::RING).map(|i| 0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / 48_000.0).sin()).collect();
    ring.push(&x, &x);
    *h.shared.ring.lock() = ring;
}

#[test]
fn clicking_a_spectrum_peak_creates_a_bell_on_it() {
    let mut h = Harness::new(&[]);
    tone_ring(&h, 1000.0);
    h.mv(pos2(gx(1060.0), gy(6.0, 12.0)), Modifiers::NONE); // near, not on, the peak
    h.frame(vec![], Modifiers::NONE);
    h.click_at(pos2(gx(1060.0), gy(6.0, 12.0)), Modifiers::NONE);
    assert_eq!(h.last("1 Used"), Some(1.0), "sets: {:?}", h.rec.lock().sets);
    let f = h.params.band(0).freq.preview_plain(h.last("1 Freq").expect("freq"));
    assert!((f - 1000.0).abs() < 60.0, "freq {f}");
    assert!(h.last("1 Gain").map_or(true, |g| h.params.band(0).gain.preview_plain(g).abs() < 0.01), "a grabbed band starts flat");
}

#[test]
fn clicking_away_from_any_peak_just_deselects() {
    let mut h = Harness::new(&[]);
    tone_ring(&h, 1000.0);
    h.click_at(pos2(gx(120.0), gy(6.0, 12.0)), Modifiers::NONE);
    assert!(h.rec.lock().sets.is_empty(), "{:?}", h.rec.lock().sets);
}

#[test]
fn spectrum_grab_is_off_when_the_analyzer_is_off() {
    let mut h = Harness::new(&[]);
    h.params.prefs.write().analyzer_mode = 0;
    tone_ring(&h, 1000.0);
    h.click_at(pos2(gx(1000.0), gy(6.0, 12.0)), Modifiers::NONE);
    assert!(h.rec.lock().sets.is_empty());
}

#[test]
fn a_box_around_two_nodes_selects_both_and_they_move_together() {
    let mut h = Harness::new(&[(0, bell(500.0, 3.0)), (1, bell(2000.0, -3.0)), (2, bell(8000.0, 0.0))]);
    // drag a box from top-left of nodes 1 and 2 to bottom-right of them (not node 3)
    h.drag(pos2(gx(300.0), gy(6.0, 12.0)), pos2(gx(3000.0), gy(-6.0, 12.0)), Modifiers::NONE);
    assert!(h.rec.lock().sets.is_empty(), "a box must not change parameters");
    assert_eq!(h.params.prefs.read().selected, Some(0));
    // now drag node 1 to the right: node 2 follows by the same ratio, node 3 stays
    h.drag(pos2(gx(500.0), gy(3.0, 12.0)), pos2(gx(500.0) + 60.0, gy(3.0, 12.0)), Modifiers::NONE);
    let f1 = h.params.band(0).freq.preview_plain(h.last("1 Freq").expect("band 1 moved"));
    let f2 = h.params.band(1).freq.preview_plain(h.last("2 Freq").expect("band 2 moved with it"));
    assert!(f1 > 560.0 && (f1 / 500.0 - f2 / 2000.0).abs() < 0.02, "{f1} {f2}");
    assert!(h.last("3 Freq").is_none(), "band 3 was outside the box");
}

#[test]
fn delete_removes_every_box_selected_band() {
    let mut h = Harness::new(&[(0, bell(500.0, 3.0)), (1, bell(2000.0, -3.0)), (2, bell(8000.0, 0.0))]);
    h.drag(pos2(gx(300.0), gy(6.0, 12.0)), pos2(gx(3000.0), gy(-6.0, 12.0)), Modifiers::NONE);
    h.clear();
    h.key(egui::Key::Delete);
    let names: Vec<String> = h.rec.lock().sets.iter().map(|(n, _)| n.clone()).collect();
    assert!(names.iter().any(|n| n == "1 Used") && names.iter().any(|n| n == "2 Used"));
    assert!(!names.iter().any(|n| n.starts_with("3 ")));
}

#[test]
fn two_quick_clicks_at_different_places_are_not_a_double_click() {
    // egui counts any two clicks within 0.3 s as a double-click; the editor must not create a band from that
    let mut h = Harness::new(&[(0, bell(1000.0, 0.0))]);
    h.click_at(pos2(gx(1000.0), gy(0.0, 12.0)), Modifiers::NONE);
    h.click_at(pos2(gx(300.0), gy(8.0, 12.0)), Modifiers::NONE);
    assert!(h.last("2 Used").is_none() && h.last("1 Enabled").is_none(), "{:?}", h.rec.lock().sets);
}

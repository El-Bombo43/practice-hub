# Handoff: Practice Hub VST3 plugins (for a local Claude Code session)

Owner: Trent. Repo: `el-bombo43/practice-hub` (branch `main`, push straight to main). Everything for the plugins lives in
`plugins/`. The web app (guitar/drum trainer) is finished and unrelated; ignore it unless asked.

## 1. What exists
Six VST3 plugins in Rust with [nih-plug](https://github.com/robbert-vdh/nih-plug) (pinned rev `de421011f41a6d10fc8c7a6084e4f4dee0143683`
in every Cargo.toml: update it deliberately, never float it) and egui GUIs. Each has its spec at `plugins/crates/<name>/SPEC.md`
(the user's own words: priorities [Must]/[Should], parameter tables, acceptance checks. Treat as the source of truth).

| Crate (folder) | Plugin | Notes |
|---|---|---|
| `contour` | Contour EQ | 16 bands, matched filters, dynamic EQ, sidechain, Pro-Q-style GUI, spectrum analyser |
| `chamber` | Chamber reverb | Plate/Hall/Room FDN tanks, exact RT60, ducking, freeze, animated wireframe "space" |
| `vox-strip` (pkg `vox_strip`) | Vox Strip | gate, HPF, de-esser, comp (Clean/Opto), 2x-oversampled saturation |
| `punch` | Punch | level-independent transient shaper |
| `snap` | Snap | pitch correction: YIN + time-domain PSOLA. Never use the Antares trademark in UI or code |
| `headroom` | Headroom | BS.1770/R128 loudness + true-peak meter; Full/Strip/Compact windows; for the user's friend (macOS) |
| `scaffold-gain` | pipeline test gain | can be deleted |

Shared code: `plugins/shared/common` (package `phub_common`): theme + widgets (`ui.rs`), DSP helpers (`dsp.rs`), parameter/format
helpers, and an offscreen egui renderer (`offscreen.rs`). Contour and Chamber predate it and keep their own copies of the GUI code.
`plugins/vendor/nih_plug_egui` is nih-plug's egui adapter with two local changes (see its README entry): `set_requested_size` is `pub`,
and the new window size is stored BEFORE `request_resize()` (upstream order made Live resize to the old size: the "window breaks until
reopened" bug, fixed in commit "Fix window resize").

## 2. Build, test, run
```
cd plugins
cargo test --workspace --release            # all tests (a few take 10-15 s: Chamber acceptance/fuzz)
cargo xtask bundle <package> --release       # e.g. headroom, vox_strip; one package per call -> target/bundled/<Name>.vst3
cargo xtask bundle-universal headroom --release   # macOS only
```
Linux needs the apt packages listed in `.github/workflows/plugins.yml`. Windows `.vst3` files come from CI
(`.github/workflows/plugins.yml`: jobs `test` (unit tests + Linux bundle + pluginval strictness 5 with `--skip-gui-tests`, no OpenGL on
the runner), `windows` (artifact `windows-vst3`), `macos` (Headroom only, universal, ad-hoc signed zip `headroom-mac`)).
CI takes 10-25 min (runner queue varies). CI bundles every package whose manifest path contains `/crates/`, so a new plugin = new
folder under `crates/`, an entry in `plugins/bundler.toml`, a unique 16-byte `VST3_CLASS_ID`.
Offscreen screenshots (no window needed): set `CONTOUR_RENDER_DIR`, `CHAMBER_RENDER_DIR`, `VOX_RENDER_DIR`, `PUNCH_RENDER_DIR`,
`SNAP_RENDER_DIR` or `HEADROOM_RENDER_DIR` to a folder and run the crate's `--test render`; open the PNGs.
The cloud sandbox cannot run a DAW, hear audio, or open real windows: everything audible or on-screen is verified by the user.

## 3. Status
- All six build, pass their tests, and passed pluginval in CI (Linux). The user is testing on Windows (Ableton Live 12 Standard) and
  reports back. First report: window resize broke (fixed, retest pending). Headroom's macOS build and pluginval-on-Mac had not been
  exercised by a human; the friend tests it.
- Nothing has been listened to yet beyond the user's first load tests. Expect tuning rounds, especially Chamber (taste) and Snap.

## 4. Known deviations and weak spots (be honest about these)
- **Snap**: detection needs ~25 ms, so in Track mode (10 ms latency) note starts are late; Mix mode (40 ms) hides it. PSOLA loses a few
  dB above ~3.5 kHz on a synthetic vowel (regression threshold 4.5 dB mean over 16 harmonics in `tests/engine.rs`); judge on real voice.
  Note strip is a fixed list, not aligned with the scrolling graph lanes. Mode/Voice changes rebuild the engine on the audio thread
  (allocation; brief dropout, allowed by the spec). CPU about 3% of a core. Key/Scale: a named scale overrides the 12 toggles, the
  editor writes the toggles when the user picks a key/scale.
- **Chamber**: stability test uses 500 ms energy trend (late tail beats about 1 dB per 50 ms); frozen level is within about 1 dB of
  the pre-freeze level (Damping is bypassed while frozen). RT60 is exact at 1 kHz by construction (per-line decay filter designed at
  1 kHz and 125 Hz, 2nd-order Butterworth damping).
- **Vox Strip**: saturation dry/wet blend ignores the oversampler's ~1 sample delay (fine at Sat Mix 100, slightly combs when blended).
- **Headroom**: unverified in a real host: window resize via the mode buttons and drag grip (depends on Live honouring
  `request_resize`), macOS build/sign/pluginval, bar.beat positions vs Live's transport (unit-tested only with synthetic beats).
  Glyphs like arrows are avoided because the bundled fonts lack them.
- **Contour**: riskiest unverified behaviours in Live are mouse-wheel, keyboard focus and DPI scaling. See `crates/contour/SPEC.md`.
- Spec file locations differ from what the specs say (`plugins/<name>/SPEC.md`): they live beside the crate in `crates/<name>/`.
- Audio-thread allocation audit: removed per-hop allocations in Snap (detector and mark search) and Headroom (loudness histograms
  replace stored blocks; snapshot publish reuses memory). Remaining known: Snap engine rebuild on Mode/Voice change, Headroom pushes
  to the overs list (pre-reserved 4096).

## 5. Next steps (suggested order)
1. Get the user's test reports (format in each SPEC.md: Chamber section 6, Snap section 5, others section "Studio check"). Fix in that order.
2. Confirm the resize fix in Live for every plugin (S/M/L buttons; Headroom's Compact/Strip/Full). If Live ignores `request_resize`,
   fall back to fixed windows per plugin and remove the mode buttons.
3. Chamber tuning from listening notes (decay shape, early reflections level, brightness, modulation); gain staging constants live in
   `crates/chamber/src/dsp/tank.rs` (`PLATE`/`HALL`/`ROOM` configs: `out_gain`, `in_gain`, delays, `er_*`).
4. Snap: real-voice quality (try alternate window shapes in `place_grain`, mark choice in `extend_marks`), Track-mode lateness, scale/hysteresis feel.
5. Headroom: friend's Mac feedback; compare numbers to a trusted meter within 0.2 LU.
6. Polish: Contour/Chamber could move onto `phub_common::ui` to remove duplicate GUI code; drag-to-resize is a coarse step (not live-drag).
7. Licensing: sharing Headroom binaries with the friend means sharing source under GPL-3.0 (nih-plug's VST3 bindings). Give him repo
   access or a source zip with the build.

## 6. Working rules that have held up
- Read `PROGRESS.md` (max 15 lines; update it at session end). Credits are limited: make technical decisions yourself, record
  non-obvious ones in `PROGRESS.md`, no browser automation.
- Write DSP as plain Rust with tests first (acceptance checks from the spec become `#[test]`s), then the plugin glue, then the GUI,
  then check the GUI with the offscreen renderer. Keep parameter IDs permanent once the user has used a plugin in saved sets (adding
  is fine, renaming or removing breaks sets).
- Commit and push to `main`; check CI with `gh run list --workflow plugins.yml`.

# VST3 plugins (Rust, nih-plug)

Windows `.vst3` plugins for Ableton Live, built with [nih-plug](https://github.com/robbert-vdh/nih-plug) (pinned to an exact commit in each `Cargo.toml`; update the `rev` deliberately). See `../docs/vst3-plugin-spec-guide.md` for how to write a spec.

## Layout
- `crates/<plugin>/` one crate per plugin. `src/dsp.rs` holds plain, unit-tested DSP; `src/lib.rs` is the plugin glue and GUI.
- `crates/scaffold-gain/` pipeline check: a gain with an egui window and meter. Copy it to start a real plugin (new folder, new `name` in `Cargo.toml`, new unique `VST3_CLASS_ID` of 16 bytes, new entry in `bundler.toml`).
- `xtask/` the `cargo xtask bundle` packager. `bundler.toml` gives bundles their display names.

## Commands (from this folder)
```
cargo test --workspace --release          # DSP unit tests
cargo xtask bundle <package> --release    # builds target/bundled/<Name>.vst3 for the machine you are on
```
Linux builds need the system libraries listed in `.github/workflows/plugins.yml`.

## CI (`.github/workflows/plugins.yml`)
- `test`: unit tests, a Linux bundle of every plugin, and **pluginval** at strictness 5 (load, parameters, state, audio processing, threading). GUI tests are skipped because CI has no OpenGL: check each editor window by hand.
- `windows`: a Windows x64 `.vst3` per plugin, uploaded as the **windows-vst3** artifact on the run (Actions tab → the run → Artifacts).

## Install in Ableton (Windows)
1. Download and unzip the `windows-vst3` artifact.
2. Copy each `*.vst3` folder to `C:\Program Files\Common Files\VST3\`.
3. Ableton → Preferences → Plug-ins → turn on **Use VST3 Plug-in System Folders** → Rescan.
4. The plugins are unsigned. If Windows shows SmartScreen on first load, choose "More info → Run anyway".

## Licence
nih-plug's VST3 support is GPL-3.0. Sharing binaries means sharing source under the GPL.

## Notes
- `vendor/nih_plug_egui` is nih-plug's egui adapter with one method made public (see `vendor/README.md`). CI bundles only the packages under `crates/`.
- `crates/contour/` is the EQ (spec: `crates/contour/SPEC.md`); `crates/chamber/` is the reverb (Plate / Hall / Room; spec: `crates/chamber/SPEC.md`). Specs live beside their crates because CI bundles whatever is under `crates/`.

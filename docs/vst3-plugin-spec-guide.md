# VST3 plugins for Ableton: what the spec needs to know

Hand this to whoever writes the spec (a separate chat is fine). It lists the **constraints, defaults and open questions**, so the spec can spend its words on **what each plugin does and how it looks**.

---

## 1. The setup in one paragraph

Two VST3 plugins, written in **Rust with [nih-plug](https://github.com/robbert-vdh/nih-plug)**, living in a `plugins/` folder inside the existing `practice-hub` repo (a new GitHub repo needs the owner at a desktop to grant the Claude GitHub App access, so we skip that). Code is written and tested on a Linux cloud machine. **Windows `.vst3` files are built by a GitHub Actions Windows runner** and downloaded from the workflow's artifacts. The target host is **Ableton Live on Windows**. Nobody can hear or click the plugin until it is installed on the studio PC, so the spec must be precise enough to build from, and the first listen is a feedback round.

## 2. What the builder (Claude) decides, and what the spec must decide

| The builder decides (don't spend spec words here) | The spec must decide |
|---|---|
| Rust/nih-plug project layout, build, CI, packaging | What each plugin **does** (sound, behaviour) |
| Parameter smoothing, denormal handling, real-time safety | **Every parameter**: name, range, default, units, scaling |
| Oversampling plumbing, latency reporting | **How it looks**: layout, colours, controls, meters |
| Unit tests, offline rendering checks, the plugin validator | **Signal flow** and any algorithm preferences |
| Windows packaging and install instructions | What "done" means (acceptance checks) |

## 3. Hard facts and limits

**Format and host**
- **VST3 only.** Ableton Live supports VST3 (and AU on Mac). It does **not** load CLAP or VST2 in current versions, so don't spec those.
- Target: **Windows 64-bit**. Ableton Live 11 or 12 (the spec should state the version if known).
- Plugins are **unsigned**. Windows may show a SmartScreen warning when the downloaded file is first opened. Fine for personal use.

**What can be verified here (cloud, Linux)**
- DSP unit tests: impulse responses, sine sweeps, null tests, gain/level maths, parameter ranges, tempo-sync maths.
- Offline rendering of test signals through the plugin code, checked numerically (peak, RMS, THD, frequency response, latency, no NaN/Inf, no clicks).
- Building a Linux VST3 and running it through **pluginval** (a headless validator that loads it and exercises the API, state save/load and automation).
- Layout of the GUI as code. It can be rendered to an image offline if needed.

**What cannot be verified here**
- How it **sounds**. How it **feels**. The GUI as actually drawn on Windows at 100/125/150% scaling. Behaviour inside Ableton (automation lanes, preset recall, device chain, freezing, undo). Real CPU load. Expect a feedback loop: build → install → listen → report → fix.

**Licensing**
- nih-plug's VST3 support is built on bindings licensed **GPL-3.0**. Personal use is fine. **Distributing binaries means offering source under the GPL.** If the plugins might ever be sold or shared closed-source, say so in the spec now (it changes the framework choice).
- nih-plug itself is ISC-licensed. Third-party DSP code or algorithms the spec asks for should be named with their licences.

## 4. Plugin type, I/O and audio basics (state these per plugin)

- **Category:** audio effect / instrument (synth, takes MIDI, makes audio) / analyser (audio passes through unchanged, shows something).
- **Channels:** mono-in/mono-out, mono-in/stereo-out, stereo/stereo. (Guitar sources are often mono; say whether a mono input should feed both channels.)
- **Sidechain input:** yes/no. nih-plug supports an auxiliary input.
- **MIDI:** none / MIDI in (instrument, or MIDI-controlled effect) / MIDI out.
- **Sample rates:** assume 44.1 to 192 kHz; **block sizes** from 32 to 4096 samples. Anything rate-dependent (filters, delay times) is computed from the host rate.
- **Latency:** if the algorithm needs lookahead or oversampling, the plugin reports it so Ableton compensates. The spec should say whether added latency is acceptable (it usually is, except for live monitoring).
- **Bypass:** host bypass is always supported. Say if the plugin needs its own "bypass with tail" behaviour.
- **Dry/wet and input/output gain:** common, cheap to add. Say if wanted.
- **CPU budget:** default target is "light" (well under 5% of a core at 48 kHz on a modern PC). Say if something is allowed to be heavy.

## 5. Parameters: what to specify

Every automatable knob/switch needs a row. A good format:

| Name (shown) | Range | Default | Unit | Scaling | Step / type | Smoothing | Notes |
|---|---|---|---|---|---|---|---|
| Drive | 0 to 100 | 30 | % | linear | continuous | 20 ms | |
| Cutoff | 20 to 20000 | 1000 | Hz | log (skew) | continuous | 10 ms | |
| Mode | A / B / C | A | – | – | enum | none | |
| Sync | off / on | off | – | – | bool | none | |

Also answer:
- **Display format** (for example `-12.0 dB`, `440 Hz`, `1/8 dotted`, `35 %`).
- **Which parameters are host-automatable** (default: all).
- **Tempo-synced values** (note divisions)? They use the host's tempo and transport.
- **Parameter IDs are permanent**: once a plugin is released and used in Ableton projects, renaming or removing an ID breaks saved sets. The spec should therefore be final about the parameter list before the first build is used in real projects (adding parameters later is fine).
- **Presets:** Ableton saves the device's full state with the Live set and its own device presets. nih-plug has no built-in preset browser. If factory presets are wanted, list them (name plus parameter values); they would appear as a simple in-plugin menu.

## 6. How it looks (the part to spend time on)

**What's easy and safe**
- A fixed-size window with flat or lightly shaded vector controls: knobs, sliders, toggles, dropdowns, LED-style meters, level bars, waveform and spectrum displays, XY pads, labels.
- A colour palette, fonts (TTF/OTF can be bundled), rounded panels, gradients, glow effects, animated meters.
- A look matching the existing app is possible and cheap: dark panels (`#0a0a0b` / `#141416`), a single accent (`#d4ff3a`), Sora for text and IBM Plex Mono for numbers. See `design/*.html` in this repo for the mockup style used so far.

**What's costly (say so if you want it)**
- Photoreal / skeuomorphic textures and large bitmap filmstrips. Possible, but needs image assets and takes more rounds.
- Resizable windows (needs layout that adapts; fixed size is much simpler).
- Complex custom visualisers (3D, particle effects).

**GUI framework (builder's default):** `nih_plug_egui` (immediate-mode, easy custom drawing). Alternatives: `nih_plug_vizia` (CSS-like styling), `nih_plug_iced`. If no GUI is wanted, a generic parameter list still works (Ableton also shows its own generic controls).

**What the spec should give for the look**
1. **Window size** in pixels at 100% scale (for example 480 × 320). It scales with Windows DPI.
2. **Layout sketch**: an HTML mockup like `design/picking.html`, a screenshot/photo of a reference, or a clear text description of the grid ("top row: three big knobs; below: a 2-row meter strip").
3. **Control style**: knob (arc, dot, or line indicator), slider, toggle button style, value readout (always visible or on hover).
4. **Colours and fonts** (or "match the practice-hub palette").
5. **Meters and displays**: what they show (input level, gain reduction, spectrum, waveform, tuner needle), their range and ballistics (peak vs RMS, fall speed), and whether they update at 30 or 60 fps.
6. **Interaction details**: double-click to reset to default? Fine-adjust with Shift/Ctrl-drag (a standard nih-plug behaviour)? Mouse-wheel? Right-click to type a value?
7. **Text**: plugin title, section labels, units, tooltips.

## 7. Behaviour edge cases worth a line each in the spec

- Silence in: silence out, or noise/tail? Tail length?
- Level and gain staging: unity gain at defaults? Maximum output level / clipping behaviour (hard clip, soft clip, or none and let the host handle it)?
- Mono input into a stereo plugin; channel linking in stereo processors.
- Sample-rate and tempo changes while playing; transport stop/start; looping.
- Parameter changes while audio runs: smoothing time, whether clicks are acceptable.
- Reset behaviour (on transport start, on reset by the host).
- Anything stateful the user can *see* (a learned value, a tuned note) and whether it persists in the saved project.

## 8. Acceptance checks (the "done when" list)

State these in the spec in plain terms, so each can be tested. Examples:
- "With Drive at 0 and Mix at 100%, the output is within ±0.1 dB of the input at all frequencies."
- "The gate opens within 2 ms of a pick attack and never chatters on a decaying note."
- "No output sample is ever NaN, infinite, or above +6 dBFS with a 0 dBFS input."
- "State recalls exactly after saving and reloading a Live set."
- "Loads and passes pluginval at strictness 5."
- "Reports its latency correctly (checked by a null test)."

## 9. Install and test loop on the studio PC

1. The GitHub Actions run produces a `.vst3` (a folder bundle on Windows) as a downloadable artifact.
2. Copy it to `C:\Program Files\Common Files\VST3\`.
3. In Ableton: **Preferences → Plug-ins**, turn on **Use VST3 Plug-in System Folders**, then **Rescan**.
4. Load it on a track; try automation, save the set, reopen it.
5. Report back with: what you did, what you expected, what happened, **Live version, sample rate, buffer size, Windows version**, and a screenshot for GUI issues.

## 10. Process and timing

- Order of work: (1) scaffold the workspace and CI and prove a Windows build loads in Ableton using a trivial placeholder, (2) plugin A: DSP and tests, then GUI, (3) plugin B the same way, (4) listen and iterate.
- Everything is committed to `practice-hub` under `plugins/`. The next session picks up from the repo, so the spec should be saved in the repo too (for example `plugins/<name>/SPEC.md`).
- Cloud credits are limited. A precise spec means fewer feedback rounds. A good order of priority in the spec: **parameters table, signal flow, GUI sketch, acceptance checks, then everything else.**

## 11. Spec template (copy per plugin)

```
# <Plugin name>
**One line:** what it does, for whom.
**Category:** effect | instrument | analyser        **Channels:** in/out        **Sidechain:** y/n   **MIDI:** none/in/out

## Sound / behaviour
Describe the processing. Signal flow as a list or diagram. Algorithms or references (plugins or papers it should resemble).

## Parameters
(table as in section 5)

## Look
Window size. Layout (mockup/sketch). Controls. Colours/fonts. Meters/displays. Interactions.

## Edge cases
(from section 7)

## Acceptance checks
(from section 8)

## Out of scope
What it deliberately does not do.
```

## 12. Questions to answer before handing the spec back

1. What does each plugin do, in one sentence, and what is the first thing you'll play through it?
2. Mono or stereo for each? Instrument or effect? Any MIDI or sidechain?
3. Fixed window size and rough layout for each. Match the app's look, or something different?
4. The full parameter table for each (ranges, defaults, units, scaling).
5. Any reference plugin or sound ("like X but Y")?
6. Is there any chance these will be shared or sold? (affects licensing, see section 3)
7. Ableton Live version, Windows version, typical sample rate and buffer size.
8. Anything that must work live (low latency, live monitoring) versus mixing/production use?

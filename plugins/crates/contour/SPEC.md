# Contour

**One line:** a mixing EQ with a Pro-Q-style workflow (drag nodes on a live spectrum) and per-band dynamic EQ, for Trent's mixes in Ableton.

**Category:** audio effect  **Channels:** stereo in/out (also a mono in/out layout)  **Sidechain:** yes, stereo aux input (used only by dynamic bands set to External)  **MIDI:** none

**Lives at:** `practice-hub/plugins/contour/` (save this file as `plugins/contour/SPEC.md`).
*(Built as `plugins/crates/contour/`, the workspace layout used by every plugin here.)*

**Context**
- Host: Ableton Live 12 **Standard**, Windows 64-bit. VST3 only.
- Use: mainly mixing, but it must also be usable on a live guitar chain, so **zero latency** in v1.
- Personal use only, never sold or distributed. GPL-3.0 via nih-plug's VST3 bindings is fine.
- Sample rate and buffer size on the studio PC: not yet known. Assume 48 kHz / 256 samples for testing, and support 44.1 to 192 kHz and 32 to 4096-sample blocks.

**Priority tags:** **[Must]** is required for the first build Trent listens to. **[Should]** goes in if it fits, otherwise next round. Cut Should items before cutting Must items.

---

## 1. Sound and behaviour

### Signal flow
```
Input ─┬─────────────► Band 1 ► Band 2 ► … ► Band 16 ► Auto Gain ► Output Gain ► Out
       │                  ▲ (each band: static filter, gain modulated by its detector if Dynamic is on)
       │  Detector source per band: Internal = this band's input; External = aux sidechain
       └──► Analyzer taps: Pre (input) and Post (after Output Gain)
```
- Bands run in **series**, in band-number order. Bands whose **Used** or **Enabled** is off are skipped entirely: no processing, no CPU.
- Stereo: both channels use identical, linked settings. Each dynamic band uses **one shared detector** (max of L/R), so the stereo image doesn't shift.
- Mono layout: mono in, mono out. If Ableton feeds mono into the stereo layout, both channels are simply identical.

### Filter types [Must]
| Type | Uses Gain | Uses Q | Uses Slope | Dynamic allowed |
|---|---|---|---|---|
| Bell | yes | width | – | yes |
| Low Shelf | yes | resonance/steepness | – | yes |
| High Shelf | yes | resonance/steepness | – | yes |
| Tilt Shelf (pivot at Freq) | yes | steepness | – | yes |
| Low Cut | – | resonance at corner | yes | no |
| High Cut | – | resonance at corner | yes | no |
| Notch | – | width | – | no |
| Band Pass | – | width | – | no |

- **Analog-matched response, no cramping near Nyquist.** A bell at 15 kHz at 44.1 kHz must look and sound like it does at 96 kHz. Use a matched-magnitude design (for example Vicanek's matched second-order filters) or oversampling. The builder chooses, as long as latency stays 0.
- Cut slopes: 6, 12, 18, 24, 36, 48, 72, 96 dB/oct, built by cascading. Q sets the resonance of the corner section only (Q 0.71 = Butterworth-flat).
- Zero-latency minimum-phase only. Linear and natural phase are out of scope for v1.

### Dynamic EQ [Must]
- When a band's **Dynamic** is on, its gain moves between its **static Gain** and **Gain + Range**, driven by how far the detected level exceeds **Threshold**.
  - Negative Range = the band cuts harder when the level rises (taming harsh notes, boomy palm mutes).
  - Positive Range = the band boosts when the level rises (expansion).
- **Detector:** the detector signal (Internal input or External sidechain) is filtered through a band-pass with the same Freq and Q as the band, so it responds to energy in that band. Peak-ish envelope with the band's Attack and Release. Soft knee, fixed at 6 dB. Ratio is implicit: full Range is reached 12 dB above Threshold.
- The GUI shows the live dynamic gain (see Look). That value is also exposed for metering, but it is not a parameter.

### Global
- **Output Gain** after all bands.
- **Auto Gain [Should]:** static loudness compensation computed from the current curve (for example, the inverse of the curve's average gain weighted over 100 Hz to 10 kHz). It updates when settings change, never per-sample from audio. It ignores dynamic movement.
- **Gain Scale:** multiplies every band's static Gain and dynamic Range (0 to 200%). It's a quick "more/less of this curve" control.
- **Band Solo [Must]:** audition one band. Output becomes the input through a band-pass at that band's Freq/Q (for a cut: the frequencies it removes). It's GUI state only: not a parameter, not saved with the set, cleared when the GUI closes.

---

## 2. Parameters

Parameter IDs are permanent once Trent uses the plugin in real sets. The IDs below are final. Adding parameters later is fine.

### Per band (n = 1 to 16), shown in Ableton as "n Freq", "n Gain", etc.
| ID | Shown name | Range | Default | Unit / display | Scaling | Type | Smoothing |
|---|---|---|---|---|---|---|---|
| `b{n}_used` | n Used | off/on | off | – | – | bool | none |
| `b{n}_on` | n Enabled | off/on | on | – | – | bool | 10 ms crossfade |
| `b{n}_type` | n Type | Bell, Low Shelf, High Shelf, Tilt Shelf, Low Cut, High Cut, Notch, Band Pass | Bell | – | – | enum | 10 ms crossfade |
| `b{n}_freq` | n Freq | 10 to 30 000 | 1000 | `82.4 Hz`, `1.25 kHz` | log | continuous | 20 ms |
| `b{n}_gain` | n Gain | −30 to +30 | 0 | `-3.50 dB` | linear | continuous, 0.01 step | 20 ms |
| `b{n}_q` | n Q | 0.025 to 40 | 1.0 | `Q 1.00` | log | continuous | 20 ms |
| `b{n}_slope` | n Slope | 6, 12, 18, 24, 36, 48, 72, 96 | 12 | `24 dB/oct` | – | enum | 10 ms crossfade |
| `b{n}_dyn` | n Dynamic | off/on | off | – | – | bool | 10 ms crossfade |
| `b{n}_thresh` | n Threshold | −60 to 0 | −20 | `-20.0 dB` | linear | continuous | 20 ms |
| `b{n}_range` | n Range | −30 to +30 | −6 | `-6.0 dB` | linear | continuous | 20 ms |
| `b{n}_attack` | n Attack | 0.5 to 200 | 10 | `10.0 ms` | log | continuous | none |
| `b{n}_release` | n Release | 10 to 2000 | 150 | `150 ms` | log | continuous | none |
| `b{n}_ext` | n Ext Sidechain | off/on | off | – | – | bool | none |

**Global**
| ID | Shown name | Range | Default | Unit / display | Scaling | Type | Smoothing |
|---|---|---|---|---|---|---|---|
| `out_gain` | Output | −24 to +24 | 0 | `0.0 dB` | linear | continuous | 20 ms |
| `auto_gain` | Auto Gain | off/on | off | – | – | bool | 50 ms |
| `gain_scale` | Gain Scale | 0 to 200 | 100 | `100 %` | linear | continuous | 20 ms |

- All of the above are host-automatable.
- Changing Freq/Gain/Q during playback must not click or zipper. Use interpolated coefficients or a smoothed parameter recompute. The builder chooses.

**GUI-only state** (saved with the set via persisted editor state, not automatable): selected band(s), window size preset, graph display range, analyzer mode/speed. Band solo is **not** saved.

### Factory presets [Should] (simple menu in the top bar)
| Name | Bands |
|---|---|
| Flat | none used |
| Guitar – Tighten Lows | Low Cut 80 Hz 24 dB/oct Q 0.71; Bell 180 Hz −2 dB Q 1.4 Dynamic, Thresh −24, Range −4, Atk 5, Rel 120; Bell 3.2 kHz −2 dB Q 2.0 |
| Vocal – Clarity | Low Cut 90 Hz 18 dB/oct; Bell 300 Hz −2.5 dB Q 1.2; High Shelf 10 kHz +2 dB Q 0.7; Bell 6.5 kHz 0 dB Q 3, Dynamic Range −5 (de-ess style) |
| Bass – Clean | Low Cut 35 Hz 24 dB/oct; Bell 250 Hz −3 dB Q 1.0; Bell 800 Hz +1.5 dB Q 1.2 |

---

## 3. Look

### Window [Must]
Three fixed sizes chosen from S / M / L buttons in the top bar. Contents scale proportionally. Not freely resizable.
- **S** 760 × 440 **M** 960 × 560 (default) **L** 1280 × 740, at 100% Windows scaling. Scales with Windows DPI.

### Layout
```
┌─────────────────────────────────────────────────────────────────────────┐
│ CONTOUR   [Preset ▾]          [Analyzer: Pre+Post ▾] [±12 ▾]  S M L     │  top bar, 36 px
├─────────────────────────────────────────────────────────────────────┬───┤
│                                                                     │ 0 │
│        GRAPH: spectrum + EQ curve + band nodes                      │   │  dB scale
│        (fills the window)                                           │-12│  (EQ left, analyzer right)
│                    ┌───────────────────────────────┐                │   │
│                    │ floating band panel (selected)│                │   │
│                    └───────────────────────────────┘                │   │
├─────────────────────────────────────────────────────────────────────┴───┤
│ 20   50   100   200   500   1k   2k   5k   10k   20k     Gain Scale ◯  Auto Gain ▢  Output ◯ │ bottom bar
└─────────────────────────────────────────────────────────────────────────┘
```

### Graph [Must]
- X: log frequency, 10 Hz to 30 kHz. Grid lines at 20/50/100/200/500/1k/2k/5k/10k/20k.
- Y (EQ): display range selectable ±3 / ±6 / ±12 / ±30 dB (default ±12). Grid every 3 or 6 dB, as appropriate.
- **EQ curve:** the combined response as a 2 px line in the accent colour, with a faint fill to 0 dB. Each band's own curve is drawn faintly in its band colour, and brighter when hovered or selected.
- **Dynamic bands:** a translucent "range" area between static gain and gain+range. The live dynamic curve animates on top, so Trent can see it working.
- **Nodes:** 10 px circles in band colour, labelled with the band number. Selected = lime ring. Disabled = hollow, 40% opacity.

### Analyzer [Must, Pre/Post modes Should]
- Post spectrum filled at low opacity; Pre spectrum as a thin muted outline. Modes: Off / Post / Pre / Pre+Post (default Post).
- Range −90 to 0 dBFS (right-hand scale). Tilt 4.5 dB/oct around 1 kHz, so a mix looks roughly flat. Smoothing about 1/6 octave.
- Speed: Slow / Medium (default) / Fast, controlling decay. Updates at 60 fps.
- Hovering the graph shows a readout at the cursor: frequency, note name and dB (for example `82.4 Hz · E2 · −18 dB`).
- Stops running when the editor is closed (no CPU cost).
- **[Should] Spectrum grab:** while hovering, the nearest prominent peak is highlighted; click it to create a bell band there.

### Band panel [Must]
Appears near the bottom of the graph for the selected band, Pro-Q style.
- Type dropdown · Slope dropdown (cut types only) · **Freq / Gain / Q** knobs with value fields · Solo (headphones icon) · Enable · Delete.
- Dynamic section (shown when type allows; greyed otherwise): Dynamic toggle · Threshold · Range · Attack · Release · Ext SC toggle · a small gain-reduction/expansion bar.
- Value fields: click to type, and accept units ("2k", "2.5 kHz", "-3").
- Knobs: arc indicator in band colour on a dark track, value text below in IBM Plex Mono.

### Interactions (the Pro-Q feel) [Must unless marked]
| Action | Result |
|---|---|
| Double-click empty graph | Create a band at that freq/gain. Left 5% of graph → Low Cut; right 5% → High Cut; otherwise Bell. Takes the lowest unused band slot. |
| Drag node | Freq (horizontal) + Gain (vertical). Cut/Notch/Band Pass nodes move horizontally only (vertical = Q for cuts). |
| Shift + drag | Fine adjust (1/10 speed) |
| Ctrl + drag | Constrain to the axis moved first |
| Mouse wheel on node | Q. Shift + wheel = fine. |
| Ctrl + wheel on node | Dynamic Range (turns Dynamic on if off and the type allows) |
| Double-click node | Toggle band Enabled |
| Alt + click node, or Delete key | Remove band (sets Used off and resets its parameters to defaults) |
| Right-click node | Menu: type list, slope, Solo, Dynamic on/off, Ext SC, Reset band, Delete |
| Click + hold the solo button | Solo while held. Click toggles latched solo. |
| Drag a box on empty space [Should] | Multi-select. Dragging any selected node moves all of them (relative). |
| Click empty space | Deselect |
| Double-click any knob | Reset that parameter to default |
| Hover node | Tooltip: `Band 3 · Bell · 1.25 kHz · −3.5 dB · Q 1.40` |

### Colours and fonts (hybrid)
- Panels from the practice-hub palette: background `#0a0a0b`, panels `#141416`, grid lines `#222226`, text `#e8e8ea`, muted text `#8a8a90`, accent `#d4ff3a` (combined curve, selection ring, active toggles).
- Band colours (cycle by band number): `#ff6b6b`, `#ffb347`, `#ffe66d`, `#7bd88f`, `#4ecdc4`, `#5aa9ff`, `#a78bfa`, `#f472b6`.
- Fonts: **Sora** for labels, **IBM Plex Mono** for numbers. Both are OFL-licensed and bundled.
- Flat vector style, rounded panels (6 px radius), with a subtle glow on the combined curve. No bitmaps.

---

## 4. Edge cases
- **No bands used:** output is bit-identical to input (except Output Gain / Gain Scale if changed).
- **Silence in:** silence out. No tail beyond normal filter ring-out. Detectors decay to rest.
- **Gain staging:** unity at defaults. No internal clipping or limiting. The float output passes through and the host handles overs.
- **Freq above Nyquist** (for example 30 kHz at 44.1 kHz): clamp internally to just below Nyquist. Nodes still display at their set value.
- **Sample-rate change / host reset:** recompute coefficients and clear filter and detector state. No pops.
- **Transport stop/start, looping:** no special behaviour. Processing is continuous.
- **Ext Sidechain on but nothing routed:** the detector sees silence, so dynamic gain stays at static Gain.
- **Type change while playing:** crossfade old/new filter over 10 ms, without a click.
- **Extreme settings** (Q 40, +30 dB at 10 Hz, 96 dB/oct stacks): stable, no NaN/Inf, no denormal CPU spikes.

## 5. Acceptance checks (done when…)
1. With no bands used, output nulls against input to −140 dBFS or better (bit-identical).
2. Bell, 1 kHz, +6 dB, Q 1: response matches the analog prototype within ±0.1 dB from 20 Hz to 20 kHz at 44.1, 48 and 96 kHz.
3. Bell, 15 kHz, +6 dB, Q 2 at 44.1 kHz: within ±0.5 dB of the analog prototype up to 20 kHz (no cramping).
4. Each cut slope measures within ±0.5 dB/oct of nominal, one octave past the corner.
5. Sweeping Freq 20 Hz → 20 kHz over 1 s on a sine produces no clicks (no sample-to-sample jump beyond the smooth envelope; checked numerically).
6. Dynamic: a −20 dBFS sine at the band's freq with Threshold −30 and Range −6 settles to the static gain −6 dB (±0.2 dB) within 5× Attack; it returns to static gain within 5× Release after the tone stops.
7. Ext Sidechain: with the band's input steady and a burst on the sidechain, the gain moves only during the burst.
8. Gain Scale 0% → flat response; 200% → doubled dB at every band.
9. Reported latency is 0 samples.
10. No output sample is NaN/Inf. Output ≤ input peak + the curve's max boost + 0.5 dB on any test signal.
11. CPU: 16 bands active, all dynamic, stereo, 48 kHz, 256-sample buffer → well under 5% of one core (benchmark in CI on Linux as a proxy).
12. Passes pluginval at strictness 5. State and GUI state (selected band, size preset, display range) recall exactly after save/reload.
13. On the studio PC (Trent checks): loads in Live 12; nodes drag smoothly at each size preset; automation of n Freq / n Gain plays back cleanly; a saved set reopens identical.

## 6. Out of scope (v1)
Linear/natural phase modes · per-band stereo placement (L/R/Mid/Side) · spectral dynamics · EQ match · collision/masking display · freely resizable window · MIDI learn · in-plugin undo (Ableton's undo covers parameter changes) · piano-roll keyboard strip · more than 16 bands (adding slots later is possible by adding new IDs).

## 7. Next after this
Feedback round on Contour, then the reverb plugin gets its own spec, using the same palette and conventions.

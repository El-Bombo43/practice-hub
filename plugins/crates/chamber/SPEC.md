# Chamber

**One line:** an algorithmic reverb (Plate / Hall / Room) for mixing guitars, vocals and drums in Ableton, with tail EQ, modulation, ducking, freeze, and an animated "space" display.

**Category:** audio effect  **Channels:** stereo in/out (also a mono-in/stereo-out layout)  **Sidechain:** no  **MIDI:** none

**Lives at:** `practice-hub/plugins/chamber/` (save this file as `plugins/chamber/SPEC.md`). Build this **after Contour**, reusing its workspace, CI, GUI theme code and fonts.

**Context**
- Host: Ableton Live 12 **Standard** (no Hybrid Reverb, so Chamber is the main "good" reverb), Windows 64-bit, VST3 only.
- Use: mainly mixing, on insert or on a Return track at 100% wet. It must also be playable live, so **0 samples latency** (pre-delay is not latency).
- Personal use only, never distributed. GPL-3.0 via nih-plug is fine.
- Same conventions as Contour: practice-hub palette, Sora/IBM Plex Mono, S/M/L fixed window sizes, double-click resets a knob, Shift-drag for fine adjust, click a value to type it.

**Priority tags:** **[Must]** is required for the first build Trent listens to. **[Should]** goes in if it fits, otherwise next round.

**Expect several listening rounds.** Unlike an EQ, a reverb's quality is mostly taste. Section 6 has a short listening-report format so each round gives the builder usable feedback.

---

## 1. Sound and behaviour

### Signal flow
```
In ─┬──────────────────────────────────────────────────────────────► Dry ──┐
    │                                                                      ├─► Mix ─► Output Gain ─► Out
    └─► Input Low Cut/High Cut ─► Pre-delay ─► Diffusion ─► Tank (mode) ─► Width ─► Duck ─► Wet ┘
                                                          ▲    │
                                         Freeze ──────────┘    └─► Early reflections (Hall/Room) summed in
    Duck detector ◄── dry input level
```
- Tail EQ (Low Cut, High Cut) filters what goes **into** the tank, so the tail stays clean. Damping and Bass Decay shape the tail **inside** the tank, so highs and lows decay at different rates.
- Mono input: summed to mono into the tank; output is always stereo (decorrelated L/R).
- Stereo input: the Plate takes a mono sum (true to the classic plate design). Hall and Room feed L and R into different tank inputs, so stereo placement is kept loosely.

### Modes [Must]
The builder chooses the exact topology. Targets and references:
| Mode | Character | Suggested basis |
|---|---|---|
| **Plate** | Bright, dense instantly, smooth, no distinct early reflections. Classic vocal and lead plate (EMT 140 flavour). | Dattorro plate (J. Dattorro, "Effect Design Part 1", JAES 1997), scaled for Size and sample rate. Implementing from the paper is licence-free. |
| **Hall** | Slower build, big and smooth, darker highs, long tails. For lead guitar and ballad vocals. | 8- or 16-line feedback delay network (Householder or Hadamard matrix) with per-line two-band decay filters, plus sparse early reflections. |
| **Room** | Short, punchy, strong early reflections. For drums, rhythm guitar glue, "in a room" vocals. | Tapped-delay early-reflection pattern (about 12 to 24 taps scaled by Size) feeding a smaller FDN. |

- **Decay** is defined as RT60 at 1 kHz, the same meaning in every mode. Size changes delay lengths (the perceived space), Decay changes how long it rings. They are independent.
- All internal delay times are in **milliseconds** and scale with sample rate, so Chamber sounds the same at 44.1, 48 and 96 kHz.
- **Mode change while playing:** crossfade the old tank's output out and the new one in over 100 ms. No click. The old tail may be cut short.

### Modulation [Must]
Slow, randomised modulation of a few tank delay lengths (interpolated reads) to break up metallic ringing. **Depth 0** = static. Higher depth gives a gentle chorus in the tail. Never audible as obvious vibrato at the default settings.

### Ducking [Must]
Turns the **wet** signal down while the dry input is loud, and lets it bloom back when you stop playing.
- Detector: peak-ish envelope of the dry input (stereo max), 5 ms attack (fixed), **Duck Release** adjustable.
- Gain reduction on the wet = Duck Amount × how far the input is over **Duck Threshold**, reaching full Amount 12 dB above threshold (soft knee, 6 dB).
- **Duck Amount 0 = off** (no detector CPU).

### Freeze [Must]
- On: input to the tank fades to silence over 50 ms; tank feedback becomes exactly lossless (infinite decay); Damping and Bass Decay are bypassed in the loop so the frozen sound doesn't drift duller. Modulation keeps running at its set depth (it gives a living pad).
- Off: normal decay resumes from wherever the tail is and input fades back in over 50 ms.
- Ducking, Width, Tail EQ and Mix still apply while frozen.

### Mix law
Equal-power dry/wet. **Mix 0%** = dry only, bit-identical to input. **Mix 100%** = wet only (use on Return tracks).

---

## 2. Parameters

Parameter IDs are permanent once Trent uses the plugin in real sets. Adding parameters later is fine.

| ID | Shown name | Range | Default | Display | Scaling | Type | Smoothing |
|---|---|---|---|---|---|---|---|
| `mode` | Mode | Plate, Hall, Room | Plate | – | – | enum | 100 ms crossfade |
| `mix` | Mix | 0 to 100 | 30 | `30 %` | linear | continuous | 20 ms |
| `predelay` | Pre-delay | 0 to 250 | 10 | `10.0 ms` | skew (fine at low values) | continuous | 50 ms (interpolated, no clicks) |
| `predelay_sync` | Pre-delay Sync | off/on | off | – | – | bool | – |
| `predelay_div` | Pre-delay Note | 1/64, 1/32T, 1/32, 1/16T, 1/16, 1/16D, 1/8T, 1/8, 1/8D, 1/4 | 1/32 | `1/16 D` | – | enum | 50 ms |
| `size` | Size | 0 to 100 | 50 | `50 %` | linear | continuous | 200 ms (slight pitch glide OK, no clicks) |
| `decay` | Decay | 0.1 to 20 | 2.0 | `2.40 s` | log | continuous | 50 ms |
| `diffusion` | Diffusion | 0 to 100 | 75 | `75 %` | linear | continuous | 50 ms |
| `early` | Early | −24 to +6 | 0 | `0.0 dB` | linear | continuous | 20 ms (Hall/Room only; greyed for Plate) |
| `low_cut` | Low Cut | 20 to 1000 | 120 | `120 Hz` | log | continuous | 20 ms |
| `high_cut` | High Cut | 1000 to 20000 | 12000 | `12.0 kHz` | log | continuous | 20 ms |
| `damping` | Damping | 1000 to 20000 | 7000 | `7.00 kHz` | log | continuous | 50 ms (frequency above which the tail decays faster) |
| `bass_decay` | Bass Decay | 0.25 to 2.0 | 1.0 | `1.00×` | log | continuous | 50 ms (low-band RT60 multiplier, below ~250 Hz) |
| `mod_rate` | Mod Rate | 0.05 to 5 | 0.6 | `0.60 Hz` | log | continuous | 50 ms |
| `mod_depth` | Mod Depth | 0 to 100 | 25 | `25 %` | linear | continuous | 50 ms |
| `width` | Width | 0 to 150 | 100 | `100 %` | linear | continuous | 20 ms (0 = mono wet, 150 = extra wide via M/S) |
| `duck_amt` | Duck | 0 to 30 | 0 | `0.0 dB` (0 shows "Off") | linear | continuous | 20 ms |
| `duck_thresh` | Duck Threshold | −60 to 0 | −30 | `-30.0 dB` | linear | continuous | 20 ms |
| `duck_release` | Duck Release | 50 to 2000 | 400 | `400 ms` | log | continuous | none |
| `freeze` | Freeze | off/on | off | – | – | bool | 50 ms fades |
| `out_gain` | Output | −24 to +12 | 0 | `0.0 dB` | linear | continuous | 20 ms |

- All are host-automatable (Freeze automation is a common use, so it must be click-free).
- Pre-delay Sync uses host tempo. On a tempo change, the delay retargets smoothly (no click). With the transport stopped, it uses the last known tempo.
- **GUI-only saved state:** window size preset.

### Factory presets [Should]
| Name | Settings (others default) |
|---|---|
| Init | defaults |
| Vocal Plate | Plate, Mix 25, Pre-delay 1/32 synced, Decay 1.8 s, Low Cut 200, High Cut 9 k, Damping 6 k, Mod Depth 20, Duck 6 dB, Thresh −30, Release 350 |
| Lead Hall (ducked) | Hall, Mix 35, Pre-delay 40 ms, Size 75, Decay 3.5 s, Low Cut 250, High Cut 8 k, Damping 5 k, Bass Decay 0.8, Mod Depth 35, Duck 12 dB, Thresh −36, Release 600 |
| Drum Room | Room, Mix 20, Pre-delay 0, Size 30, Decay 0.7 s, Early +2, Low Cut 80, High Cut 14 k, Damping 9 k, Mod Depth 10, Width 110 |
| Clean Guitar Wash | Hall, Mix 45, Pre-delay 20 ms, Size 90, Decay 6 s, Low Cut 300, High Cut 7 k, Damping 4.5 k, Mod Rate 0.3, Mod Depth 60, Width 130 |
| Return 100% | Plate, Mix 100, everything else Init |

---

## 3. Look

### Window [Must]
Three fixed sizes from S / M / L buttons. Contents scale proportionally.
- **S** 640 × 400 **M** 820 × 512 (default) **L** 1100 × 690, at 100% Windows scaling.

### Layout
```
┌────────────────────────────────────────────────────────────────────┐
│ CHAMBER  [Preset ▾]      [ PLATE | HALL | ROOM ]          S  M  L  │ top bar
├──────────────────────────────────────────────────────┬─────────────┤
│                                                      │ IN  WET  GR │
│              ANIMATED SPACE (≈ 65% of width)         │ ▮   ▮    ▮  │ meters
│                                                      │             │
│                                                      │  [ FREEZE ] │ big toggle
├──────────────────────────────────────────────────────┴─────────────┤
│ SPACE   Pre-delay(♪) Size  Decay  Diffusion  Early │ MIX  OUTPUT     │ knob row 1
│ TONE    Low Cut  High Cut  Damping  Bass Decay     │                 │
│ MOTION  Mod Rate  Mod Depth  Width  │ DUCK  Amount  Thresh  Release  │ knob row 2
└────────────────────────────────────────────────────────────────────┘
```
- Mix and Output are larger knobs on the right. Pre-delay has a small ♪ toggle for Sync, and the knob then steps through note values.
- Section labels (SPACE, TONE, MOTION, DUCK) in small caps, muted text.

### Animated space [Must: basic version · Should: polish]
A 2D vector drawing in **pseudo-3D wireframe** (no 3D engine), animated at 60 fps, which stops when the editor is closed. It should look alive but stay cheap: under 1 ms of GUI time per frame at M size.
- **Shape follows Mode:**
  - Plate: a suspended rectangular sheet seen at an angle, with 4 corner springs.
  - Hall: a long room in perspective with a high arched ceiling line.
  - Room: a small box in perspective.
- **Size** scales the shape (smoothly animated, about 200 ms ease).
- **Reflections:** each time the input's level rises (a transient), spawn an expanding ring or wavefront from a "source" point. It travels outward, hits the walls and fades. Ring travel speed is tied to Size. Fade time is tied to Decay, so long decays visibly linger. At most ~24 rings alive at once.
- **Brightness / glow** of the walls follows the live wet level (RMS, ~100 ms smoothing).
- **Tone tint:** high Damping or a low High Cut shifts the glow warmer and dimmer; bright settings make it crisper. Keep it subtle.
- **Modulation:** walls shimmer/wobble very slightly at Mod Rate, scaled by Mod Depth.
- **Ducking:** the whole space dims by the current duck gain reduction.
- **Freeze:** rings stop spawning, existing ones hold in place and slowly pulse, and the space takes on a cool blue-white tint (`#bfe7ff`). A "FROZEN" label appears in the corner.
- **Width:** the side walls spread apart slightly as Width increases.
- Colours: wireframe lines in muted grey `#3a3a40`, live elements in the accent `#d4ff3a` at varying opacity, freeze tint as above.

### Meters [Must]
Vertical bars: **IN** (peak, dBFS), **WET** (peak, after duck), **GR** (duck reduction, 0 to −30 dB, top-down). Range −60 to 0 dB. Peak hold 1 s, then fall at ~20 dB/s.

### Controls and colours
Same style as Contour: arc knobs on dark tracks, values in IBM Plex Mono under each knob, click to type. Background `#0a0a0b`, panels `#141416`, grid/wireframe `#222226`/`#3a3a40`, text `#e8e8ea`, muted `#8a8a90`, accent `#d4ff3a`. Mode selector: segmented buttons, active segment filled with accent and black text. Freeze button: large, outlined; when on, filled with the freeze tint.

---

## 4. Edge cases
- **Silence in:** the tail decays naturally. Once the wet output is below −140 dBFS for 0.5 s (not frozen), output exact zeros and skip tank processing (CPU drops). The tank wakes instantly on new input.
- **Never self-oscillates** at any combination of settings, except Freeze, which holds level and never grows.
- **Gain staging:** at defaults, a 0 dBFS pink-noise burst produces wet peaks no louder than about +3 dB over the input. No internal clipping. Float passes through.
- **Sample-rate change / host reset:** clear tank, pre-delay, detectors and visual rings. No pops.
- **Transport stop/start, loop:** no reset. The tail continues (like hardware).
- **Size change while playing:** delay lengths glide (a slight pitch smear is fine) with no clicks or zipper.
- **Freeze on with no tail:** freezes silence. That's fine.
- **Freeze on for a long time:** level stays stable, with no slow growth or DC buildup (DC-block in the loop).
- **Denormals:** no CPU spikes on long decays into silence.

## 5. Acceptance checks (done when…)
1. Mix 0% nulls against the input to −140 dBFS or better. Reported latency is 0.
2. Measured RT60 at 1 kHz (impulse response, octave band) is within ±10% of the Decay setting for each mode, at Decay 0.5 s, 2 s and 8 s, at 44.1, 48 and 96 kHz.
3. Bass Decay 2.0 → RT60 at 125 Hz is 1.8–2.2× the 1 kHz value. Damping 2 kHz → RT60 at 8 kHz is clearly shorter (≤0.5×) than at 1 kHz.
4. Impulse-response energy envelope decreases monotonically (in 50 ms windows) for every mode at Decay 20 s, Size 100, Mod Depth 100 (stability).
5. Freeze: after a noise burst, wet RMS stays within ±1 dB over 60 s and never exceeds the pre-freeze level.
6. Width 0% → wet L and R identical. Width 100% → wet L/R correlation between −0.2 and +0.4 on a mono impulse.
7. Ducking: steady −12 dBFS sine in, Duck 12 dB, Thresh −24 → wet reduced by 12 ± 1 dB. After the input stops, it recovers to within 1 dB of no-duck level within 5× Release.
8. Pre-delay Sync at 120 BPM, 1/16 → 125 ms ± 1 sample.
9. Automating Freeze, Mode and Size, and sweeping Pre-delay, produces no discontinuity beyond the smooth fades (checked numerically: no sample-to-sample jump above a threshold set from the signal's normal slope).
10. No NaN/Inf on any test signal, including full-scale square waves and DC.
11. CPU: Hall (heaviest mode), stereo, 48 kHz, 256-sample buffer, Mod Depth 100 → well under 5% of one core. Silent input after the tail → near 0.
12. Passes pluginval at strictness 5. State and window size recall exactly after save/reload.
13. **[Should]** Echo density: normalised echo density (Abel & Huang) reaches ≥ 0.9 within 150 ms for Plate and Hall at Diffusion 75 (a numeric proxy for "no grainy flutter").
14. On the studio PC (Trent checks): loads in Live 12; the animation is smooth at each size; Freeze automation is click-free; a saved set reopens identical.

## 6. Listening rounds (how Trent reports back)
For each round, run these through Chamber and Ableton's stock Reverb at similar settings, and report with the format below:
- Clean guitar arpeggio, **Hall**, Lead Hall preset.
- A sung phrase, **Plate**, Vocal Plate preset.
- Drum loop, **Room**, Drum Room preset.
- A distorted lead line with Duck on vs off.

```
Mode / preset:
Source:
Problem (pick): metallic ringing · grainy/fluttery · too bright · too dark · boomy · too short/long · smeared attack · pumping duck · other
When I hear it: (e.g. on snare hits, on sustained notes, at the tail end)
Compared to stock Reverb: better / same / worse, and why
Live version, sample rate, buffer size:
```

## 7. Out of scope (v1)
Convolution/impulse responses · Shimmer or pitch-shifted modes · external sidechain for ducking · gating · tempo-synced decay · reverse reverb · freely resizable window · MIDI learn · a real 3D engine for the visual.

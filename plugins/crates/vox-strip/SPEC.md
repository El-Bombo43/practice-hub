# Vox Strip

**One line:** a one-plugin vocal chain (gate, high-pass, de-esser, compressor, saturation) for Trent's rock and metal vocals, from clean singing to screams.

**Category:** audio effect  **Channels:** mono in/out and stereo in/out layouts  **Sidechain:** no  **MIDI:** none

**Lives at:** `practice-hub/plugins/vox-strip/SPEC.md`.

**Shared conventions:** same as Contour and Chamber (see their SPEC.md files in `plugins/`):
- Live 12 Standard on Windows 64-bit; VST3; personal use (GPL is fine).
- Practice-hub palette with hybrid accents; Sora and IBM Plex Mono fonts.
- S/M/L fixed window sizes.
- Controls: double-click to reset, Shift-drag for fine adjustment, click a value to type it.
- [Must]/[Should] priorities.
- Reuse Contour's filter code and the shared GUI theme crate.

**Latency: 0 samples.** No lookahead anywhere, so Trent can sing through it while tracking.
**Recommended chain in Ableton:** Snap (pitch) → Vox Strip → Contour → sends to Chamber/Echo.

---

## 1. Sound and behaviour

### Signal flow
```
In ─► Input Gain ─► Gate ─► High-Pass ─► De-esser ─► Compressor ─► Saturation ─► Output Gain ─┬─► Mix ─► Out
 └──────────────────────────────── dry (parallel) ───────────────────────────────────────────┘
```
- Each module has its own **On** switch. Off means fully bypassed: no CPU, and a 10 ms crossfade when switching.
- Stereo: both channels share one detector per module (linked), so the image never shifts.
- **Mix** is whole-strip parallel processing ("New York" vocal compression). 100% = fully processed.

### Modules
**Gate [Must]**
- A downward expander that reduces bleed and breaths between phrases. **Range** sets how far it turns down, not necessarily to silence.
- Hold is fixed at 20 ms. Hysteresis is fixed at 4 dB, so it opens at Threshold and closes 4 dB below it, which stops chatter on decaying notes.
- No lookahead. It should open within 1 ms of the signal crossing the threshold.

**High-Pass [Must]**
- Removes rumble and mic-stand thumps.
- Slope 12 or 24 dB/oct, Butterworth.

**De-esser [Must]**
- Detector: band-pass around **Freq**, compared to Threshold.
- **Split-band** mode (default): reduces only the sibilant band (a dynamic high shelf at Freq).
- **Wideband** mode: reduces the whole signal.
- **Listen** (GUI-only, not saved): outputs only the detector band, so Trent can find the "s" frequency.

**Compressor [Must]**
- Feed-forward, with a soft knee.
- Styles:
  - **Clean**: peak-ish detector, exact attack and release. Precise and transparent.
  - **Opto**: RMS-ish detector with a program-dependent two-stage release. It recovers fast from short peaks and slowly after long loud passages, in the smooth LA-2A-style vocal manner. In Opto, the Attack and Release knobs scale the curve rather than setting exact times.
- **Auto Makeup [Should]**: adds makeup gain based on Threshold and Ratio (the static curve's gain at −18 dBFS input).

**Saturation [Must]**
- **Tape**: soft symmetric curve, a gentle high-frequency roll-off as Drive rises, mostly odd harmonics.
- **Tube**: asymmetric curve (even harmonics), warmer.
- Built-in level compensation, so turning up Drive adds grit but not much volume.
- 2× oversampling inside this module only (polyphase IIR, so it adds no latency) to avoid aliasing on screams.

---

## 2. Parameters
| ID | Shown name | Range | Default | Display | Scaling | Type |
|---|---|---|---|---|---|---|
| `in_gain` | Input | −24 to +24 | 0 | `0.0 dB` | linear | cont. |
| `gate_on` | Gate On | off/on | off | – | – | bool |
| `gate_thresh` | Gate Threshold | −80 to 0 | −50 | `-50.0 dB` | linear | cont. |
| `gate_range` | Gate Range | 0 to 80 | 20 | `20.0 dB` | linear | cont. |
| `gate_attack` | Gate Attack | 0.1 to 50 | 1 | `1.0 ms` | log | cont. |
| `gate_release` | Gate Release | 10 to 1000 | 120 | `120 ms` | log | cont. |
| `hpf_on` | HPF On | off/on | on | – | – | bool |
| `hpf_freq` | HPF Freq | 20 to 400 | 80 | `80 Hz` | log | cont. |
| `hpf_slope` | HPF Slope | 12, 24 | 12 | `12 dB/oct` | – | enum |
| `ds_on` | De-ess On | off/on | on | – | – | bool |
| `ds_freq` | De-ess Freq | 3000 to 12000 | 6500 | `6.50 kHz` | log | cont. |
| `ds_thresh` | De-ess Threshold | −60 to 0 | −30 | `-30.0 dB` | linear | cont. |
| `ds_range` | De-ess Range | 0 to 20 | 6 | `6.0 dB` | linear | cont. |
| `ds_mode` | De-ess Mode | Split-band, Wideband | Split-band | – | – | enum |
| `comp_on` | Comp On | off/on | on | – | – | bool |
| `comp_style` | Comp Style | Clean, Opto | Clean | – | – | enum |
| `comp_thresh` | Comp Threshold | −60 to 0 | −18 | `-18.0 dB` | linear | cont. |
| `comp_ratio` | Comp Ratio | 1 to 20 | 3 | `3.0:1` | log | cont. |
| `comp_attack` | Comp Attack | 0.1 to 100 | 10 | `10.0 ms` | log | cont. |
| `comp_release` | Comp Release | 10 to 1000 | 120 | `120 ms` | log | cont. |
| `comp_knee` | Comp Knee | 0 to 24 | 6 | `6.0 dB` | linear | cont. |
| `comp_makeup` | Comp Makeup | −12 to +24 | 0 | `0.0 dB` | linear | cont. |
| `comp_auto` | Auto Makeup | off/on | off | – | – | bool |
| `sat_on` | Sat On | off/on | off | – | – | bool |
| `sat_type` | Sat Type | Tape, Tube | Tape | – | – | enum |
| `sat_drive` | Drive | 0 to 100 | 20 | `20 %` | linear | cont. |
| `sat_mix` | Sat Mix | 0 to 100 | 100 | `100 %` | linear | cont. |
| `out_gain` | Output | −24 to +24 | 0 | `0.0 dB` | linear | cont. |
| `mix` | Mix | 0 to 100 | 100 | `100 %` | linear | cont. |

- All parameters are automatable.
- Continuous parameters use 20 ms smoothing. Switches and enums use a 10 ms crossfade.

### Factory presets [Should]
| Name | Key settings |
|---|---|
| Init | defaults |
| Rock Lead | HPF 100; De-ess 6.5 k −30/6; Comp Opto, −20, 4:1; Sat Tape 25 |
| Screams | Gate on −45/30; HPF 120; De-ess 5.5 k −28/8; Comp Clean, −24, 8:1, Atk 2, Rel 60; Sat Tube 45; Mix 85 |
| Clean Ballad | HPF 80; De-ess 7 k −32/5; Comp Opto, −16, 2.5:1; Sat off |
| Backing Vocals | HPF 150; De-ess 6.5 k −34/10; Comp Clean, −22, 6:1; Sat Tape 15 |

---

## 3. Look
**Window:** S 760 × 360, **M 960 × 450 (default)**, L 1280 × 600.

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ VOX STRIP  [Preset ▾]                                              S  M  L  │
├──────────────────────────────────────────────────────────────────────────────┤
│  SCROLLING LEVEL HISTORY: input waveform (muted) · output (accent) ·        │ [Should]
│  comp gain reduction drawn down from the top (band colour)                  │
├──────┬──────┬──────────┬──────────────┬──────────┬──────────────────────────┤
│ IN   │ GATE │  HPF     │  DE-ESS      │  COMP    │  SAT        │ OUT  MIX   │
│ knob │ ● on │  ● on    │  ● on  [🎧]  │  ● on    │  ● on       │ knob knob  │
│ meter│ LED  │  freq    │  freq thr rng│ Clean|Opto│ Tape|Tube │ meter      │
│      │ 4 kn │  slope   │  mode  GR▮   │ 6 knobs  │ drive mix   │            │
│      │      │          │              │  GR ▮    │             │            │
└──────┴──────┴──────────┴──────────────┴──────────┴──────────────────────────┘
```
- Modules are laid out as panels left to right, in signal order. Each has an on/off pill at the top. A module that is off dims to 40%.
- Each module has its own band colour (reusing Contour's palette order): Gate red, HPF orange, De-ess yellow, Comp green, Sat cyan.
- Meters:
  - Input and output peak/RMS bars, −60 to 0 dB.
  - A gate open/closed LED.
  - De-ess and compressor gain-reduction bars, 0 to −20 dB.
  - Ballistics: peak hold 1 s, fall 20 dB/s.
- **[Should] Level history:** scrolls at about 5 s across the width and redraws at 60 fps. It is the main way to see what the compressor is doing.
- **[Should] Compressor curve:** a small static transfer-curve graph (input vs output dB, showing knee and ratio) inside the Comp panel.

## 4. Edge cases
- All modules off, Mix 100, gains 0: bit-identical passthrough.
- Silence in: silence out. The gate closes and the detectors rest.
- No internal clipping. Saturation is the only module that intentionally distorts.
- Sample-rate change or host reset: clear all filter and detector states.
- Switching a module on or off while playing: 10 ms crossfade, no click.

## 5. Acceptance checks
1. All modules off: output nulls with input to −140 dBFS. Reported latency is 0.
2. HPF at 80 Hz, 12 and 24 dB/oct: −3 dB at 80 Hz ± 2%, and the slope is within ±0.5 dB/oct one octave below.
3. De-esser split-band: with a 6.5 kHz sine at −10 dBFS, Thresh −30 and Range 6, the 6.5 kHz level drops by 6 ± 0.5 dB, and a 500 Hz sine mixed in changes by < 0.2 dB.
4. Compressor Clean, static curve (slow ramp test): matches threshold, ratio and knee within ±0.5 dB from −60 to 0 dBFS. Attack and release time constants are within ±10% (step test, 63% point).
5. Opto: release after a 50 ms burst is clearly faster than after a 2 s burst (at least 2× faster to 63% recovery).
6. Gate: opens within 1 ms of a step crossing the threshold. With an exponentially decaying 220 Hz tone, it never toggles more than once (no chatter).
7. Saturation:
   - Drive 0 gives THD < −80 dB.
   - THD rises monotonically with Drive.
   - Output RMS on a −18 dBFS sine stays within ±1.5 dB from Drive 0 to 100.
   - Aliasing products from a 5 kHz sine at Drive 100 stay below −60 dB relative to the fundamental.
8. Mix 0: nulls with input.
9. No NaN/Inf. CPU with all modules on, stereo, 48 kHz is well under 3% of a core.
10. Passes pluginval at strictness 5, and state recalls exactly.
11. Studio check (Trent): sing a clean verse and a scream through each preset, and confirm no clicks when toggling modules.

## 6. Out of scope (v1)
Lookahead · EQ beyond the HPF (use Contour) · external sidechain · a limiter stage · pitch (that's Snap) · doubler · freely resizable window.

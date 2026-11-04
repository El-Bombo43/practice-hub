# Punch

**One line:** a level-independent transient shaper (more or less attack, more or less sustain) for drums, palm mutes and bass.

**Category:** audio effect  **Channels:** mono in/out and stereo in/out  **Sidechain:** no  **MIDI:** none

**Lives at:** `practice-hub/plugins/punch/SPEC.md`.

**Shared conventions:** same as Contour/Chamber/Vox Strip. Live 12 Standard, Windows 64-bit, VST3, personal use, practice-hub palette with hybrid accents, Sora/Plex Mono, S/M/L sizes, standard knob interactions, [Must]/[Should]. Reuse Vox Strip's envelope-follower code.

**Latency: 0 samples** (no lookahead).

---

## 1. Sound and behaviour
```
In ─┬─► Focus filter (detector only) ─► Fast env + Slow env ─► Attack gain & Sustain gain ─┐
    └────────────────────────────────────────────────────────────────► × gain ─► Clip guard ─► Output ─► Mix ─► Out
```
- **Level-independent**, in the style of the classic SPL Transient Designer: it reacts to the *shape* of the sound, not its loudness. There is no threshold knob. A quiet snare and a loud snare get the same treatment.
- Two envelope followers run on the detector signal:
  - A **fast** one with ~1 ms attack.
  - A **slow** one whose attack is set by **Attack Length**.
  - While fast > slow, the sound is in its transient, and **Attack** gain applies, scaled by the difference.
  - A second pair compares a held-peak envelope to a release envelope set by **Sustain Length**. While the signal is decaying, **Sustain** gain applies.
- **Attack** ±100% maps to about ±12 dB at the peak of a hard transient. **Sustain** ±100% maps to about ±18 dB on the tail.
- **Focus** high-passes the detector only, not the audio. Raising it makes Punch react to the click of a kick or a pick attack instead of low-end boom.
- **Stereo link** on (default): one shared detector, so the image stays stable.
- **Clip guard [Should]:** an optional soft clip at the output, so big Attack boosts don't spike past 0 dBFS.

---

## 2. Parameters
| ID | Shown name | Range | Default | Display | Scaling | Type |
|---|---|---|---|---|---|---|
| `attack` | Attack | −100 to +100 | 0 | `+40 %` | linear | cont. |
| `sustain` | Sustain | −100 to +100 | 0 | `-30 %` | linear | cont. |
| `attack_len` | Attack Length | 1 to 50 | 10 | `10.0 ms` | log | cont. |
| `sustain_len` | Sustain Length | 20 to 1000 | 150 | `150 ms` | log | cont. |
| `focus` | Focus | 20 to 2000 | 20 | `20 Hz` (20 shows "Off") | log | cont. |
| `link` | Stereo Link | off/on | on | – | – | bool |
| `clip` | Clip Guard | off/on | off | – | – | bool |
| `out_gain` | Output | −24 to +12 | 0 | `0.0 dB` | linear | cont. |
| `mix` | Mix | 0 to 100 | 100 | `100 %` | linear | cont. |

- All are automatable. Continuous parameters use 20 ms smoothing.

### Factory presets [Should]
| Name | Settings |
|---|---|
| Init | defaults |
| Snappy Kick | Attack +60, Sustain −20, Attack Len 8, Focus 80 |
| Tight Palm Mutes | Attack +25, Sustain −45, Sustain Len 120, Focus 150 |
| Room Killer (drums) | Attack +10, Sustain −70, Sustain Len 200 |
| Fat Snare | Attack +30, Sustain +40, Sustain Len 250 |
| Soften Pick | Attack −40, Sustain 0, Focus 300 |

---

## 3. Look
**Window:** S 520 × 320, **M 660 × 400 (default)**, L 880 × 540.

```
┌──────────────────────────────────────────────────────────────┐
│ PUNCH  [Preset ▾]                                    S  M  L │
├──────────────────────────────────────────────────────────────┤
│  SCROLLING WAVEFORM (~3 s): input outline (muted),           │
│  output filled; boosted regions tinted accent,               │
│  cut regions tinted red                                      │
├──────────────────────────────────────────────────────────────┤
│        ◯ ATTACK              ◯ SUSTAIN       (big, centre-zero arcs) │
│  Atk Len ◯  Sus Len ◯  Focus ◯  Link ▢  Clip ▢   Mix ◯  Out ◯ │
└──────────────────────────────────────────────────────────────┘
```
- **Waveform [Must]:** scrolls at 60 fps. The gain change is shown by colour: accent `#d4ff3a` where Punch is adding level, red `#ff6b6b` where it's cutting, and intensity proportional to dB. This is the main feedback for what the knobs are doing.
- **Attack and Sustain knobs** are large, with **centre-zero** arcs that fill left (cut, red) or right (boost, accent) from 12 o'clock.
- Small input/output peak meters sit beside the waveform, from −60 to 0 dB.

## 4. Edge cases
- Attack 0, Sustain 0, Mix 100, Output 0: bit-identical passthrough.
- Silence in: silence out, with no gain pumping on the noise floor. Below −80 dBFS the gain eases to 0 dB.
- A steady sine (no transients) settles to 0 dB gain change.
- Sample-rate change or reset: clear the envelopes.
- No clipping unless Clip Guard is on. Then the soft clip starts at −1 dBFS and never exceeds 0 dBFS.

## 5. Acceptance checks
1. Neutral settings: output nulls with input to −140 dBFS. Reported latency is 0.
2. **Level independence:** the same drum loop at −6 dBFS and −26 dBFS gets the same gain-change curve within ±0.5 dB (after normalising level).
3. Attack +100 on a 1 kHz tone burst with an instant onset: the first 5 ms rise by 12 ± 1.5 dB relative to the settled level. Attack −100 cuts by a similar amount.
4. Sustain −100 on a burst with a 300 ms exponential decay: the tail 100 ms after onset is reduced by ≥ 12 dB. Sustain +100 raises it by ≥ 12 dB.
5. A steady sine settles to within ±0.2 dB of unity gain within 2× Sustain Length.
6. Focus 500 Hz: a 60 Hz burst triggers less than half the attack gain of a 3 kHz burst at the same level.
7. Clip Guard on: with a 0 dBFS input and Attack +100, no output sample exceeds 0 dBFS.
8. No NaN/Inf. CPU is tiny (< 1% of a core, stereo, 48 kHz).
9. Passes pluginval at strictness 5, and state recalls exactly.
10. **Studio check (Trent):** a drum loop and a palm-muted riff through each preset. Waveform colours should match what he hears, with no clicks when sweeping knobs.

## 6. Out of scope (v1)
Lookahead · multiband shaping · per-band frequency splitting · threshold-based mode · sidechain.

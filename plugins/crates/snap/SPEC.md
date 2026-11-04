# Snap

**One line:** real-time vocal pitch correction, from natural cleanup to the hard-snapped robotic effect, with a key/scale picker, note toggles and a live pitch graph.

**Category:** audio effect  **Channels:** mono in/out and stereo in/out (detection on the mono sum, the same shift applied to both channels)  **Sidechain:** no  **MIDI:** none

**Lives at:** `practice-hub/plugins/snap/SPEC.md`. **Build this last.** It's the hardest DSP of the set. Snap is a working name and can be renamed freely before the first real use.

**Shared conventions:** same as Contour/Chamber/Vox Strip. Live 12 Standard, Windows 64-bit, VST3, personal use, practice-hub palette with hybrid accents, Sora/Plex Mono, S/M/L sizes, standard knob interactions, [Must]/[Should].

**Never call it "Auto-Tune" anywhere in the UI or code** (Antares trademark). Use "pitch correction".

---

## 1. Sound and behaviour

### Signal flow
```
In ─► (mono sum) ─► Pitch detector ─► Note decision (enabled notes, hysteresis) ─► Correction curve (Retune / Humanize / Flex / Amount)
 │                                                                                         │
 └──────────────► Delay line (latency) ──────────────► Pitch shifter ◄──── shift ratio ────┘ ─► Mix ─► Output Gain ─► Out
```

### Latency modes [Must]
| Mode | Reported latency | Use |
|---|---|---|
| **Track** | ~10 ms | Singing through it while recording. Slightly less clean on low voices. |
| **Mix** (default) | ~40 ms | Best quality. Ableton's delay compensation hides it in mixing. |

- Latency is reported to the host. Changing the mode changes the reported latency, and a brief dropout when switching mid-playback is acceptable.
- The mode is **not automatable**.

### Detection [Must]
- A monophonic pitch detector, YIN or pYIN style, with a voiced/unvoiced decision. The builder chooses the method.
- **Voice** sets the search range, for accuracy and to avoid octave errors:
  - High: 150–1000 Hz
  - Mid: 80–800 Hz (default)
  - Low: 55–450 Hz
- **Unvoiced sounds** (s, t, k, breaths, noise) pass through unshifted. Crossfade between shifted and unshifted over about 5 ms at voicing changes.

### Note decision [Must]
- Target = the nearest **enabled** note to the detected pitch, in any octave.
- **Decision hysteresis** of 15 cents: once on a target, the detected pitch must move 15 cents past the halfway point before switching. Sliding notes therefore don't flicker between two targets.
- If no notes are enabled, behave as Amount 0 (pass through).

### Correction [Must]
- **Retune** is the time to cover 90% of the distance to the target after a new note starts. 0 ms is an instant snap, which gives the hard robotic effect.
- **Humanize [Should]** slows correction on sustained notes. After a note has held for more than 150 ms, the effective Retune lengthens by up to 4× at Humanize 100, so long notes keep some natural drift while note starts still snap.
- **Flex** (vibrato keep) keeps the singer's own vibrato around the target. The correction follows the slow (sub-3 Hz) pitch centre, and the faster wobble passes through, scaled by Flex: 0 flattens vibrato, 100 keeps it fully.
- **Amount** scales the final correction in cents. 0% is no correction.

### Pitch shifter [Must]
- Time-domain PSOLA (pitch-synchronous overlap-add) driven by the detector's pitch marks. It keeps the voice's natural formants for the small shifts used here (corrections up to ±6 semitones).
- Mix mode uses longer analysis windows and lookahead for cleaner marks. Track mode uses shorter ones.
- No noticeable artefacts at Amount 0 or with zero shift: the output equals the input delayed by the latency.

### Mix
Wet/dry. Note in the GUI tooltip that partial mix of shifted and unshifted signal sounds chorus-like. That's expected.

---

## 2. Parameters
| ID | Shown name | Range | Default | Display | Scaling | Type | Automatable |
|---|---|---|---|---|---|---|---|
| `mode` | Mode | Track, Mix | Mix | – | – | enum | **no** |
| `voice` | Voice | High, Mid, Low | Mid | – | – | enum | yes |
| `key` | Key | C, C#, D, … B | C | `E` | – | enum | yes |
| `scale` | Scale | Chromatic, Major, Natural Minor, Harmonic Minor, Melodic Minor, Major Pentatonic, Minor Pentatonic, Blues, Dorian, Phrygian, Lydian, Mixolydian, Custom | Major | – | – | enum | yes |
| `n_c` … `n_b` (12 IDs: `n_c, n_cs, n_d, n_ds, n_e, n_f, n_fs, n_g, n_gs, n_a, n_as, n_b`) | C … B | off/on | per C Major | – | – | bool | yes |
| `retune` | Retune | 0 to 400 | 50 | `50 ms` (0 shows "Snap") | skew (fine near 0) | cont. | yes |
| `humanize` | Humanize | 0 to 100 | 20 | `20 %` | linear | cont. | yes |
| `flex` | Flex | 0 to 100 | 50 | `50 %` | linear | cont. | yes |
| `amount` | Amount | 0 to 100 | 100 | `100 %` | linear | cont. | yes |
| `mix` | Mix | 0 to 100 | 100 | `100 %` | linear | cont. | yes |
| `out_gain` | Output | −24 to +12 | 0 | `0.0 dB` | linear | cont. | yes |

**Key, Scale and note-toggle rule:** the 12 note toggles are the truth.
- Choosing a Key or Scale writes the 12 toggles to match.
- Clicking any toggle sets Scale to **Custom** and leaves Key unchanged.
- Chromatic enables all 12.

### Factory presets [Should]
| Name | Settings |
|---|---|
| Natural – E minor | Key E, Natural Minor, Retune 80, Humanize 40, Flex 70 |
| Natural – Chromatic cleanup | Chromatic, Retune 120, Humanize 50, Flex 80, Amount 70 |
| Hard Snap – E minor | Key E, Natural Minor, Retune 0, Humanize 0, Flex 0 |
| Track Monitor | Mode Track, Chromatic, Retune 60 |

---

## 3. Look
**Window:** S 720 × 420, **M 920 × 540 (default)**, L 1240 × 720.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ SNAP  [Preset ▾]   Key [E ▾]  Scale [Natural Minor ▾]   [TRACK|MIX] S M L│
├──────┬───────────────────────────────────────────────────────┬──────────┤
│ B  ▢ │                                                       │   E3     │
│ A# ▢ │   SCROLLING PITCH GRAPH (≈ 6 s across)                │  +12 ¢   │ big note readout
│ A  ■ │   • note lanes: enabled notes lit, disabled dim       │ ◄──┼──►  │ cents meter
│ G# ▢ │   • input pitch: thin muted line                      │          │
│ G  ■ │   • corrected pitch: accent line                      │  in  out │
│ …    │   • unvoiced gaps: no line                            │  ▮   ▮   │ level meters
├──────┴───────────────────────────────────────────────────────┴──────────┤
│  Retune ◯     Humanize ◯     Flex ◯     Amount ◯     Voice [Mid▾]    Mix ◯  Output ◯ │
└──────────────────────────────────────────────────────────────────────────┘
```
- **Note strip** (left): 12 clickable toggles, highest note at the top, aligned with the graph's note lanes. Lit means enabled. The note currently targeted pulses in the accent colour.
- **Pitch graph [Must]:**
  - Vertical axis in semitones, auto-centred on recent pitch with about a 1.5-octave view. Mouse wheel zooms, drag pans.
  - Input pitch is drawn as a thin line in `#8a8a90`. Corrected pitch is drawn in accent `#d4ff3a`. Where they differ, the area between them is filled faintly.
  - Updates at 60 fps.
- **Note readout + cents meter [Must]:** shows the detected note name with octave and its cents offset from the nearest note (−50 to +50, centre-zero needle, green within ±10 ¢).
  - It shows the **input** (uncorrected) pitch, so Snap also works as a **singing-accuracy trainer**: Amount 0 plus this readout tells Trent how close his raw notes land.
- Retune is the biggest knob. Its readout shows "Snap" at 0.

## 4. Edge cases
- **Silence or noise in:** output equals input (delayed by the latency). No warbling on noise.
- **Polyphonic input** (chords, a guitar strum): not supported. Fall back gracefully: unvoiced or low-confidence frames pass through unshifted, with no loud artefacts.
- **Very low notes** below the Voice range: pass through unshifted.
- **Sample-rate change or reset:** clear buffers and pitch history.
- **Key/Scale automated mid-note:** the new target takes effect via the Retune glide, with no click.
- **Bypass:** the host bypass must stay latency-compensated. nih-plug and Ableton handle this, but verify it in the studio check.

## 5. Acceptance checks
Test signals: sines and a **synthetic vowel** (an impulse train through 3 formant filters at about 700/1200/2600 Hz) from 80 to 800 Hz, with and without vibrato.

1. **Detection:** in Mix mode, pitch is within ±5 cents of truth within 25 ms of onset for 80–800 Hz. Octave errors occur in < 1% of voiced frames on the test set. White noise is flagged unvoiced in > 95% of frames.
2. **Hard snap:** a vowel at 445 Hz (A4 +20 ¢) with A enabled and Retune 0 → output within ±3 ¢ of 440 Hz within 30 ms of onset.
3. **Retune timing:** for Retune 100 ms, the time to cover 90% of a 40 ¢ correction is 100 ms ± 20%.
4. **Scale snapping:** in C Major, a vowel at G#4 (−5 ¢) snaps to the nearer of G4/A4, and a glide from G4 to A4 switches target only once (hysteresis).
5. **Flex:** a vowel on A4 with 5.5 Hz ±40 ¢ vibrato keeps ≥ 80% of its depth at Flex 100 and ≤ 20% at Flex 0, centred within ±5 ¢ of 440 Hz.
6. **Transparency:** with Amount 0, the output nulls against the input delayed by the reported latency (−100 dBFS or better) in both modes.
7. **Latency:** the reported latency matches cross-correlation measurement within ±1 sample, in both modes.
8. **Quality:** a corrected sustained vowel shows no discontinuities. The spectral distance from an ideally re-synthesised vowel is low; the builder sets a numeric threshold from the first working version and keeps it as a regression test.
9. No NaN/Inf on any signal, including full-scale noise and DC.
10. CPU in Mix mode, stereo, 48 kHz is well under 5% of a core.
11. Passes pluginval at strictness 5. State, including the 12 toggles, recalls exactly.
12. **Studio check (Trent):** sing a slow slide, a scale run in E minor and a held note with vibrato, at Natural and Hard Snap presets, in both modes. Report using the listening format below.

```
Preset / mode:
What I sang:
Problem (pick): warble · robotic when it shouldn't be · wrong note · flickers between notes · lisp-like "s" · latency too much to sing through · other
When: (note starts, held notes, low notes, slides)
Live version, sample rate, buffer size:
```

## 6. Out of scope (v1)
MIDI target notes · auto key detection · formant shifting and gender/throat controls · harmonies · Melodyne-style note editing · polyphonic correction · transposition beyond correcting to the nearest note.

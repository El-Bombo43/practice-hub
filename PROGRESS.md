# Progress

## Session 1 — done
Scaffold + Pages deploy, metronome (count-in bar, gap click), 13 exercises, scoring + keyboard simulator, practice screen, routine editing, XP/level/streak/combo/PBs, IndexedDB persistence, Settings (accent, volume, window, Export/Import JSON).
## Session 2 — done (34 tests; build passes; hardware untested)
`input/midi.ts` (device picker, remembered device, learn mode for L/R pads + two footswitch CCs, press-only ≥64), MIDI ms→audio-clock conversion re-synced each bar, lanes enforced for pad exercises (wrong hand = out of window, shown as ✕ ring), per-source latency calibration (16 clicks @90 BPM, median, 150 ms outlier cut), drum timeline (sticking row, R top/L bottom, velocity bars), L/R balance + drum tips, "lead with weaker hand" toggle.
## Decisions
- All inputs publish to `input/bus.ts`; keyboard keeps working (F = L, J = R, velocity 0.8). Default pad notes L=36, R=37 until learned. Guitar lanes stay informational.
- Pad notes only from the chosen device; footswitch CCs from any device. Learning a pad also selects its device.
- Mirroring applies from the bar after the next; needs ≥8 hits per hand before balance/tips show.
- Calibration stored in `settings.calibrationMs[source]` and subtracted before scoring.
## Known issues / next
- Not verified with real MPK/Chocolate Plus or in a browser. Session 3: audio onset input. TODO: burst logic, history charts.

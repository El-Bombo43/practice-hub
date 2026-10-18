# Progress

## Sessions 1–3 — done
Scaffold + Pages deploy, metronome (count-in, gap click, playhead sweep, count switch), 13 exercises, scoring, routine, XP/level/streak/combo/PBs, IndexedDB + JSON backup; MIDI pads (learn mode, footswitch, enforced lanes, drum screen, balance tips); guitar audio onset input (worklet + pure detector), stroke evenness, burst and endurance modes; per-source latency calibration.
## Session 4 — done (78 tests; build passes; nothing hardware- or browser-tested)
- Strum screen per `design/strum.html` (arrows, timing dot, 1 e & a count, accent %). Accent % = accent peak / mean non-accent peak − 1; lands at ≥ +15%; stat "Accents landed x/y" (cumulative per exercise); tip names the weakest beat after 2 bars. Keyboard: hold Shift for an accent (velocity 1.0 vs 0.8).
- Pattern builder (8ths/16ths; rest → strike → accent → ghost; parity directions), saved in IndexedDB + backup, appears in the library and routine; invalid saved patterns are dropped on load.
- History tab: clean-max line (SVG, running max per day), 8-week streak calendar, this week's minutes per instrument, recent sessions. Polish: level-up/PB/tempo toasts (3 s), +XP flash on clean reps, empty states, end-of-routine summary.
## Decisions
- Audio velocity = frame-RMS peak × 2 (clips at 0.5 RMS); accents depend on clean DI levels.
## Known issues / next
- Untested with real hardware or in a browser; verify strum accents with the Scarlett and tune sensitivity. Plan complete: next ideas are real-device tuning, burst/endurance history, a pads accent/ghost mode.

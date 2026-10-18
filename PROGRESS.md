# Progress

## Sessions 1–4 — done (99 tests; build passes; nothing hardware- or browser-tested)
Scaffold + Pages deploy, metronome (count-in, gap click, playhead sweep, count switch), 13 exercises, scoring, routine, XP/level/streak/combo/PBs, IndexedDB + JSON backup. MIDI pads (learn mode, footswitch, enforced lanes, drum screen, balance tips). Guitar audio input (worklet + pure detector), stroke evenness, burst/endurance. Strum screen with accent scoring (Shift = accent on keyboard), pattern builder, History tab, toasts/+XP flash/end-of-routine summary.
## Timing fixes after real-device testing
- Clock: `audio/clock.ts` maps `event.timeStamp` (keyboard/MIDI) to the audio clock with the median of many `getOutputTimestamp()` readings; audio onsets subtract the median output lag. Replaces the plan's per-bar MIDI re-sync. Lookahead 150 ms; late clicks are counted and shown as "Click glitch".
- Stray hits (beyond ±max(2×window, 60 ms)) show as dashed rings with a note; calibration reports spread and has ±1/±5 ms nudges; a "Re-center" bar shifts an input's calibration from the last ≥12 real hits.
- Detector (`onsetCore.ts`): three bands (low <300 Hz with a long frame, mid 300–1200, high >1200), each judged against its own 1 s history and its recent average (not the previous frame); refractory = half a slot (35–80 ms); cap 24 dB. Tests use plucked-string signals (82–330 Hz, hum, sustain, palm mutes, dull low E). A first low-E note right after loud high-E notes can still be missed.
## Decisions
- Audio velocity = frame-RMS peak × 2 (clips at 0.5 RMS). Stroke direction = the matched slot's lane.
## Known issues / next
- Tune on the real Scarlett/pads: use the pick counter in Guitar input. Ideas: burst/endurance history, pads accent mode.

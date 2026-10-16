# Progress

## Sessions 1–2 — done
Scaffold + Pages deploy, metronome, 13 exercises, scoring, routine, XP/level/streak/combo/PBs, IndexedDB + JSON backup; MIDI pads (learn mode, footswitch CCs, enforced lanes, drum screen, balance tips), per-source latency calibration. Follow-ups: sweeping playhead on the timeline, count switch (8ths/triplets/16ths/sextuplets; non-native counts keep their own PB via id `name@n`).
## Session 3 — done (63 tests; build passes; audio hardware untested)
- Guitar audio: `input/audioOnset.ts` (getUserMedia, no echo/noise/AGC, remembered device, "Close Ableton first" note) → `onsetWorklet.ts` (bundled with `?worker&url`) → `onsetCore.ts` (pure; HP 80 Hz, 256/128 frames, median(1 s)+k, 35 ms refractory, sample-accurate onset, 30 ms peak). Tests: noise bursts at 90–200 BPM 16ths, 44.1/48 kHz, ± background noise: ≤5 ms, no doubles.
- Setup panel: device, level meter (flashes on picks), sensitivity (k = 12 − 9·s dB), loudest-channel lock + "Detect channel", calibration for audio.
- Up/down evenness: stat + tips (offset gap >8 ms, then loudness gap >10%). Burst (3 bars at clean max + 8, 1 rest) and Endurance (30/60 s, report with accuracy, drift ms/10 s). Both are frozen-tempo: no ramp, PB or last-BPM change.
## Decisions
- Threshold = min(median + k, loudest frame − 15 dB), floor −50 dBFS, ≥6 dB frame rise: plain median+k misses dense 16ths. Stroke = the matched slot's lane (same as strict alternation, but a skipped note can't flip later strokes).
## Known issues / next
- Untested with a real Scarlett/guitar; tune sensitivity and calibrate first. Session 4 next. TODO: history charts.

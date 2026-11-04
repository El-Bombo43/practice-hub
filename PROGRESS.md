# Progress

## Sessions 1–5 + timing fixes — done
Trainer (metronome, scoring, routine, XP/PBs, History), MIDI pads, guitar audio input (three-band onset detector, clock smoothing, re-center), strum screen + accents, pattern builder, 8-week picking program mode (Program tab, logging, import/export, test history). See git history.
## Extras round (after Session 5) — done (157+ tests; build passes; nothing browser- or hardware-tested)
- Program: session position survives a reload (localStorage, same date, <12 h); retest/test blocks step on the clock: 60% clean tempo with click, 40% motion ceiling without.
- Guitar: Capture panel (10 s recording → waveform + detected picks, "I played N picks" → auto-sets sensitivity, WAV download). Detector median/max now refreshed every 4th hop (speed).
- Practice: "Slots to watch" (≥4 attempts, ≥40% late/early/missed; dashed columns; "drop 10%" button). Burst/endurance runs saved and shown in History (tables + trend lines). Pads: accent + ghost-note (`Slot.soft`, ≤60% of normal) scoring with 3 new exercises; L/R balance tip is skipped when dynamics are on purpose.
- Sound: click / wood / beep / rock beat / half-time beat (synthesised), tap tempo. Weekly summary in History (week picker, copy as text). PWA: manifest, icons, service worker (network-first pages, stale-while-revalidate assets), install button in Settings.
- Fixes/extras: leaving the Program tab pauses its click; chunk-switch flash no longer remounts the card. Badges (22, in History; checked as profile/program log change; toast on unlock). Keyboard cheat sheet in Settings. Probed a "low-string boost" for the detector: no gain (the one remaining miss is a low E over loud ringing E4, a band-overlap limit), so not built.
## Decisions
- Program exercises are `prog-*` ids. Auto-tune searches 11 sensitivity steps (each run ≈0.3 s per 10 s of audio).
## Known issues / next
- Verify on real hardware; service worker is untested in a browser. Ideas left: a phone layout, per-string detection profiles.

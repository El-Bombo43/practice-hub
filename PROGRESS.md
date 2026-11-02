# Progress

## Sessions 1–4 + timing fixes — done
Trainer (metronome, scoring, routine, XP/PBs, History), MIDI pads, guitar audio input (three-band onset detector, clock smoothing, re-center), strum screen + accents, pattern builder. See git history for detail.
## Session 5 (8-week picking program) — done (data file added after approval; build passes)
- Data: `src/data/programs/picking-8wk.json` + `plan/session-5.md` copied from `handoff/picking-program` once approved; `programs/programs.test.ts` validates the file (53 sessions, 23 exercises).
- Built: `data/program.ts` (types, `programDate`/shifts/"Repeat this week", `dayFor`, mapping to `Exercise`: strokes → lanes, bar widened to whole cycles, sweeps/legato `unscored`), `programLog.ts` (keys as the artifact, merge by date + newest `updatedAt` per key, import/export `practice-log/v1`, last-tempo, hints, CT/MC rows), Program tab (one session timer, 2-tone chime per block, 1 tone + flash per chunk, no click in lab/warmup/break/music, tabs in mono boxes, entry forms, CT auto-fill from clean max), History CT lines + test table, import/export dialog, "probe" label, manual ramp for program exercises (suggest, never apply).
## Decisions
- Program exercises get ids `prog-A2` (own PB/last BPM, hidden from the library); Space/footswitch drive the session on the Program tab; unscored hits are ignored.
## Known issues / next
- Nothing here is browser- or hardware-tested. Next: real-device tuning, pads accent mode.

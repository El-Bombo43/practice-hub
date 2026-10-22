# Progress

## Sessions 1–4 + timing fixes — done
Trainer (metronome, scoring, routine, XP/PBs, History), MIDI pads, guitar audio input (three-band onset detector, clock smoothing, re-center), strum screen + accents, pattern builder. See git history for detail.
## Session 5 (8-week picking program) — code done, DATA FILE NOT YET IN THE REPO (132 tests; build passes)
- Blocked: copying `plan/session-5.md` + `src/data/programs/picking-8wk.json` from branch `handoff/picking-program` was denied by the permission classifier (untrusted code integration). Build works without it (`import.meta.glob`; the Program tab stays hidden). To finish: `git checkout origin/handoff/picking-program -- src/data/programs/picking-8wk.json plan/session-5.md`; `programs/programs.test.ts` then validates it (checked once against a scratch copy: 53 sessions, 23 exercises OK).
- Built: `data/program.ts` (types, `programDate`/shifts/"Repeat this week", `dayFor`, mapping to `Exercise`: strokes → lanes, bar widened to whole cycles, sweeps/legato `unscored`), `programLog.ts` (keys as the artifact, merge by date + newest `updatedAt` per key, import/export `practice-log/v1`, last-tempo, hints, CT/MC rows), Program tab (one session timer, 2-tone chime per block, 1 tone + flash per chunk, no click in lab/warmup/break/music, tabs in mono boxes, entry forms, CT auto-fill from clean max), History CT lines + test table, import/export dialog, "probe" label, manual ramp for program exercises (suggest, never apply).
## Decisions
- Program exercises get ids `prog-A2` (own PB/last BPM, hidden from the library); Space/footswitch drive the session on the Program tab; unscored hits are ignored.
## Known issues / next
- Nothing here is browser- or hardware-tested. Next: real-device tuning, pads accent mode.

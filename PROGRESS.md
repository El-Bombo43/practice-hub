# Progress

## Session 1 — done
App scaffold + deploy workflow, metronome (lookahead, count-in bar, gap click), 13-exercise library, scoring + keyboard simulator, practice screen, 6-step routine (swap/add/remove/move), XP/level/streak/combo/PBs, IndexedDB persistence, Settings (accent, volume, window 10/15/25, Export/Import JSON). `npm run build` and `npm test` pass (17 tests).
## Decisions
- Lanes are informational: F/J never fail a hit on lane mismatch (session 1 only).
- Settings window (10/15/25) overrides each exercise's `windowMs`. First bar after Start is an unscored count-in.
- Slot times come from per-beat scheduled times, so tempo changes mid-bar still score correctly; auto ramp applies at the next bar line.
- +50 PB XP only when beating an existing PB (the first clean rep just sets the baseline). Streak: today counts once ≥10 min; a short today doesn't break it.
- Added `@types/react` and `@types/react-dom` (dev) — required by strict TS. Burst mode is a toggle stub only.
## Known issues
- Not verified in a browser (per CLAUDE.md). Push/deploy: GitHub App access to the repo was refused from the cloud session; Pages needs Settings → Pages → Source: GitHub Actions.
## Next
- Session 2 (MIDI). TODO: calibration UI (`settings.calibrationMs` exists), history charts, burst logic.

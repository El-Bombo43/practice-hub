# Session 1 — Core app, metronome, routine, keyboard scoring

**Done when:**
- The app is deployed on GitHub Pages.
- I can run today's routine with a working metronome.
- Hits from the keyboard simulator are scored and drawn on the hit timeline.
- Progress survives a page reload.
- `npm run build` and `npm test` pass.

## Tasks
1. **Scaffold.** Vite + React + TS, `tokens.css`, fonts via @fontsource, `.github/workflows/deploy.yml` (build → Pages), `base: '/practice-hub/'`. Keep the README to 10 lines or fewer: run, build, deploy.
2. **Metronome** (see CLAUDE.md).
   - Start/Pause (Space), −4 / +4, BPM 30–300.
   - Click volume and a gap-click toggle.
   - The AudioContext must start on the first user gesture.
3. **Exercise library** (`data/exercises.ts`), about 12 entries:
   - Guitar picking: single-string tremolo (16ths), chromatic 1-2-3-4 (16ths), string skipping (16ths), palm-muted 16ths, sextuplets (subdivision 6), gallop (pattern per beat: play, rest, play, play).
     - Picking lanes alternate down/up, starting with down.
   - Strum: basic 8ths (D, D U, U D U).
   - Strum: "16th strum, accents on 2 & 4". Beats 1–4 × 16ths, slots 1–16, odd = down, even = up.
     - Played: 1, 3, 4, 5 (accent), 6, 8, 9, 11, 12, 13 (accent), 14, 16.
     - Ghost: 2, 7, 10, 15.
   - Pads (lanes R/L): singles RLRL, doubles RRLL, paradiddle RLRR LRLL, paradiddle-diddle RLRRLL (subdivision 6), gallop on pads (R RL per beat, as 16ths: play, rest, play, play).
4. **Scoring** (`scoring/score.ts`) and Vitest tests covering:
   - offset sign (+ = late)
   - window edges
   - extras
   - ghost slots expect no hit
   - clean-rep logic
   - combo reset
   - ramp advance
5. **Keyboard simulator** feeds scoring:
   - F = L/down, J = R/up.
   - For exercises without lanes, either key counts.
   - Lanes are informational in this session (lane mismatch doesn't fail a hit). Note this in PROGRESS.md.
6. **Practice screen**, following `design/picking.html`:
   - Generic for every exercise kind; drum- and strum-specific visuals come later.
   - Hit timeline shows the last completed rep.
   - Stats: accuracy, combo (best), average offset, XP this session.
   - Tip card: one simple rule for now ("Mostly late: relax / mostly early: don't rush" when the mean offset is more than 8 ms).
7. **Routine:**
   - Default routine of about 21 min: tremolo 3 → paradiddle 4 → chromatic 5 → 16th strum 4 → gallop on pads 3 → burst (chromatic) 2.
   - Swap an exercise (same instrument), add, remove, move up/down.
   - When an exercise's time is up, show a "Next" button. Don't force-advance.
8. **Game and persistence:**
   - XP, level, streak, combo, personal bests, per-exercise last BPM.
   - Session log entries: date, exercise, minutes, best clean BPM, accuracy.
   - Settings: accent color, click volume, window ms (10/15/25), Export/Import JSON.

## Out of scope this session
MIDI, microphone, history charts, pattern builder, burst/endurance logic beyond a plain toggle stub.

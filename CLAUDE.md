# Practice Hub

Personal practice trainer for **guitar** (alternate picking, strum patterns) and **drum rudiments on MIDI pads** (Akai MPK Mini Plus). One user, Chrome on a Windows studio PC, big monitor viewed from ~1.5 m. Game-like feedback: streaks, XP, levels, combos, personal bests.

## Working rules — credits are limited, be efficient
- Do **only** the current session's file in `plan/`. Don't start later sessions' work; leave a TODO instead.
- Make technical decisions yourself. Never stop to ask. Record non-obvious decisions in `PROGRESS.md`.
- Read only the files you need. Don't re-read files you just wrote. Open `design/*.html` only when building that screen.
- Verify with `npm run build` and `npm test` (Vitest, pure logic only). **No** browser automation, screenshots, Playwright, or dev-server polling.
- Approved dependencies only (below). Adding one needs a strong reason, noted in `PROGRESS.md`.
- Hardware (MIDI pads, guitar audio) can't be tested in the cloud. Every input path must also work through the keyboard simulator so the scoring logic is testable without hardware.
- End of session: update `PROGRESS.md` (max 15 lines: done, decisions, known issues, next), commit, push to `main`.

## Stack (decided)
- Vite + React 18 + TypeScript (strict). Plain CSS with variables in `src/styles/tokens.css`. No UI kit, no Tailwind, no chart library (hand-drawn SVG).
- Approved deps: `react`, `react-dom`, `idb-keyval`, `@fontsource/sora`, `@fontsource/ibm-plex-mono`; dev: `vite`, `@vitejs/plugin-react`, `typescript`, `vitest`.
- Hosting: GitHub Pages via GitHub Actions (`.github/workflows/deploy.yml`), Vite `base: '/practice-hub/'`. HTTPS is required for Web MIDI and microphone access.
- Storage: IndexedDB via `idb-keyval`; everything stays in the browser. Settings has Export/Import JSON backup.

## Architecture
```
src/
  audio/clock.ts        AudioContext singleton; all timing uses ctx.currentTime (seconds)
  audio/metronome.ts    lookahead scheduler: setInterval 25 ms, schedule 100 ms ahead; oscillator clicks;
                        accent on beat 1, quieter subdivision ticks; gap-click (2 bars on, 1 bar muted); emits beat events
  input/types.ts        InputEvent { time: number /*audio-clock s*/, source: 'keyboard'|'midi'|'audio', lane?: Lane, velocity: number /*0-1*/ }
  input/keyboard.ts     simulator, always on: F = left / downstroke, J = right / upstroke, Space = start/pause
  input/midi.ts         (session 2)
  input/audioOnset.ts   (session 3) + worklet
  scoring/score.ts      pure functions, unit tested
  game/progress.ts      XP, levels, streak, combo, personal bests
  data/exercises.ts     exercise library as data
  data/store.ts         idb-keyval wrappers
  ui/                   screens and components
```

```ts
type Lane = 'L' | 'R' | 'down' | 'up';
type Slot = { play: boolean; lane?: Lane; accent?: boolean; ghost?: boolean }; // ghost = hand moves, no hit expected
type Exercise = {
  id: string; name: string; instrument: 'guitar' | 'pads';
  kind: 'picking' | 'strum' | 'rudiment';
  beatsPerBar: number; subdivision: 2 | 3 | 4 | 6;     // notes per beat
  pattern: Slot[];                                     // one bar; length = beatsPerBar * subdivision
  startBpm: number; ramp: { stepBpm: number; cleanRepsToAdvance: number };
  windowMs: number;                                    // default 15
  minutes: number;                                     // default length in a routine
};
```
- A rep = one bar.
- **Scoring:** match each hit to the nearest expected slot within ±max(2×window, 60 ms). Record `offsetMs` (+ = late) and `inWindow`. Hits matching nothing count as extras. A rep is **clean** when every played (non-ghost) slot is hit in window and there are no extras.
- **Latency:** a per-input calibration offset in ms is subtracted from every hit before scoring.

## Game rules
- XP: +1 per on-time hit, +10 per clean rep, +25 per earned tempo step, +50 for beating a personal best. Every level needs 1000 XP.
- Combo: consecutive on-time hits. A miss, an out-of-window hit, or an extra resets it.
- Streak: consecutive local days with at least 10 minutes of practice.
- Personal best ("clean max"): highest BPM with a clean rep, per exercise.
- Ramp: after `cleanRepsToAdvance` clean reps in a row, add `stepBpm`.
- Burst mode (guitar): 3 bars at clean max + 8 BPM, then 1 bar rest, repeating. Doesn't change the ramp or the personal best.

## Design — match `design/*.html` (static mockups with sample data)
- Colors: bg `#0a0a0b` · panel `#141416` · panel border `#232326` · inset `#1c1c1f` · grid line `#3a3a3f` · off segment `#2c2c30` · text `#f2f2f0` · muted `#a1a1a6` · faded `#7c7c82` · tip-card border `#3d2a14`.
- Accent is a user setting: `#d4ff3a` (default), `#3ee6d4`, `#ff4fd8`.
- Timing colors: on time = accent, **early `#5aa9ff`**, **late `#ff9f43`**. Use blue and orange for nothing else.
- Fonts: Sora for UI text; IBM Plex Mono for every number.
- Radii: panels 22 px, cards 16 px, pills 999 px. Buttons at least 44 px tall. Primary button is an accent pill with dark text.
- Layout (desktop, 1440 px wide, must still work at 1024 px; no phone layout needed):
  - **Top bar:** app name · level pill + XP bar · streak · "Step x/y · N min left".
  - **Left panel:** instrument/kind label · exercise name · tempo (160 px mono) · 4-segment beat bar · clean-rep pips + "N more → next BPM" · −4 / Pause / +4 · toggles.
  - **Right column:** hit-timeline panel, then a stats strip of 4 numbers plus one tip card.
  - **Bottom:** routine cards (done / now / upcoming) with "+ Add exercise".
- **Hit timeline:** one column per slot. Each column has a vertical grid line and a pill-shaped band showing ±window. The dot shifts horizontally 2 px per ms (1 px per ms when there are more than 8 columns). Use two rows when the exercise has lanes (down/up or R/L), otherwise one. The offset in ms goes under each column.

## Plan
`plan/session-1.md` → `plan/session-4.md`. Check `PROGRESS.md` for where things stand.

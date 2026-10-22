# Session 5: 8-week picking program (program mode)

Handoff from The Music Journey chat, Oct 22 2026. Trent started an 8-week picking program today. He runs it from the "Daily Picking Session" artifact page and a printed workbook. This session brings it into Practice Hub so the app can replace both. Follow `CLAUDE.md` working rules: make decisions yourself, unit-test pure logic only, run `npm run build` + `npm test`, update `PROGRESS.md`.

**Done when:**
- `src/data/programs/picking-8wk.json` (attached, `schema: "practice-program/v1"`) loads, and the app opens on **today's planned session** by local date.
- A session plays as a timed sequence of blocks and interleaved A/B chunks, and logs clean bpm and tension per exercise.
- Test blocks record clean tempo (CT) and motion ceiling (MC), and History charts them.
- A log pasted from the artifact's "Copy log as JSON" (`schema: "practice-log/v1"`) imports without loss.
- `npm run build` and `npm test` pass.

## The data (`picking-8wk.json`)
- `program`: `id`, `title`, `techniques` (key → label), `week1Monday` (2026-10-19), `kickoff` (2026-10-22), `end`, `phases[] {n, name, weeks, goal}`, `rules[]`, `safety[]`.
- `exercises`: 23 entries, `A1–A6, E1–E3, S0–S5, K1–K3, C0–C3, TL`.
  - Fields: `name, technique (alternate|economy|sweep|skipping|combo), purpose, how, tempo, cue, next, source, tabs[] {label, tab}`.
  - `tab` is monospace ASCII with aligned columns. When there's a pick row, it's the last line.
  - `hub`: `{subdivision, scorable, strokes[][]}`. `strokes[i]` is the per-note stroke list for `tabs[i]`: `D` = down, `U` = up, `L` = legato note (hammer-on or pull-off: sounds but isn't picked). `x` in a tab means a muted string.
- `tests`: `T1–T6` → `{name, technique, ex, pattern}`. `ex` is the exercise whose tab the test uses.
- `sessions`: 53 rows, Oct 22 to Dec 13, one per date. Rest days are included.
  - Row fields: `{date, week, phase, dow (0 = Mon), type: kickoff|weekday|weekend|rest, main, secondary, minutes, retest?, blocks[]}`.
  - `blocks[]`: `{key, start, end (min), kind: warmup|lab|main|secondary|combo|wrap|test|break|music|rest, title, text, items?: [{ex, focus?}], chunks?: [{i, min}], log: none|note|tempo|test|session, test?|tests?}`.
  - `chunks` is the A/B order: `i` indexes `items`. Weekday main = A2 B2 A2 B2 and secondary = A3 B3. The Saturday circuit is 4 × 4–5 min.

## Tasks
1. **Program loader** (`data/program.ts`, pure, tested). Resolve a real date to a program date as `programDate = date − Σ shift.days` over every shift with `from ≤ date`. Shifts are `{from, days: 7}[]`, stored in IndexedDB. Pressing "Repeat this week" adds `{from: next Monday, days: 7}`. Tests:
   - No shifts: Dec 13 maps to itself.
   - One shift from Nov 2: Nov 2 maps to Oct 26, and Dec 20 maps to Dec 13.
   - Dates before the kickoff, or past the end, return before/after states.
   The reference implementation is `programDate()` / `sessionFor()` in the artifact page.
2. **Map to `Exercise`.** Only where `hub.scorable` is true (alternate, economy, skipping, C0):
   - Build `pattern` by cycling `hub.strokes[0]` (or the concatenation of all variants when their total divides `beatsPerBar × subdivision`) until it fills one bar.
   - Lanes: D → `down`, U → `up`.
   - `ramp = {stepBpm: 4, cleanRepsToAdvance: 4}`. The program says 4 clean reps; the app default is 3.
   - Economy puts same-direction strokes back to back. Scoring already takes direction from the slot lane, so keep it that way.
   - Sweep and legato exercises (`scorable: false`) get timer + click + manual logging only. No onset scoring, because hammer-ons and multi-string sweeps are out of scope.
3. **Session screen.** Match the artifact:
   - Block list with minute stamps.
   - One timer runs the whole session: a 2-tone chime at each block change, 1 tone plus a flash at each chunk switch.
   - The active exercise card is highlighted, and every card shows its `tabs` in a monospace box.
   - `lab`, `warmup`, `break` and `music` blocks run with **no click and no scoring**. The motion lab is meant to be speed-first exploration without a click.
4. **Logging.** Use the same keys as the artifact, so imports line up:
   - Exercises: `"{blockKey}:{exId}:{focus}"` → `{ct, tension, clean3, note}`.
   - Tests: `"test:{Tn}"` → `{ct, mc, tension, video, note}`.
   - Notes: `"{blockKey}:note"` → `{note}`.
   - Also per session: `note` and `done`.
   - When scoring ran, auto-fill `ct` from the session's clean max, and keep it editable.
   - Show "Last: N bpm (date)" from the latest earlier entry with the same exercise id.
5. **Advancement hints.** Show these but never enforce them:
   - Raise the click after 4 clean reps.
   - Suggest the next exercise after 3/3 clean on 2 different days.
   - Show a "drop back" warning when tension is above 5.
   - Put "Repeat this week" on the Saturday review of Weeks 2, 4 and 6.
6. **History.** Add a test table showing CT / MC per test date, with the CT–MC gap and the % change from the kickoff. Add a CT line per test to the existing SVG chart.
7. **Import.** Accept `practice-log/v1`: `{schema, program, exportedAt, config: {shifts[]}, sessions: [{date, programDate, week, entries{}, note, done, updatedAt}]}`. Merge into IndexedDB by `date`, and within a date by entry key, newest `updatedAt` wins. Import `config.shifts` too.
8. **Burst mode.** Keep it, but label it "probe" for program exercises. The research behind the program treats short fast attempts as a way to find a motion, not as speed-building reps. They already stay out of personal bests.

## Out of scope
- Detecting sweeps or hammer-ons from audio, or multi-string accuracy. Score timing only.
- Editing the program in the app. To change the program, regenerate the JSON (the source is `build_program.py`, which Trent has).

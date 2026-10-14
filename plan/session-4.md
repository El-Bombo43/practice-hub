# Session 4 — Strum patterns, pattern builder, history, polish

**Done when:**
- The strum screen matches `design/strum.html`.
- Custom patterns can be built and saved.
- A History screen shows progress over time.
- `npm run build` and `npm test` pass.

## Tasks
1. **Strum screen**, following `design/strum.html`. One column per 16th, and each column shows:
   - an arrow: big for a strike, faded for a ghost, accent-colored with ">" for an accent
   - a timing dot
   - the count label (1 e & a …)
   - for accents, a loudness % underneath
2. **Accent scoring.**
   - Accent loudness % = accent peak / mean peak of the non-accent strums in the same bar − 1.
   - An accent lands if it's at least 15% louder.
   - Stat: "Accents landed x/y". Tip: "Beat-N accent is getting lost — only N% louder."
   - Strum direction isn't detected; arrows are guidance only.
3. **Pattern builder.**
   - 1 bar, choose 8ths or 16ths.
   - Click a slot to cycle: rest → strike → accent → ghost. Direction comes from slot parity.
   - Name and save it; custom patterns appear in the library and can be added to routines.
   - Unit-test the slot cycling and serialization.
4. **History screen.**
   - Per exercise: a clean-max-over-time line (hand-drawn SVG).
   - Streak calendar for the last 8 weeks.
   - Recent sessions list.
   - Total minutes per instrument this week.
5. **Polish.**
   - Level-up and new-PB toasts (3 s).
   - A small "+XP" flash on clean reps.
   - Empty states.
   - A "Today's routine" summary at the end of the session.

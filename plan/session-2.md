# Session 2 — MIDI pads (drum module)

**Done when:**
- Pads on the MPK Mini Plus are scored with the right timing and velocity.
- The drum screen matches `design/drums.html`.
- `npm run build` and `npm test` pass.

## Tasks
1. **`input/midi.ts`:**
   - `navigator.requestMIDIAccess()`, a device picker, and the last device remembered.
   - Note-on with velocity > 0 → InputEvent with velocity/127.
   - Convert `event.timeStamp` (performance.now ms) to audio-clock seconds using `ctx.getOutputTimestamp()` (`contextTime`, `performanceTime`). Recompute the mapping each bar.
   - Unit-test the conversion.
2. **Pad mapping (learn mode):**
   - "Hit the pad for LEFT hand", then RIGHT. Store the note numbers.
   - Lanes are now enforced for pad exercises: a hit with the wrong hand counts as out of window and shows as wrong-lane in the timeline.
3. **Footswitch (optional learn):**
   - Map any MIDI CC to Pause/Start and to Next exercise.
   - The user has an M-Vave Chocolate Plus sending CC messages.
   - Act only on value ≥ 64.
4. **Latency calibration, per input source:**
   - 16 clicks at 90 BPM; the user hits along.
   - Ignore hits more than 150 ms from a click. The median offset becomes the stored calibration.
   - Show the result and a Redo button.
   - Unit-test the median and outlier handling.
5. **Drum screen**, following `design/drums.html`:
   - Sticking row (R/L letters) above the timeline.
   - Two lanes: R on top, L on the bottom.
   - A velocity bar under each hit.
   - L/R balance stat = mean L velocity / mean R velocity, as a percent of the weaker over the stronger.
6. **Tip rules** (first match wins):
   - Balance under 90% → "[Hand] is softer — about N% quieter. Next rep starts on the [weaker] hand."
     - Offer a "lead with weaker hand" toggle that mirrors the sticking.
   - Mean offsets of the two hands differ by more than 8 ms → "[Hand] drags/rushes by N ms."
   - Otherwise, the session 1 rule.
7. The keyboard simulator keeps working (F = L, J = R, velocity 0.8).

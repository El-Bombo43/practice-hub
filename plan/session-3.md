# Session 3 — Guitar audio input (picking)

**Done when:**
- Guitar picked through the audio interface (Focusrite Scarlett, clean DI signal) is detected and scored.
- Upstroke/downstroke evenness is shown.
- Burst and endurance modes work.
- `npm run build` and `npm test` pass.

## Tasks
1. **Input.**
   - `getUserMedia({ audio: { deviceId, echoCancellation: false, noiseSuppression: false, autoGainControl: false } })`.
   - Input picker that remembers the last device.
   - Mono: use the channel with the highest RMS over the first second.
   - Show this note in the UI: "Close Ableton first — it can hold the interface."
2. **Onset detector.**
   - Put the core in `input/onsetCore.ts` as a pure, testable function. A thin AudioWorklet calls it.
   - One-pole high-pass at about 80 Hz.
   - Frame 256 samples, hop 128.
   - Log-energy rise detector with an adaptive threshold: median energy of the last 1 s + k (k is the Sensitivity slider).
   - Refractory 35 ms.
   - Report onset time to sample accuracy within the frame (first sample over the threshold), plus peak RMS over the next 30 ms.
   - Post `{time, peak}` to the main thread, where time is converted to audio-clock seconds.
3. **Detector tests.** Generate decaying noise bursts at known times, with and without background noise, at 16ths up to 200 BPM. Assert detection within ±5 ms with no doubles.
4. **Level meter and sensitivity slider** in an input setup panel.
5. **Latency calibration:** same flow as session 2; the user picks muted strings on each click.
6. **Picking scoring.**
   - Strokes are assigned down/up by order within the rep (strict alternate, first note down).
   - Evenness: mean offset and mean peak, up vs down.
   - Tip if the offsets differ by more than 8 ms or the loudness by more than 10%: "Upstrokes are dragging — +N ms late on average."
7. **Burst mode:** 3 bars at clean max + 8 BPM, 1 bar rest, repeating.
8. **Endurance mode:** hold the tempo for 30 or 60 s, then report accuracy and drift (slope of offset over time, ms per 10 s).
9. The keyboard simulator keeps working for guitar exercises.

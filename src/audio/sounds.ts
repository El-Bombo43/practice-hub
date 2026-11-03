export type Sound = 'click' | 'wood' | 'beep' | 'rock' | 'halftime';
export const SOUNDS: { id: Sound; label: string }[] = [
  { id: 'click', label: 'Click' }, { id: 'wood', label: 'Wood block' }, { id: 'beep', label: 'Beep' },
  { id: 'rock', label: 'Rock beat' }, { id: 'halftime', label: 'Half-time beat' },
];
export const isDrums = (s: Sound): boolean => s === 'rock' || s === 'halftime';

export type Drum = 'kick' | 'snare' | 'hat';

/**
 * Which drums sound on a tick. Rock: kick on 1 and 3, snare on 2 and 4. Half-time: kick on 1, snare on the
 * middle of the bar. The hat plays every tick (quieter between the beats), so it follows whatever subdivision
 * the exercise is counted in. Other bar lengths fall back to kick on even beats and snare on odd ones.
 */
export function drumHits(sound: Sound, beat: number, sub: number, beatsPerBar: number): Drum[] {
  const out: Drum[] = ['hat'];
  if (sub !== 0) return out;
  if (sound === 'halftime') {
    if (beat === 0) out.push('kick');
    else if (beat === Math.floor(beatsPerBar / 2)) out.push('snare');
  } else if (beat % 2 === 0) out.push('kick');
  else out.push('snare');
  return out;
}

/** Tap tempo: BPM from the gaps between recent taps (ms timestamps). A pause over 2 s starts again. Needs 3 taps. */
export function tapTempo(taps: number[]): number | null {
  let run = [taps[taps.length - 1]];
  for (let i = taps.length - 2; i >= 0 && taps[i + 1] - taps[i] <= 2000; i--) run = [taps[i], ...run];
  run = run.slice(-8);
  if (run.length < 3) return null;
  const gaps = run.slice(1).map((t, i) => t - run[i]).sort((a, b) => a - b);
  const mid = gaps.length >> 1;
  const median = gaps.length % 2 ? gaps[mid] : (gaps[mid - 1] + gaps[mid]) / 2;
  return Math.max(30, Math.min(300, Math.round(60000 / median)));
}

import type { Exercise, Lane, Slot } from './types';

const BEATS = 4;
const alt = (a: Lane, b: Lane) => (i: number): Lane => (i % 2 === 0 ? a : b);
const downUp = alt('down', 'up');

/** Every slot played, picking lanes alternating down/up starting with down. */
const picking = (n: number): Slot[] => Array.from({ length: n }, (_, i) => ({ play: true, lane: downUp(i) }));

/** Build a pattern from a string of chars: 'x' play, 'X' accent, 'g' ghost, '-' rest. */
function fromString(s: string, lane: (i: number) => Lane | undefined): Slot[] {
  return [...s].map((c, i) => {
    const l = lane(i);
    if (c === 'x') return { play: true, lane: l };
    if (c === 'X') return { play: true, lane: l, accent: true };
    if (c === 'g') return { play: false, lane: l, ghost: true };
    return { play: false, lane: l };
  });
}

/** Sticking string such as "RLRR" repeated to fill `n` slots. */
const sticking = (pat: string, n: number): Slot[] =>
  Array.from({ length: n }, (_, i) => ({ play: true, lane: pat[i % pat.length] as Lane }));

const guitar = (
  id: string, name: string, kind: Exercise['kind'], subdivision: Exercise['subdivision'],
  pattern: Slot[], startBpm: number, minutes: number, step = 4, reps = 3,
): Exercise => ({
  id, name, instrument: 'guitar', kind, beatsPerBar: BEATS, subdivision, pattern,
  startBpm, ramp: { stepBpm: step, cleanRepsToAdvance: reps }, windowMs: 15, minutes,
});
const pads = (
  id: string, name: string, subdivision: Exercise['subdivision'], pattern: Slot[],
  startBpm: number, minutes: number, step = 4, reps = 3,
): Exercise => ({
  id, name, instrument: 'pads', kind: 'rudiment', beatsPerBar: BEATS, subdivision, pattern,
  startBpm, ramp: { stepBpm: step, cleanRepsToAdvance: reps }, windowMs: 15, minutes,
});

// Guitar gallop per beat: play, rest, play, play (rests keep the picking hand moving → ghost).
const gallopGuitar = fromString('xgxx'.repeat(BEATS), downUp);
// Pads gallop per beat: R, rest, R, L.
const gallopPads: Slot[] = Array.from({ length: BEATS }, () => [
  { play: true, lane: 'R' as Lane },
  { play: false },
  { play: true, lane: 'R' as Lane },
  { play: true, lane: 'L' as Lane },
]).flat();

// Strum slots: odd (1-based) = down, even = up.
const strumLane = alt('down', 'up');

export const EXERCISES: Exercise[] = [
  guitar('tremolo', 'Single-string tremolo', 'picking', 4, picking(16), 80, 3),
  guitar('chromatic', 'Chromatic 1-2-3-4, string crossing', 'picking', 4, picking(16), 90, 5),
  guitar('skipping', 'String skipping', 'picking', 4, picking(16), 80, 4),
  guitar('palm-muted', 'Palm-muted 16ths', 'picking', 4, picking(16), 90, 4),
  guitar('sextuplets', 'Sextuplets', 'picking', 6, picking(24), 60, 4),
  guitar('gallop', 'Gallop', 'picking', 4, gallopGuitar, 90, 3),
  guitar('strum-8ths', 'Basic 8th strum (D, D U, U D U)', 'strum', 2, fromString('xgxxgxxx', strumLane), 80, 3),
  guitar('strum-16th-accents', '16th strum, accents on 2 & 4', 'strum', 4,
    fromString('xgxxXxgxxgxxXxgx', strumLane), 70, 4, 2, 3),
  pads('pads-singles', 'Singles RLRL', 4, sticking('RL', 16), 80, 3),
  pads('pads-doubles', 'Doubles RRLL', 4, sticking('RRLL', 16), 70, 3),
  pads('pads-paradiddle', 'Paradiddle RLRR LRLL', 4, sticking('RLRRLRLL', 16), 70, 4),
  pads('pads-paradiddle-diddle', 'Paradiddle-diddle RLRRLL', 6, sticking('RLRRLL', 24), 60, 4),
  pads('pads-gallop', 'Gallop on pads (R RL)', 4, gallopPads, 80, 3),
];

export const exerciseById = (id: string): Exercise => {
  const ex = EXERCISES.find((e) => e.id === id);
  if (!ex) throw new Error(`Unknown exercise ${id}`);
  return ex;
};

export const hasLanes = (ex: Exercise): boolean => ex.pattern.some((s) => s.lane !== undefined);

const SWAP: Partial<Record<Lane, Lane>> = { L: 'R', R: 'L' };

/** Same exercise with the sticking mirrored (R↔L), for leading with the other hand. */
export const mirrorExercise = (ex: Exercise): Exercise => ({
  ...ex,
  pattern: ex.pattern.map((s) => (s.lane && SWAP[s.lane] ? { ...s, lane: SWAP[s.lane] } : s)),
});

export const firstLane = (ex: Exercise): Lane | undefined => ex.pattern.find((s) => s.play && s.lane)?.lane;

/** Repeating lane cycle for exercises whose pattern is uniform, so they can be re-counted (8ths, triplets, 16ths, sextuplets). */
const CYCLES: Record<string, Lane[]> = {
  tremolo: ['down', 'up'],
  chromatic: ['down', 'up'],
  skipping: ['down', 'up'],
  'palm-muted': ['down', 'up'],
  sextuplets: ['down', 'up'],
  'pads-singles': ['R', 'L'],
  'pads-doubles': ['R', 'R', 'L', 'L'],
  'pads-paradiddle': ['R', 'L', 'R', 'R', 'L', 'R', 'L', 'L'],
  'pads-paradiddle-diddle': ['R', 'L', 'R', 'R', 'L', 'L'],
};

/** Counts this exercise can be played in. The cycle must fit the bar so the sticking never restarts mid-bar. */
export function allowedSubdivisions(ex: Exercise): Exercise['subdivision'][] {
  const c = CYCLES[ex.id];
  if (!c) return [ex.subdivision];
  return ([2, 3, 4, 6] as const).filter((s) => (ex.beatsPerBar * s) % c.length === 0);
}

/**
 * The exercise re-counted in another subdivision. Its id gets an `@n` suffix so personal bests,
 * last BPM and the log stay separate per count; the start tempo keeps roughly the same note speed.
 */
export function withSubdivision(ex: Exercise, sub: Exercise['subdivision']): Exercise {
  const c = CYCLES[ex.id];
  if (!c || sub === ex.subdivision || !allowedSubdivisions(ex).includes(sub)) return ex;
  return {
    ...ex,
    id: `${ex.id}@${sub}`,
    subdivision: sub,
    pattern: Array.from({ length: ex.beatsPerBar * sub }, (_, i) => ({ play: true, lane: c[i % c.length] })),
    startBpm: Math.max(30, Math.min(300, Math.round((ex.startBpm * ex.subdivision) / sub))),
  };
}

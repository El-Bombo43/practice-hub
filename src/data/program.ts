import type { Exercise, Lane, Slot } from './types';

/** `practice-program/v1`: the shape of `src/data/programs/*.json`. */
export type ProgramTab = { label: string; tab: string };
export type ProgramExercise = {
  id: string; name: string; technique: string; purpose: string; how: string; tempo: string; cue: string; next: string;
  source: string; tabs: ProgramTab[];
  hub: { subdivision: number; scorable: boolean; strokes: string[][] };
};
export type BlockKind = 'warmup' | 'lab' | 'main' | 'secondary' | 'combo' | 'wrap' | 'test' | 'break' | 'music' | 'rest';
export type ProgramBlock = {
  key: string; start: number; end: number; kind: BlockKind; title: string; text: string;
  items?: { ex: string; focus?: string }[];
  chunks?: { i: number; min: number }[];
  log: 'none' | 'note' | 'tempo' | 'test' | 'session';
  test?: string; tests?: string[];
};
export type ProgramSession = {
  date: string; week: number; phase: number; dow: number; type: 'kickoff' | 'weekday' | 'weekend' | 'rest';
  main: string | null; secondary: string | null; minutes: number; retest?: boolean; blocks: ProgramBlock[];
};
export type Program = {
  schema: 'practice-program/v1';
  program: {
    id: string; title: string; techniques: Record<string, string>; week1Monday: string; kickoff: string; end: string;
    phases: { n: number; name: string; weeks: number[]; goal: string }[]; rules: string[]; safety: string[];
  };
  exercises: Record<string, ProgramExercise>;
  tests: Record<string, { id: string; name: string; technique: string; ex: string; pattern: string }>;
  sessions: ProgramSession[];
};

/** Blocks that play with the click (and score, when the exercise is scorable). */
export const CLICK_KINDS: BlockKind[] = ['main', 'secondary', 'combo', 'test'];
export const hasClick = (b: ProgramBlock): boolean => CLICK_KINDS.includes(b.kind);

// ---------- dates (local YYYY-MM-DD keys, computed in UTC so DST never shifts a day) ----------

const toUtc = (key: string): number => {
  const [y, m, d] = key.split('-').map(Number);
  return Date.UTC(y, m - 1, d);
};
export const addDays = (key: string, n: number): string => new Date(toUtc(key) + n * 86400000).toISOString().slice(0, 10);
export const dayOfWeek = (key: string): number => (new Date(toUtc(key)).getUTCDay() + 6) % 7; // 0 = Monday
/** The Monday strictly after `key`. */
export const nextMonday = (key: string): string => addDays(key, 7 - dayOfWeek(key));

// ---------- shifts: "Repeat this week" ----------

export type Shift = { from: string; days: number };

/** programDate = date − Σ days over every shift with from ≤ date. */
export function programDate(date: string, shifts: Shift[]): string {
  let back = 0;
  for (const s of shifts) if (s.from <= date) back += s.days;
  return addDays(date, -back);
}

/** Pressing "Repeat this week" on `today` replays the week from the next Monday. Idempotent. */
export function addRepeat(shifts: Shift[], today: string): Shift[] {
  const from = nextMonday(today);
  if (shifts.some((s) => s.from === from)) return shifts;
  return [...shifts, { from, days: 7 }].sort((a, b) => a.from.localeCompare(b.from));
}
export const removeRepeat = (shifts: Shift[], today: string): Shift[] => shifts.filter((s) => s.from !== nextMonday(today));
export const hasRepeat = (shifts: Shift[], today: string): boolean => shifts.some((s) => s.from === nextMonday(today));

export type ProgramDay =
  | { state: 'before'; programDate: string; daysUntil: number }
  | { state: 'after'; programDate: string }
  | { state: 'during'; programDate: string; session: ProgramSession | null };

export function dayFor(date: string, program: Program, shifts: Shift[]): ProgramDay {
  const pd = programDate(date, shifts);
  const { kickoff, end } = program.program;
  if (pd < kickoff) return { state: 'before', programDate: pd, daysUntil: Math.round((toUtc(kickoff) - toUtc(pd)) / 86400000) };
  if (pd > end) return { state: 'after', programDate: pd };
  return { state: 'during', programDate: pd, session: program.sessions.find((s) => s.date === pd) ?? null };
}

/** Weekly Saturday reviews where the "Repeat this week" button is offered. */
export const offersRepeat = (s: ProgramSession): boolean => s.dow === 5 && [2, 4, 6].includes(s.week);

// ---------- exercise mapping ----------

const gcd = (a: number, b: number): number => (b ? gcd(b, a % b) : a);
const lcm = (a: number, b: number): number => (a / gcd(a, b)) * b;

/**
 * The stroke cycle for one bar: the first variant, or all variants joined when their total divides the bar.
 * The bar is 4 beats when the cycle fits; otherwise it is the smallest whole number of beats holding whole
 * cycles (at least 4 beats, at most 12; beyond that the cycle is cut at 4 beats).
 */
export function barStrokes(hub: ProgramExercise['hub']): { strokes: string[]; beatsPerBar: number } {
  const sub = hub.subdivision;
  const variants = hub.strokes.filter((v) => v.length > 0);
  if (!variants.length) return { strokes: [], beatsPerBar: 4 };
  const total = variants.reduce((n, v) => n + v.length, 0);
  const cycle = variants.length > 1 && (4 * sub) % total === 0 ? variants.flat() : variants[0];
  let slots = 4 * sub;
  if (slots % cycle.length !== 0) {
    let s = lcm(cycle.length, sub);
    while (s / sub < 4) s *= 2;
    slots = s / sub <= 12 ? s : 4 * sub;
  }
  return { strokes: Array.from({ length: slots }, (_, i) => cycle[i % cycle.length]), beatsPerBar: slots / sub };
}

const LANE: Record<string, Lane | undefined> = { D: 'down', U: 'up' };
const START_BPM: Record<number, number> = { 2: 90, 3: 80, 4: 80, 6: 60 };

export const programExerciseId = (exId: string): string => `prog-${exId}`;
export const isProgramExerciseId = (id: string): boolean => id.startsWith('prog-');

/**
 * Hub exercise for a program exercise. Scorable ones (alternate, economy, skipping, C0) get a real pattern with
 * lanes from the strokes; the rest (sweeps, legato) get an unscored exercise: click and timer only.
 */
export function toHubExercise(pe: ProgramExercise, minutes = 4): Exercise {
  const sub = (pe.hub.subdivision === 2 || pe.hub.subdivision === 3 || pe.hub.subdivision === 6 ? pe.hub.subdivision : 4) as Exercise['subdivision'];
  const base = {
    id: programExerciseId(pe.id), name: `${pe.id} · ${pe.name}`, instrument: 'guitar' as const, kind: 'picking' as const,
    subdivision: sub, startBpm: START_BPM[sub] ?? 80, windowMs: 15, minutes,
    ramp: { stepBpm: 4, cleanRepsToAdvance: 4, manual: true }, // the program says 4 clean reps; raising the click stays the player's call
  };
  if (!pe.hub.scorable) {
    const slots = 4 * sub;
    return { ...base, beatsPerBar: 4, unscored: true, pattern: Array.from({ length: slots }, (): Slot => ({ play: false })) };
  }
  const { strokes, beatsPerBar } = barStrokes(pe.hub);
  const pattern = strokes.map((c): Slot => {
    const lane = LANE[c];
    return lane ? { play: true, lane } : { play: false, ghost: true }; // legato 'L': the hand moves, no onset expected
  });
  return { ...base, beatsPerBar, pattern: pattern.length ? pattern : Array.from({ length: 4 * sub }, (): Slot => ({ play: false })) };
}

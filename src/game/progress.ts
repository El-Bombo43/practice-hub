import type { Exercise } from '../data/types';
import { rampStep, updateCombo, type RepResult } from '../scoring/score';

export const XP_PER_LEVEL = 1000;
export const STREAK_MINUTES = 10;

export type LogEntry = {
  id: string; // stint id, so a stint updates one entry in place
  date: string; // local YYYY-MM-DD
  exerciseId: string;
  minutes: number;
  bestCleanBpm: number | null;
  accuracy: number | null; // 0-100
};

export type Profile = {
  xp: number;
  bestCombo: number;
  personalBests: Record<string, number>; // exercise id → highest BPM with a clean rep
  lastBpm: Record<string, number>;
  dayMinutes: Record<string, number>; // local date → minutes practised
  log: LogEntry[];
};

export const emptyProfile = (): Profile => ({
  xp: 0, bestCombo: 0, personalBests: {}, lastBpm: {}, dayMinutes: {}, log: [],
});

/** State for the exercise currently being practised (resets when the exercise changes). */
export type Live = {
  combo: number;
  bestCombo: number;
  cleanStreak: number;
  reps: number;
  expected: number;
  onTime: number;
  offsetSum: number;
  offsetN: number;
  bestCleanBpm: number | null;
};

export const emptyLive = (): Live => ({
  combo: 0, bestCombo: 0, cleanStreak: 0, reps: 0, expected: 0, onTime: 0, offsetSum: 0, offsetN: 0,
  bestCleanBpm: null,
});

export const MIN_BPM = 30;
export const MAX_BPM = 300;
export const clampBpm = (n: number): number => Math.max(MIN_BPM, Math.min(MAX_BPM, Math.round(n)));

export type RepOutcome = {
  live: Live;
  profile: Profile;
  bpm: number; // tempo to use for the next rep
  xpGain: number;
  stepped: boolean;
  newBest: boolean;
};

/** Fold one scored rep into the live state and the profile. Pure. */
export function applyRep(live: Live, profile: Profile, ex: Exercise, bpm: number, rep: RepResult): RepOutcome {
  const c = updateCombo(live.combo, live.bestCombo, rep.sequence);
  const ramp = rampStep(live.cleanStreak, rep.clean, ex.ramp);
  const prevBest = profile.personalBests[ex.id];
  const newBest = rep.clean && prevBest !== undefined && bpm > prevBest;

  let xpGain = rep.onTimeCount;
  if (rep.clean) xpGain += 10;
  if (ramp.advance) xpGain += 25;
  if (newBest) xpGain += 50;

  const nextBpm = ramp.advance ? clampBpm(bpm + ex.ramp.stepBpm) : bpm;
  const personalBests = rep.clean && (prevBest === undefined || bpm > prevBest)
    ? { ...profile.personalBests, [ex.id]: bpm }
    : profile.personalBests;

  const nextLive: Live = {
    combo: c.combo,
    bestCombo: c.best,
    cleanStreak: ramp.cleanStreak,
    reps: live.reps + 1,
    expected: live.expected + rep.expectedCount,
    onTime: live.onTime + rep.onTimeCount,
    offsetSum: live.offsetSum + (rep.meanOffsetMs ?? 0) * rep.hitCount,
    offsetN: live.offsetN + rep.hitCount,
    bestCleanBpm: rep.clean ? Math.max(live.bestCleanBpm ?? 0, bpm) : live.bestCleanBpm,
  };
  const nextProfile: Profile = {
    ...profile,
    xp: profile.xp + xpGain,
    bestCombo: Math.max(profile.bestCombo, c.best),
    personalBests,
    lastBpm: { ...profile.lastBpm, [ex.id]: nextBpm },
  };
  return { live: nextLive, profile: nextProfile, bpm: nextBpm, xpGain, stepped: ramp.advance, newBest };
}

export const level = (xp: number): number => Math.floor(xp / XP_PER_LEVEL) + 1;
export const xpInLevel = (xp: number): number => xp % XP_PER_LEVEL;

export const dateKey = (d: Date = new Date()): string => {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

/**
 * Consecutive local days with ≥ STREAK_MINUTES of practice. Today doesn't break the streak
 * while it's still short of the target; it counts once it reaches it.
 */
export function streak(dayMinutes: Record<string, number>, today: Date = new Date()): number {
  const ok = (d: Date) => (dayMinutes[dateKey(d)] ?? 0) >= STREAK_MINUTES;
  const d = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  if (!ok(d)) d.setDate(d.getDate() - 1);
  let n = 0;
  while (ok(d)) { n++; d.setDate(d.getDate() - 1); }
  return n;
}

export const accuracyPct = (live: Live): number | null =>
  live.expected ? Math.round((100 * live.onTime) / live.expected) : null;

export const meanOffset = (live: Live): number | null =>
  live.offsetN ? live.offsetSum / live.offsetN : null;

export type Tip = { tone: 'late' | 'early' | 'ok' | 'idle'; title: string; text: string };

/** One simple rule: a mean offset beyond 8 ms either way. */
export function tip(mean: number | null): Tip {
  if (mean === null) return { tone: 'idle', title: 'Ready', text: 'Start the click, then play along with F and J.' };
  const ms = Math.round(Math.abs(mean));
  if (mean > 8) return { tone: 'late', title: 'Mostly late', text: `+${ms} ms on average. Relax.` };
  if (mean < -8) return { tone: 'early', title: 'Mostly early', text: `−${ms} ms on average. Don't rush.` };
  return { tone: 'ok', title: 'Locked in', text: 'Within 8 ms of the click on average. Keep going.' };
}

export const MAX_LOG = 500;

/** Insert or update the log entry for the current stint. */
export function upsertLog(profile: Profile, entry: LogEntry): Profile {
  const i = profile.log.findIndex((e) => e.id === entry.id);
  const log = i >= 0 ? profile.log.map((e, j) => (j === i ? entry : e)) : [...profile.log, entry];
  return { ...profile, log: log.length > MAX_LOG ? log.slice(-MAX_LOG) : log };
}

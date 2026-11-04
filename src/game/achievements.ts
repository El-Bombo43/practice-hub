import type { ProgramLog } from '../data/programLog';
import type { Profile } from './progress';
import { level, streak } from './progress';

export type BadgeContext = {
  profile: Profile;
  programLog: ProgramLog;
  /** Notes per second for a clean tempo on an exercise key, or null when the exercise is not guitar picking. */
  pickRate: (exerciseId: string, bpm: number) => number | null;
  today?: Date;
};

export type Badge = { id: string; title: string; hint: string; earned: (c: BadgeContext) => boolean };

const totalMinutes = (p: Profile): number => Object.values(p.dayMinutes).reduce((a, b) => a + b, 0);
const bestRate = (c: BadgeContext): number =>
  Math.max(0, ...Object.entries(c.profile.personalBests).map(([id, bpm]) => c.pickRate(id, bpm) ?? 0));
const programDone = (c: BadgeContext): number => Object.values(c.programLog.sessions).filter((s) => s.done).length;
const testsLogged = (c: BadgeContext): number =>
  new Set(Object.values(c.programLog.sessions).flatMap((s) => Object.entries(s.entries).filter(([k, e]) => k.startsWith('test:') && typeof e.ct === 'number').map(([k]) => k))).size;

const speed = (n: number): Badge => ({
  id: `speed-${n}`, title: `${n} notes a second`, hint: `Land a clean bar of picking at ${n} notes per second.`, earned: (c) => bestRate(c) >= n,
});
const streakBadge = (n: number): Badge => ({
  id: `streak-${n}`, title: `${n}-day streak`, hint: `Practise 10+ minutes on ${n} days in a row.`, earned: (c) => streak(c.profile.dayMinutes, c.today) >= n,
});

export const BADGES: Badge[] = [
  { id: 'first-clean', title: 'First clean bar', hint: 'Play one bar with every note in the window.', earned: (c) => Object.keys(c.profile.personalBests).length > 0 },
  speed(8), speed(10), speed(12), speed(14), speed(16),
  streakBadge(3), streakBadge(7), streakBadge(14), streakBadge(30),
  { id: 'level-5', title: 'Level 5', hint: 'Earn 4,000 XP.', earned: (c) => level(c.profile.xp) >= 5 },
  { id: 'level-10', title: 'Level 10', hint: 'Earn 9,000 XP.', earned: (c) => level(c.profile.xp) >= 10 },
  { id: 'hours-2', title: 'Two hours in', hint: 'Practise for 2 hours in total.', earned: (c) => totalMinutes(c.profile) >= 120 },
  { id: 'hours-10', title: 'Ten hours in', hint: 'Practise for 10 hours in total.', earned: (c) => totalMinutes(c.profile) >= 600 },
  { id: 'combo-100', title: 'Century combo', hint: 'Hit 100 on-time notes in a row.', earned: (c) => c.profile.bestCombo >= 100 },
  { id: 'combo-500', title: 'Unbroken', hint: 'Hit 500 on-time notes in a row.', earned: (c) => c.profile.bestCombo >= 500 },
  { id: 'burst', title: 'Probe', hint: 'Land a clean bar in burst mode.', earned: (c) => c.profile.runs.some((r) => r.type === 'burst' && r.cleanReps > 0) },
  {
    id: 'steady-hands', title: 'Steady hands', hint: 'Finish an endurance run at 90%+ with drift under 1 ms per 10 s.',
    earned: (c) => c.profile.runs.some((r) => r.type === 'endurance' && (r.accuracy ?? 0) >= 90 && r.drift !== null && Math.abs(r.drift) < 1),
  },
  { id: 'first-test', title: 'Baseline set', hint: 'Log a clean tempo for a program test.', earned: (c) => testsLogged(c) > 0 },
  { id: 'all-tests', title: 'Full picture', hint: 'Log clean tempos for 5 different program tests.', earned: (c) => testsLogged(c) >= 5 },
  { id: 'program-5', title: 'Five program sessions', hint: 'Mark 5 program sessions done.', earned: (c) => programDone(c) >= 5 },
  { id: 'program-20', title: 'Twenty program sessions', hint: 'Mark 20 program sessions done.', earned: (c) => programDone(c) >= 20 },
];

/** Badges satisfied now that the player does not have yet. */
export function newBadges(c: BadgeContext, have: Record<string, string>): Badge[] {
  return BADGES.filter((b) => !have[b.id] && b.earned(c));
}

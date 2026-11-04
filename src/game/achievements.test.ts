import { describe, expect, it } from 'vitest';
import { BADGES, newBadges, type BadgeContext } from './achievements';
import { emptyProfile, type Profile } from './progress';

const ctx = (p: Partial<Profile> = {}, over: Partial<BadgeContext> = {}): BadgeContext => ({
  profile: { ...emptyProfile(), ...p },
  programLog: { shifts: [], sessions: {} },
  pickRate: (id, bpm) => (id.startsWith('tremolo') ? (bpm * 4) / 60 : null), // 16ths
  today: new Date(2026, 10, 4),
  ...over,
});
const ids = (c: BadgeContext, have: Record<string, string> = {}) => newBadges(c, have).map((b) => b.id);

describe('badges', () => {
  it('ids are unique', () => {
    expect(new Set(BADGES.map((b) => b.id)).size).toBe(BADGES.length);
  });
  it('nothing for a fresh profile', () => {
    expect(ids(ctx())).toEqual([]);
  });
  it('first clean bar and speed milestones use notes per second, only for picking', () => {
    expect(ids(ctx({ personalBests: { tremolo: 150 } }))).toEqual(['first-clean', 'speed-8', 'speed-10']); // 150 BPM 16ths = 10 /s
    expect(ids(ctx({ personalBests: { 'pads-singles': 200 } }))).toEqual(['first-clean']);
  });
  it('streaks, levels, hours and combos', () => {
    const dayMinutes = { '2026-11-04': 12, '2026-11-03': 15, '2026-11-02': 10 };
    const got = ids(ctx({ dayMinutes, xp: 4200, bestCombo: 120 }));
    expect(got).toEqual(expect.arrayContaining(['streak-3', 'level-5', 'combo-100']));
    expect(got).not.toContain('streak-7');
    expect(got).not.toContain('hours-2');
  });
  it('does not re-award what the player already has', () => {
    expect(ids(ctx({ personalBests: { tremolo: 150 } }), { 'first-clean': '2026-10-01', 'speed-8': '2026-10-01' })).toEqual(['speed-10']);
  });
  it('runs: burst needs a clean bar; steady hands needs accuracy and low drift', () => {
    const run = { id: 'a', date: 'd', exerciseId: 'tremolo', bpm: 100, seconds: 60, reps: 10, cleanReps: 0, onTime: 1, expected: 1, accuracy: 92, drift: 0.4, meanOffset: 1 };
    expect(ids(ctx({ runs: [{ ...run, type: 'endurance' }] }))).toEqual(['steady-hands']);
    expect(ids(ctx({ runs: [{ ...run, type: 'endurance', drift: 3 }] }))).toEqual([]);
    expect(ids(ctx({ runs: [{ ...run, type: 'burst', cleanReps: 1 }] }))).toEqual(['burst']);
  });
  it('program badges come from the program log', () => {
    const sessions = Object.fromEntries(Array.from({ length: 5 }, (_, i) => {
      const date = `2026-10-2${i}`;
      return [date, { date, programDate: date, week: 1, updatedAt: 1, done: true, entries: { [`test:T${i + 1}`]: { ct: 100 } } }];
    }));
    expect(ids(ctx({}, { programLog: { shifts: [], sessions } }))).toEqual(['first-test', 'all-tests', 'program-5']);
  });
});

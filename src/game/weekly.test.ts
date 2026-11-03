import { describe, expect, it } from 'vitest';
import { summaryText, weeklySummary } from './weekly';
import { emptyProfile, type LogEntry, type Profile } from './progress';
import type { ProgramLog } from '../data/programLog';

const e = (date: string, exerciseId: string, minutes: number, bestCleanBpm: number | null, i = 0): LogEntry =>
  ({ id: `${date}${exerciseId}${i}`, date, exerciseId, minutes, bestCleanBpm, accuracy: 90 });
const inst = (id: string) => (id.startsWith('pads') ? 'pads' : id === 'zzz' ? undefined : 'guitar');
const wed = new Date(2026, 10, 4); // Wed 4 Nov 2026 → week Mon 2 Nov – Sun 8 Nov
const profile = (log: LogEntry[], extra: Partial<Profile> = {}): Profile => ({ ...emptyProfile(), log, ...extra });
const noLog: ProgramLog = { shifts: [], sessions: {} };

describe('weeklySummary', () => {
  const log = [
    e('2026-10-30', 'tremolo', 20, 100), e('2026-11-01', 'pads-singles', 15, 80), // last week
    e('2026-11-02', 'tremolo', 12, 104), e('2026-11-03', 'tremolo', 6, 108, 1), e('2026-11-03', 'pads-singles', 8, 80), e('2026-11-04', 'chromatic', 5, 90),
  ];
  const s = weeklySummary(profile(log, { dayMinutes: { '2026-11-03': 14, '2026-11-02': 12 } }), noLog, [], inst, wed);
  it('covers Monday to Sunday only', () => {
    expect([s.from, s.to]).toEqual(['2026-11-02', '2026-11-08']);
    expect(s.minutes).toEqual({ guitar: 23, pads: 8, total: 31 });
    expect(s.practiceDays).toBe(3);
  });
  it('lists clean maxes set or beaten this week, biggest gain first', () => {
    expect(s.pbs).toEqual([{ exerciseId: 'tremolo', from: 100, to: 108 }, { exerciseId: 'chromatic', from: null, to: 90 }]);
  });
  it('counts runs and finished program sessions, and carries the streak', () => {
    const p = profile(log, { runs: [
      { id: 'a', date: '2026-11-03', type: 'burst', exerciseId: 'tremolo', bpm: 110, seconds: null, reps: 3, cleanReps: 2, onTime: 40, expected: 48, accuracy: 83, drift: null, meanOffset: null },
      { id: 'b', date: '2026-10-20', type: 'endurance', exerciseId: 'tremolo', bpm: 100, seconds: 60, reps: 10, cleanReps: 5, onTime: 100, expected: 160, accuracy: 62, drift: 2, meanOffset: 3 },
    ] });
    const pl: ProgramLog = { shifts: [], sessions: { '2026-11-03': { date: '2026-11-03', programDate: '2026-11-03', week: 3, entries: {}, done: true, updatedAt: 1 } } };
    const r = weeklySummary(p, pl, [], inst, wed);
    expect([r.bursts, r.endurance, r.programDone]).toEqual([1, 0, 1]);
  });
  it('shows the latest test CT this week and its change from the previous one', () => {
    const pl: ProgramLog = { shifts: [], sessions: {
      '2026-10-22': { date: '2026-10-22', programDate: '', week: 1, entries: { 'test:T1': { ct: 120 } }, updatedAt: 1 },
      '2026-11-03': { date: '2026-11-03', programDate: '', week: 3, entries: { 'test:T1': { ct: 132 }, 'test:T2': { ct: 90 } }, updatedAt: 1 },
    } };
    expect(weeklySummary(profile([]), pl, ['T1', 'T2', 'T3'], inst, wed).tests).toEqual([{ id: 'T1', ct: 132, delta: 12 }, { id: 'T2', ct: 90, delta: null }]);
  });
  it('writes a short pasteable summary', () => {
    const text = summaryText(s, (id) => id.toUpperCase(), (id) => id);
    expect(text).toContain('Practice week 2026-11-02 to 2026-11-08');
    expect(text).toContain('31 min over 3 days (guitar 23, pads 8)');
    expect(text).toContain('TREMOLO 100 → 108');
    expect(text).toContain('CHROMATIC first → 90');
  });
  it('an empty week is all zeros', () => {
    const r = weeklySummary(profile([]), noLog, [], inst, wed);
    expect(r.minutes.total).toBe(0);
    expect(r.pbs).toEqual([]);
  });
});

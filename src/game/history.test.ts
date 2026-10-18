import { describe, expect, it } from 'vitest';
import { calendarWeeks, cleanMaxSeries, exercisesWithHistory, weekMinutes, weekStartKey } from './history';
import type { LogEntry } from './progress';

const e = (date: string, exerciseId: string, bestCleanBpm: number | null, minutes = 5): LogEntry =>
  ({ id: date + exerciseId + bestCleanBpm, date, exerciseId, minutes, bestCleanBpm, accuracy: 90 });

describe('cleanMaxSeries', () => {
  it('is the running max per day, ignoring other exercises and stints without a clean rep', () => {
    const log = [e('2026-10-01', 'a', 100), e('2026-10-01', 'a', 104), e('2026-10-02', 'a', 98), e('2026-10-02', 'b', 200),
      e('2026-10-03', 'a', null), e('2026-10-04', 'a', 110)];
    expect(cleanMaxSeries(log, 'a')).toEqual([
      { date: '2026-10-01', bpm: 104 }, { date: '2026-10-02', bpm: 104 }, { date: '2026-10-04', bpm: 110 },
    ]);
    expect(cleanMaxSeries(log, 'zzz')).toEqual([]);
  });
  it('lists exercises with history, newest first', () => {
    expect(exercisesWithHistory([e('2026-10-01', 'a', 90), e('2026-10-02', 'b', 90), e('2026-10-03', 'c', null), e('2026-10-04', 'a', 95)])).toEqual(['a', 'b']);
  });
});

describe('calendarWeeks', () => {
  const today = new Date(2026, 9, 14); // Wednesday
  const weeks = calendarWeeks({ '2026-10-13': 12, '2026-10-12': 40, '2026-10-05': 3 }, today, 8);
  it('8 Monday-first weeks ending with the current one', () => {
    expect(weeks.length).toBe(8);
    expect(weeks.every((w) => w.length === 7)).toBe(true);
    expect(weeks[7][0].date).toBe('2026-10-12');
    expect(weeks[0][0].date).toBe('2026-08-24');
  });
  it('levels and future days', () => {
    expect(weeks[7][0].level).toBe(3);
    expect(weeks[7][1].level).toBe(2);
    expect(weeks[6][0].level).toBe(1);
    expect(weeks[7][2].future).toBe(false); // today
    expect(weeks[7][3].future).toBe(true);
  });
});

describe('weekMinutes', () => {
  it('sums this week per instrument', () => {
    const today = new Date(2026, 9, 14);
    expect(weekStartKey(today)).toBe('2026-10-12');
    const log = [e('2026-10-11', 'g', 90, 50), e('2026-10-12', 'g', 90, 5.5), e('2026-10-13', 'p', 80, 4), e('2026-10-14', 'g@2', 80, 3)];
    const inst = (id: string) => (id.startsWith('p') ? 'pads' : id.startsWith('g') ? 'guitar' : undefined);
    expect(weekMinutes(log, inst, today)).toEqual({ guitar: 8.5, pads: 4 });
  });
});

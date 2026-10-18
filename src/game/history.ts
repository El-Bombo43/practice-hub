import { dateKey, type LogEntry } from './progress';

/** Clean-max over time for one exercise key: the running highest clean BPM, one point per practice day. */
export function cleanMaxSeries(log: LogEntry[], exerciseId: string): { date: string; bpm: number }[] {
  const byDay = new Map<string, number>();
  for (const e of log) {
    if (e.exerciseId !== exerciseId || e.bestCleanBpm === null) continue;
    byDay.set(e.date, Math.max(byDay.get(e.date) ?? 0, e.bestCleanBpm));
  }
  let best = 0;
  return [...byDay.keys()].sort().map((date) => {
    best = Math.max(best, byDay.get(date) as number);
    return { date, bpm: best };
  });
}

/** Exercise keys that have at least one clean BPM logged, most recently practised first. */
export function exercisesWithHistory(log: LogEntry[]): string[] {
  const seen: string[] = [];
  for (let i = log.length - 1; i >= 0; i--) {
    const e = log[i];
    if (e.bestCleanBpm !== null && !seen.includes(e.exerciseId)) seen.push(e.exerciseId);
  }
  return seen;
}

export type CalendarCell = { date: string; minutes: number; level: 0 | 1 | 2 | 3; future: boolean };

/**
 * The last `weeks` weeks as a grid, one array per week (Monday first), ending with the current week.
 * Level: 0 none, 1 under 10 min, 2 streak-worthy (10+), 3 a long day (30+).
 */
export function calendarWeeks(dayMinutes: Record<string, number>, today: Date = new Date(), weeks = 8): CalendarCell[][] {
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7) - 7 * (weeks - 1)); // Monday of the first week
  const todayKey = dateKey(today);
  const out: CalendarCell[][] = [];
  for (let w = 0; w < weeks; w++) {
    const week: CalendarCell[] = [];
    for (let d = 0; d < 7; d++) {
      const date = dateKey(start);
      const minutes = dayMinutes[date] ?? 0;
      week.push({ date, minutes, level: minutes >= 30 ? 3 : minutes >= 10 ? 2 : minutes > 0 ? 1 : 0, future: date > todayKey });
      start.setDate(start.getDate() + 1);
    }
    out.push(week);
  }
  return out;
}

/** Monday of the week containing `d`, as a date key. */
export function weekStartKey(d: Date = new Date()): string {
  const m = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  m.setDate(m.getDate() - ((m.getDay() + 6) % 7));
  return dateKey(m);
}

/** Minutes logged this week (since Monday) per instrument; `instrumentOf` maps an exercise key to its instrument. */
export function weekMinutes(
  log: LogEntry[], instrumentOf: (exerciseId: string) => 'guitar' | 'pads' | undefined, today: Date = new Date(),
): { guitar: number; pads: number } {
  const from = weekStartKey(today);
  const out = { guitar: 0, pads: 0 };
  for (const e of log) {
    if (e.date < from) continue;
    const inst = instrumentOf(e.exerciseId);
    if (inst) out[inst] += e.minutes;
  }
  return { guitar: Math.round(out.guitar * 10) / 10, pads: Math.round(out.pads * 10) / 10 };
}

/** Exercise key `name@n` → base id. */
export const baseId = (key: string): string => key.split('@')[0];

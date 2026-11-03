import { addDays } from '../data/program';
import { testRows, type ProgramLog } from '../data/programLog';
import { dateKey, streak, type Profile } from './progress';

export type WeekSummary = {
  from: string; to: string; // Monday and Sunday
  minutes: { guitar: number; pads: number; total: number };
  practiceDays: number;
  pbs: { exerciseId: string; from: number | null; to: number }[]; // clean max set or beaten this week
  bursts: number; endurance: number;
  programDone: number;
  tests: { id: string; ct: number; delta: number | null }[]; // latest CT this week and the change from the one before
  streak: number;
};

const mondayOf = (d: Date): string => {
  const m = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  m.setDate(m.getDate() - ((m.getDay() + 6) % 7));
  return dateKey(m);
};

/** What happened in the Monday–Sunday week containing `day`. `instrumentOf` maps an exercise key to its instrument. */
export function weeklySummary(
  profile: Profile, programLog: ProgramLog, testIds: string[],
  instrumentOf: (exerciseId: string) => 'guitar' | 'pads' | undefined, day: Date = new Date(),
): WeekSummary {
  const from = mondayOf(day);
  const to = addDays(from, 6);
  const inWeek = (d: string) => d >= from && d <= to;

  const minutes = { guitar: 0, pads: 0, total: 0 };
  const days = new Set<string>();
  const bestBefore = new Map<string, number>();
  const bestWeek = new Map<string, number>();
  for (const e of profile.log) {
    if (inWeek(e.date)) {
      const inst = instrumentOf(e.exerciseId);
      if (inst) minutes[inst] += e.minutes;
      minutes.total += e.minutes;
      if (e.minutes > 0) days.add(e.date);
      if (e.bestCleanBpm !== null) bestWeek.set(e.exerciseId, Math.max(bestWeek.get(e.exerciseId) ?? 0, e.bestCleanBpm));
    } else if (e.date < from && e.bestCleanBpm !== null) {
      bestBefore.set(e.exerciseId, Math.max(bestBefore.get(e.exerciseId) ?? 0, e.bestCleanBpm));
    }
  }
  const round = (n: number) => Math.round(n * 10) / 10;
  const gain = (p: { from: number | null; to: number }) => (p.from === null ? -1 : p.to - p.from);
  const pbs = [...bestWeek]
    .filter(([id, bpm]) => bpm > (bestBefore.get(id) ?? 0))
    .map(([exerciseId, to2]) => ({ exerciseId, from: bestBefore.get(exerciseId) ?? null, to: to2 }))
    .sort((a, b) => gain(b) - gain(a) || a.exerciseId.localeCompare(b.exerciseId)); // beaten bests by size of gain, then first-ever ones

  const tests: WeekSummary['tests'] = [];
  for (const id of testIds) {
    const rows = testRows(programLog, id).filter((r) => r.ct !== null);
    const thisWeek = rows.filter((r) => inWeek(r.date)).at(-1);
    if (!thisWeek || thisWeek.ct === null) continue;
    const before = rows.filter((r) => r.date < from).at(-1);
    tests.push({ id, ct: thisWeek.ct, delta: before?.ct != null ? thisWeek.ct - before.ct : null });
  }

  return {
    from, to,
    minutes: { guitar: round(minutes.guitar), pads: round(minutes.pads), total: round(minutes.total) },
    practiceDays: days.size, pbs,
    bursts: profile.runs.filter((r) => r.type === 'burst' && inWeek(r.date)).length,
    endurance: profile.runs.filter((r) => r.type === 'endurance' && inWeek(r.date)).length,
    programDone: Object.values(programLog.sessions).filter((s) => s.done && inWeek(s.date)).length,
    tests, streak: streak(profile.dayMinutes, day),
  };
}

/** Plain text for pasting into a chat or a note. */
export function summaryText(s: WeekSummary, nameOf: (exerciseId: string) => string, testName: (id: string) => string): string {
  const lines = [
    `Practice week ${s.from} to ${s.to}`,
    `${s.minutes.total} min over ${s.practiceDays} day${s.practiceDays === 1 ? '' : 's'} (guitar ${s.minutes.guitar}, pads ${s.minutes.pads}). Streak: ${s.streak} day${s.streak === 1 ? '' : 's'}.`,
  ];
  if (s.pbs.length) lines.push('New clean max: ' + s.pbs.map((p) => `${nameOf(p.exerciseId)} ${p.from === null ? 'first' : p.from} → ${p.to}`).join('; ') + '.');
  if (s.tests.length) lines.push('Tests (CT): ' + s.tests.map((t) => `${testName(t.id)} ${t.ct}${t.delta === null ? '' : ` (${t.delta >= 0 ? '+' : '−'}${Math.abs(t.delta)})`}`).join('; ') + '.');
  if (s.bursts + s.endurance + s.programDone) lines.push(`Burst runs ${s.bursts}, endurance runs ${s.endurance}, program sessions done ${s.programDone}.`);
  return lines.join('\n');
}

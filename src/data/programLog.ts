import type { Shift } from './program';

/** One logged exercise or test. Unknown fields from an imported log are kept as they came. */
export type Entry = {
  ct?: number; // clean tempo (bpm)
  tension?: number; // 0–10
  clean3?: boolean; // 3 of 3 clean at the target
  note?: string;
  mc?: number; // motion ceiling (tests)
  video?: boolean; // tests
  auto?: boolean; // ct was filled in from the session's clean max and not edited since (not exported)
  [k: string]: unknown;
};
export type SessionLog = {
  date: string; programDate: string; week: number;
  entries: Record<string, Entry>; // "{blockKey}:{exId}:{focus}", "test:{Tn}", "{blockKey}:note"
  note?: string; done?: boolean; updatedAt: number;
};
export type ProgramLog = { shifts: Shift[]; sessions: Record<string, SessionLog> };

export const emptyProgramLog = (): ProgramLog => ({ shifts: [], sessions: {} });

export const exerciseKey = (blockKey: string, exId: string, focus?: string): string => `${blockKey}:${exId}:${focus ?? ''}`;
export const testKey = (testId: string): string => `test:${testId}`;
export const noteKey = (blockKey: string): string => `${blockKey}:note`;

/** Exercise id of an exercise entry key, or null for test and note keys. */
export function exIdOfKey(key: string): string | null {
  const p = key.split(':');
  return p.length >= 3 && p[0] !== 'test' ? p[1] : null;
}

// ---------- import / export: practice-log/v1 ----------

export const LOG_SCHEMA = 'practice-log/v1';

export type ImportedLog = { shifts: Shift[]; sessions: SessionLog[] };

const isObj = (x: unknown): x is Record<string, unknown> => !!x && typeof x === 'object' && !Array.isArray(x);

/** Parse a pasted or uploaded log. Throws a readable error for anything that isn't practice-log/v1. */
export function parsePracticeLog(text: string): ImportedLog {
  let raw: unknown;
  try { raw = JSON.parse(text); } catch { throw new Error('That is not valid JSON'); }
  if (!isObj(raw) || raw.schema !== LOG_SCHEMA) throw new Error(`Expected a ${LOG_SCHEMA} log`);
  if (!Array.isArray(raw.sessions)) throw new Error('The log has no sessions list');
  const sessions: SessionLog[] = [];
  for (const s of raw.sessions) {
    if (!isObj(s) || typeof s.date !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(s.date)) continue;
    sessions.push({
      date: s.date,
      programDate: typeof s.programDate === 'string' ? s.programDate : s.date,
      week: typeof s.week === 'number' ? s.week : 0,
      entries: isObj(s.entries) ? (s.entries as Record<string, Entry>) : {},
      ...(typeof s.note === 'string' ? { note: s.note } : {}),
      ...(typeof s.done === 'boolean' ? { done: s.done } : {}),
      updatedAt: typeof s.updatedAt === 'number' ? s.updatedAt : 0,
    });
  }
  const cfg = isObj(raw.config) && Array.isArray(raw.config.shifts) ? raw.config.shifts : [];
  const shifts = cfg.filter((x): x is Shift => isObj(x) && typeof x.from === 'string' && typeof x.days === 'number');
  return { shifts, sessions };
}

/** Merge by date; within a date by entry key, the newer session's `updatedAt` wins. Keys on only one side are kept. */
export function mergeLogs(base: ProgramLog, incoming: ImportedLog): ProgramLog {
  const sessions = { ...base.sessions };
  for (const inc of incoming.sessions) {
    const cur = sessions[inc.date];
    if (!cur) { sessions[inc.date] = inc; continue; }
    const incNewer = inc.updatedAt >= cur.updatedAt; // a tie goes to the import
    const [older, newer] = incNewer ? [cur, inc] : [inc, cur];
    sessions[inc.date] = {
      ...older, ...newer,
      entries: { ...older.entries, ...newer.entries },
      note: newer.note ?? older.note,
      done: newer.done ?? older.done,
      updatedAt: Math.max(cur.updatedAt, inc.updatedAt),
    };
  }
  const byFrom = new Map<string, Shift>();
  for (const s of [...base.shifts, ...incoming.shifts]) byFrom.set(s.from, s);
  return { shifts: [...byFrom.values()].sort((a, b) => a.from.localeCompare(b.from)), sessions };
}

/** The log in the same shape the artifact exports, so the two stay interchangeable. */
export function exportPracticeLog(log: ProgramLog, programId: string, now = Date.now()) {
  const sessions = Object.values(log.sessions)
    .sort((a, b) => a.date.localeCompare(b.date))
    .map((s) => ({
      ...s,
      entries: Object.fromEntries(Object.entries(s.entries).map(([k, e]) => {
        const { auto: _auto, ...rest } = e;
        return [k, rest];
      })),
    }));
  return { schema: LOG_SCHEMA, program: programId, exportedAt: new Date(now).toISOString(), config: { shifts: log.shifts }, sessions };
}

// ---------- reading the log ----------

/** Update (or create) one entry in a date's session log. */
export function setEntry(
  log: ProgramLog, date: string, meta: { programDate: string; week: number }, key: string, patch: Partial<Entry>, now = Date.now(),
): ProgramLog {
  const cur = log.sessions[date] ?? { date, ...meta, entries: {}, updatedAt: now };
  return {
    ...log,
    sessions: { ...log.sessions, [date]: { ...cur, ...meta, entries: { ...cur.entries, [key]: { ...cur.entries[key], ...patch } }, updatedAt: now } },
  };
}
export function setSessionFields(
  log: ProgramLog, date: string, meta: { programDate: string; week: number }, patch: { note?: string; done?: boolean }, now = Date.now(),
): ProgramLog {
  const cur = log.sessions[date] ?? { date, ...meta, entries: {}, updatedAt: now };
  return { ...log, sessions: { ...log.sessions, [date]: { ...cur, ...meta, ...patch, updatedAt: now } } };
}

/** The latest earlier clean tempo logged for an exercise (or a test, with `test:Tn` as `exId`). */
export function lastFor(log: ProgramLog, exId: string, beforeDate: string): { bpm: number; date: string } | null {
  let best: { bpm: number; date: string } | null = null;
  for (const s of Object.values(log.sessions)) {
    if (s.date >= beforeDate) continue;
    for (const [k, e] of Object.entries(s.entries)) {
      const match = exId.startsWith('test:') ? k === exId : exIdOfKey(k) === exId;
      if (!match || typeof e.ct !== 'number' || e.ct <= 0) continue;
      if (!best || s.date > best.date) best = { bpm: e.ct, date: s.date };
    }
  }
  return best;
}

// ---------- hints (shown, never enforced) ----------

export const TENSION_LIMIT = 5;
export const tensionTooHigh = (e: Entry | undefined): boolean => typeof e?.tension === 'number' && e.tension > TENSION_LIMIT;

/** 3 of 3 clean on two different days → ready for the next exercise. */
export function readyForNext(log: ProgramLog, exId: string): boolean {
  const days = new Set<string>();
  for (const s of Object.values(log.sessions)) {
    for (const [k, e] of Object.entries(s.entries)) if (exIdOfKey(k) === exId && e.clean3 === true) days.add(s.date);
  }
  return days.size >= 2;
}

// ---------- tests: CT / MC over time ----------

export type TestRow = { date: string; ct: number | null; mc: number | null; gap: number | null; pctFromStart: number | null };

/** Per test: every date it was recorded, the CT–MC gap, and CT's % change from the first (kickoff) CT. */
export function testRows(log: ProgramLog, testId: string): TestRow[] {
  const rows: TestRow[] = [];
  let first: number | null = null;
  for (const s of Object.values(log.sessions).sort((a, b) => a.date.localeCompare(b.date))) {
    const e = s.entries[testKey(testId)];
    if (!e) continue;
    const ct = typeof e.ct === 'number' && e.ct > 0 ? e.ct : null;
    const mc = typeof e.mc === 'number' && e.mc > 0 ? e.mc : null;
    if (ct === null && mc === null) continue;
    if (first === null && ct !== null) first = ct;
    rows.push({
      date: s.date, ct, mc,
      gap: ct !== null && mc !== null ? mc - ct : null,
      pctFromStart: ct !== null && first !== null ? Math.round(((ct - first) / first) * 1000) / 10 : null,
    });
  }
  return rows;
}

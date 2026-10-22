import { describe, expect, it } from 'vitest';
import {
  exIdOfKey, exerciseKey, exportPracticeLog, lastFor, mergeLogs, parsePracticeLog, readyForNext, setEntry, tensionTooHigh, testRows,
  type ProgramLog, type SessionLog,
} from './programLog';

const sess = (date: string, entries: SessionLog['entries'], updatedAt: number, extra: Partial<SessionLog> = {}): SessionLog =>
  ({ date, programDate: date, week: 1, entries, updatedAt, ...extra });
const log = (...s: SessionLog[]): ProgramLog => ({ shifts: [], sessions: Object.fromEntries(s.map((x) => [x.date, x])) });

describe('keys', () => {
  it('match the artifact: "{blockKey}:{exId}:{focus}"', () => {
    expect(exerciseKey('main', 'A2')).toBe('main:A2:');
    expect(exerciseKey('circuit', 'E1', 'ascending')).toBe('circuit:E1:ascending');
    expect(exIdOfKey('circuit:E1:ascending')).toBe('E1');
    expect(exIdOfKey('main:A2:')).toBe('A2');
    expect(exIdOfKey('test:T1')).toBeNull();
    expect(exIdOfKey('lab:note')).toBeNull();
  });
});

describe('importing practice-log/v1', () => {
  const sample = {
    schema: 'practice-log/v1', program: 'picking-8wk-2026', exportedAt: '2026-10-25T10:00:00Z',
    config: { shifts: [{ from: '2026-11-02', days: 7 }] },
    sessions: [
      { date: '2026-10-22', programDate: '2026-10-22', week: 1, updatedAt: 1000, note: 'felt ok', done: true,
        entries: { 'test:T1': { ct: 120, mc: 150, tension: 3, video: true, note: 'x', weird: { keep: 1 } } } },
      { date: 'bogus' },
    ],
  };
  it('keeps every field, including ones this app does not know', () => {
    const r = parsePracticeLog(JSON.stringify(sample));
    expect(r.sessions.length).toBe(1);
    expect(r.sessions[0].entries['test:T1']).toEqual(sample.sessions[0].entries?.['test:T1']);
    expect(r.sessions[0]).toMatchObject({ note: 'felt ok', done: true, week: 1, updatedAt: 1000 });
    expect(r.shifts).toEqual([{ from: '2026-11-02', days: 7 }]);
  });
  it('rejects the wrong schema and bad JSON with readable errors', () => {
    expect(() => parsePracticeLog('nope')).toThrow('not valid JSON');
    expect(() => parsePracticeLog('{"schema":"other/v1"}')).toThrow('practice-log/v1');
    expect(() => parsePracticeLog('{"schema":"practice-log/v1"}')).toThrow('no sessions');
  });
  it('an export re-imports without loss (round trip), minus the internal auto flag', () => {
    const base: ProgramLog = log(sess('2026-10-22', { 'main:A2:': { ct: 100, tension: 2, auto: true, clean3: true }, 'test:T1': { ct: 120, mc: 150 } }, 5, { note: 'n', done: true }));
    const out = exportPracticeLog(base, 'p', 0);
    expect(out.schema).toBe('practice-log/v1');
    const back = parsePracticeLog(JSON.stringify(out));
    expect(back.sessions[0].entries['main:A2:']).toEqual({ ct: 100, tension: 2, clean3: true });
    expect(back.sessions[0].entries['test:T1']).toEqual({ ct: 120, mc: 150 });
    expect(mergeLogs({ shifts: [], sessions: {} }, back).sessions['2026-10-22'].note).toBe('n');
  });
});

describe('mergeLogs', () => {
  it('adds new dates', () => {
    const m = mergeLogs(log(sess('2026-10-22', { a: { ct: 1 } }, 1)), { shifts: [], sessions: [sess('2026-10-23', { b: { ct: 2 } }, 1)] });
    expect(Object.keys(m.sessions)).toEqual(['2026-10-22', '2026-10-23']);
  });
  it('within a date, the newer updatedAt wins per entry key; keys on one side are kept', () => {
    const mine = sess('2026-10-22', { a: { ct: 100 }, onlyMine: { ct: 5 } }, 2000, { note: 'mine' });
    const theirs = sess('2026-10-22', { a: { ct: 120 }, onlyTheirs: { ct: 7 } }, 1000, { note: 'theirs' });
    const m = mergeLogs(log(mine), { shifts: [], sessions: [theirs] }).sessions['2026-10-22'];
    expect(m.entries.a.ct).toBe(100); // mine is newer
    expect(m.entries.onlyMine.ct).toBe(5);
    expect(m.entries.onlyTheirs.ct).toBe(7);
    expect(m.note).toBe('mine');
    expect(m.updatedAt).toBe(2000);
    const m2 = mergeLogs(log(theirs), { shifts: [], sessions: [mine] }).sessions['2026-10-22'];
    expect(m2.entries.a.ct).toBe(100); // import is newer
  });
  it('unions shifts by from-date', () => {
    const m = mergeLogs({ shifts: [{ from: '2026-11-02', days: 7 }], sessions: {} }, { shifts: [{ from: '2026-11-02', days: 7 }, { from: '2026-10-26', days: 7 }], sessions: [] });
    expect(m.shifts).toEqual([{ from: '2026-10-26', days: 7 }, { from: '2026-11-02', days: 7 }]);
  });
});

describe('lastFor / hints', () => {
  const l = log(
    sess('2026-10-23', { 'main:A2:': { ct: 90 }, 'secondary:A2:': { ct: 96, clean3: true } }, 1),
    sess('2026-10-24', { 'main:A2:': { ct: 100, clean3: true }, 'test:T2': { ct: 110 } }, 1),
    sess('2026-10-25', { 'main:A2:': { ct: 104 } }, 1),
  );
  it('latest earlier clean tempo for the same exercise id', () => {
    expect(lastFor(l, 'A2', '2026-10-25')).toEqual({ bpm: 100, date: '2026-10-24' });
    expect(lastFor(l, 'A2', '2026-10-23')).toBeNull();
    expect(lastFor(l, 'test:T2', '2026-12-01')).toEqual({ bpm: 110, date: '2026-10-24' });
    expect(lastFor(l, 'A3', '2026-12-01')).toBeNull();
  });
  it('next exercise only after 3/3 clean on two different days', () => {
    expect(readyForNext(l, 'A2')).toBe(true); // 23rd (secondary) + 24th
    expect(readyForNext(log(sess('2026-10-23', { 'main:A2:': { clean3: true }, 'secondary:A2:': { clean3: true } }, 1)), 'A2')).toBe(false); // same day twice
  });
  it('tension above 5 warns', () => {
    expect(tensionTooHigh({ tension: 6 })).toBe(true);
    expect(tensionTooHigh({ tension: 5 })).toBe(false);
    expect(tensionTooHigh(undefined)).toBe(false);
  });
});

describe('setEntry', () => {
  it('creates the session and stamps updatedAt', () => {
    const l = setEntry({ shifts: [], sessions: {} }, '2026-10-22', { programDate: '2026-10-22', week: 1 }, 'main:A2:', { ct: 90 }, 42);
    expect(l.sessions['2026-10-22']).toMatchObject({ updatedAt: 42, entries: { 'main:A2:': { ct: 90 } } });
    const l2 = setEntry(l, '2026-10-22', { programDate: '2026-10-22', week: 1 }, 'main:A2:', { tension: 3 }, 50);
    expect(l2.sessions['2026-10-22'].entries['main:A2:']).toEqual({ ct: 90, tension: 3 });
  });
});

describe('testRows', () => {
  const l = log(
    sess('2026-10-22', { 'test:T1': { ct: 120, mc: 150 } }, 1),
    sess('2026-10-31', { 'test:T1': { ct: 126, mc: 160 }, 'test:T2': { ct: 90 } }, 1),
    sess('2026-11-14', { 'test:T1': { ct: 138 } }, 1),
  );
  it('CT, MC, gap and % change from the kickoff CT', () => {
    expect(testRows(l, 'T1')).toEqual([
      { date: '2026-10-22', ct: 120, mc: 150, gap: 30, pctFromStart: 0 },
      { date: '2026-10-31', ct: 126, mc: 160, gap: 34, pctFromStart: 5 },
      { date: '2026-11-14', ct: 138, mc: null, gap: null, pctFromStart: 15 },
    ]);
    expect(testRows(l, 'T2').length).toBe(1);
    expect(testRows(l, 'T9')).toEqual([]);
  });
});

import { describe, expect, it } from 'vitest';
import { addDays, addRepeat, barStrokes, dayFor, hasRepeat, nextMonday, offersRepeat, programDate, removeRepeat, toHubExercise, type Program, type ProgramExercise } from './program';

const tiny = (over: Partial<Program['program']> = {}): Program => ({
  schema: 'practice-program/v1',
  program: { id: 'p', title: 'P', techniques: {}, week1Monday: '2026-10-19', kickoff: '2026-10-22', end: '2026-12-13', phases: [], rules: [], safety: [], ...over },
  exercises: {}, tests: {},
  sessions: [{ date: '2026-10-22', week: 1, phase: 1, dow: 3, type: 'kickoff', main: null, secondary: null, minutes: 30, blocks: [] },
    { date: '2026-12-13', week: 8, phase: 4, dow: 6, type: 'rest', main: null, secondary: null, minutes: 0, blocks: [] },
    { date: '2026-10-26', week: 2, phase: 1, dow: 0, type: 'weekday', main: 'a', secondary: 'b', minutes: 30, blocks: [] }],
});

describe('dates', () => {
  it('adds days across month ends and DST', () => {
    expect(addDays('2026-10-31', 1)).toBe('2026-11-01');
    expect(addDays('2026-11-01', 1)).toBe('2026-11-02'); // US DST ends Nov 1 2026
    expect(addDays('2026-12-13', -7)).toBe('2026-12-06');
    expect(nextMonday('2026-10-31')).toBe('2026-11-02'); // Saturday
    expect(nextMonday('2026-11-02')).toBe('2026-11-09'); // a Monday asks for the next one
    expect(nextMonday('2026-11-01')).toBe('2026-11-02'); // Sunday
  });
});

describe('programDate (shifts)', () => {
  it('no shifts: a date maps to itself', () => {
    expect(programDate('2026-12-13', [])).toBe('2026-12-13');
  });
  it('one shift from Nov 2: Nov 2 → Oct 26 and Dec 20 → Dec 13', () => {
    const shifts = [{ from: '2026-11-02', days: 7 }];
    expect(programDate('2026-11-02', shifts)).toBe('2026-10-26');
    expect(programDate('2026-12-20', shifts)).toBe('2026-12-13');
    expect(programDate('2026-11-01', shifts)).toBe('2026-11-01'); // before the shift
  });
  it('shifts stack', () => {
    const shifts = [{ from: '2026-11-02', days: 7 }, { from: '2026-11-16', days: 7 }];
    expect(programDate('2026-11-16', shifts)).toBe('2026-11-02');
    expect(programDate('2026-12-27', shifts)).toBe('2026-12-13');
  });
  it('"Repeat this week" adds a shift from the next Monday, once', () => {
    const a = addRepeat([], '2026-10-31');
    expect(a).toEqual([{ from: '2026-11-02', days: 7 }]);
    expect(addRepeat(a, '2026-10-31')).toBe(a);
    expect(hasRepeat(a, '2026-10-30')).toBe(true);
    expect(removeRepeat(a, '2026-10-31')).toEqual([]);
  });
});

describe('dayFor', () => {
  const p = tiny();
  it('before the kickoff, during, after the end', () => {
    expect(dayFor('2026-10-20', p, [])).toEqual({ state: 'before', programDate: '2026-10-20', daysUntil: 2 });
    expect(dayFor('2026-10-22', p, [])).toMatchObject({ state: 'during', session: { date: '2026-10-22' } });
    expect(dayFor('2026-12-14', p, [])).toEqual({ state: 'after', programDate: '2026-12-14' });
  });
  it('a repeated week keeps the program running past its nominal end', () => {
    const shifts = [{ from: '2026-11-02', days: 7 }];
    expect(dayFor('2026-12-20', p, shifts)).toMatchObject({ state: 'during', programDate: '2026-12-13' });
    expect(dayFor('2026-11-02', p, shifts)).toMatchObject({ programDate: '2026-10-26', session: { week: 2 } });
  });
  it('a day with no row is during with no session', () => {
    expect(dayFor('2026-10-23', p, [])).toMatchObject({ state: 'during', session: null });
  });
  it('Saturday reviews of weeks 2, 4, 6 offer the repeat button', () => {
    const s = (dow: number, week: number) => ({ ...p.sessions[0], dow, week });
    expect([2, 4, 6].map((w) => offersRepeat(s(5, w)))).toEqual([true, true, true]);
    expect(offersRepeat(s(5, 3))).toBe(false);
    expect(offersRepeat(s(4, 2))).toBe(false);
  });
});

const pe = (id: string, sub: number, strokes: string[][], scorable = true): ProgramExercise => ({
  id, name: id, technique: 'alternate', purpose: '', how: '', tempo: '', cue: '', next: '', source: '', tabs: [], hub: { subdivision: sub, scorable, strokes },
});

describe('mapping to a hub Exercise', () => {
  it('cycles one variant to fill a 4-beat bar (A2: sextuplets D U)', () => {
    const ex = toHubExercise(pe('A2', 6, [['D', 'U', 'D', 'U', 'D', 'U']]));
    expect(ex.id).toBe('prog-A2');
    expect(ex.subdivision).toBe(6);
    expect(ex.beatsPerBar).toBe(4);
    expect(ex.pattern.length).toBe(24);
    expect(ex.pattern.slice(0, 6).map((s) => s.lane)).toEqual(['down', 'up', 'down', 'up', 'down', 'up']);
    expect(ex.ramp).toMatchObject({ stepBpm: 4, cleanRepsToAdvance: 4 });
  });
  it('joins variants when their total divides the bar (C0: alternate bar + economy bar)', () => {
    const c0 = [['D', 'U', 'D', 'U', 'D', 'U'], ['D', 'U', 'D', 'D', 'U', 'D']];
    const { strokes } = barStrokes({ subdivision: 6, scorable: true, strokes: c0 });
    expect(strokes.join('')).toBe('DUDUDUDUDDUD'.repeat(2));
  });
  it('economy keeps same-direction strokes back to back, from the slot lanes', () => {
    const ex = toHubExercise(pe('E1', 6, [['D', 'U', 'D', 'D', 'U', 'D'], ['U', 'D', 'U', 'U', 'D', 'U']]));
    expect(ex.pattern.slice(0, 6).map((s) => s.lane)).toEqual(['down', 'up', 'down', 'down', 'up', 'down']);
    expect(ex.pattern.slice(6, 12).map((s) => s.lane)).toEqual(['up', 'down', 'up', 'up', 'down', 'up']);
  });
  it('uses the first variant when the total does not divide the bar (A3: 3 × 6)', () => {
    const v = ['D', 'U', 'D', 'U', 'D', 'U'];
    expect(barStrokes({ subdivision: 6, scorable: true, strokes: [v, v, v] }).strokes.length).toBe(24);
  });
  it('widens the bar to hold whole cycles (K2: 24 sixteenths = 6 beats; A5: 9 sextuplets × 2 = 18 → 6 beats)', () => {
    expect(barStrokes({ subdivision: 4, scorable: true, strokes: [Array(24).fill('D')] }).beatsPerBar).toBe(6);
    expect(barStrokes({ subdivision: 6, scorable: true, strokes: [Array(9).fill('D'), Array(9).fill('U')] }).beatsPerBar).toBe(6);
    expect(barStrokes({ subdivision: 4, scorable: true, strokes: [Array(16).fill('D')] }).beatsPerBar).toBe(4);
  });
  it('sweeps and legato are unscored: click and timer only, no expected hits', () => {
    const ex = toHubExercise(pe('S0', 3, [['D', 'D', 'D', 'U', 'U', 'U']], false));
    expect(ex.unscored).toBe(true);
    expect(ex.pattern.every((s) => !s.play)).toBe(true);
    expect(ex.pattern.length).toBe(12);
  });
  it('legato notes are ghosts, not expected onsets', () => {
    const ex = toHubExercise(pe('X', 4, [['D', 'L', 'U', 'L']]));
    expect(ex.pattern.slice(0, 4).map((s) => !!s.ghost)).toEqual([false, true, false, true]);
  });
});

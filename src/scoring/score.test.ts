import { describe, expect, it } from 'vitest';
import { accentStats, driftPer10s, rampStep, scoreRep, slotTimesFromBeats, updateCombo } from './score';
import type { Slot } from '../data/types';

const P: Slot = { play: true };
const G: Slot = { play: false, ghost: true };
const times = (n: number) => Array.from({ length: n }, (_, i) => i * 0.125); // 120 BPM 16ths

describe('offset sign', () => {
  it('+ is late, - is early', () => {
    const r = scoreRep([P, P], [0, 0.125], [0.01, 0.125 - 0.007], 15);
    expect(r.slots[0].offsetMs).toBe(10);
    expect(r.slots[1].offsetMs).toBe(-7);
    expect(r.meanOffsetMs).toBe(1.5);
  });
});

describe('window edges', () => {
  it('±window is inside, beyond is outside but still matched', () => {
    const r = scoreRep([P, P, P], times(3), [0.015, 0.125 - 0.015, 0.25 + 0.016], 15);
    expect(r.slots[0].inWindow).toBe(true);
    expect(r.slots[1].inWindow).toBe(true);
    expect(r.slots[2].inWindow).toBe(false);
    expect(r.slots[2].hit).toBe(true);
    expect(r.extras).toBe(0);
    expect(r.clean).toBe(false);
  });
  it('a hit past max(2×window, 60 ms) is an extra, not a match', () => {
    const edge = scoreRep([P], [0], [0.06], 15);
    expect(edge.slots[0].hit).toBe(true); // 60 ms is the limit for window 15
    const out = scoreRep([P], [0], [0.0601], 15);
    expect(out.slots[0].hit).toBe(false);
    expect(out.extras).toBe(1);
    const wide = scoreRep([P], [0], [0.07], 40); // limit 80 ms
    expect(wide.slots[0].hit).toBe(true);
  });
});

describe('extras', () => {
  it('a double hit on one slot makes the second an extra', () => {
    const r = scoreRep([P, P], times(2), [0, 0.01, 0.125], 15);
    expect(r.extras).toBe(1);
    expect(r.slots[0].offsetMs).toBe(0);
    expect(r.clean).toBe(false);
  });
});

describe('ghost slots', () => {
  it('expect no hit; missing one is fine, hitting one is an extra', () => {
    const pattern = [P, G, P, P];
    const ok = scoreRep(pattern, times(4), [0, 0.25, 0.375], 15);
    expect(ok.expectedCount).toBe(3);
    expect(ok.clean).toBe(true);
    const bad = scoreRep(pattern, times(4), [0, 0.125, 0.25, 0.375], 15);
    expect(bad.extras).toBe(1);
    expect(bad.clean).toBe(false);
  });
});

describe('clean rep', () => {
  it('needs every played slot in window and no extras', () => {
    const pattern = [P, P, P, P];
    const t = times(4);
    expect(scoreRep(pattern, t, [0, 0.125, 0.25, 0.375], 15).clean).toBe(true);
    expect(scoreRep(pattern, t, [0, 0.125, 0.25], 15).clean).toBe(false); // miss
    expect(scoreRep(pattern, t, [0, 0.125, 0.25, 0.375, 0.5], 15).clean).toBe(false); // extra
    expect(scoreRep(pattern, t, [], 15).clean).toBe(false);
  });
});

describe('lane enforcement', () => {
  const R: Slot = { play: true, lane: 'R' };
  const L: Slot = { play: true, lane: 'L' };
  it('wrong hand at the right time is out of window and not clean (when enforced)', () => {
    const hits = [{ time: 0, lane: 'R' as const, velocity: 0.8 }, { time: 0.125, lane: 'R' as const, velocity: 0.7 }];
    const on = scoreRep([R, L], times(2), hits, 15, { enforceLanes: true });
    expect(on.slots[1].wrongLane).toBe(true);
    expect(on.slots[1].inWindow).toBe(false);
    expect(on.slots[1].hitLane).toBe('R');
    expect(on.slots[1].velocity).toBe(0.7);
    expect(on.clean).toBe(false);
    expect(on.sequence).toEqual([true, false]);
    const off = scoreRep([R, L], times(2), hits, 15);
    expect(off.clean).toBe(true);
  });
});

describe('combo', () => {
  it('counts on-time hits and resets on miss, out-of-window hit or extra', () => {
    const r = updateCombo(0, 0, [true, true, true, false, true, true]);
    expect(r).toEqual({ combo: 2, best: 3 });
    expect(updateCombo(5, 5, [true, false]).combo).toBe(0);
    expect(updateCombo(5, 5, [true, true]).best).toBe(7);
  });
  it('sequence from scoreRep is time ordered and includes extras', () => {
    const r = scoreRep([P, P], times(2), [0.2, 0, 0.125, 0.5], 15);
    // 0 ok, 0.125 ok, 0.2 matches nothing free (slot 1 taken) → extra, 0.5 → extra
    expect(r.sequence).toEqual([true, true, false, false]);
    expect(updateCombo(0, 0, r.sequence)).toEqual({ combo: 0, best: 2 });
  });
});

describe('ramp advance', () => {
  const ramp = { stepBpm: 4, cleanRepsToAdvance: 3 };
  it('advances after N clean reps in a row and restarts the run', () => {
    let s = { cleanStreak: 0, advance: false };
    s = rampStep(s.cleanStreak, true, ramp);
    s = rampStep(s.cleanStreak, true, ramp);
    expect(s).toEqual({ cleanStreak: 2, advance: false });
    s = rampStep(s.cleanStreak, true, ramp);
    expect(s).toEqual({ cleanStreak: 0, advance: true });
  });
  it('an unclean rep resets the run', () => {
    expect(rampStep(2, false, ramp)).toEqual({ cleanStreak: 0, advance: false });
  });
});

describe('slotTimesFromBeats', () => {
  it('derives slot times from per-beat tempo', () => {
    const t = slotTimesFromBeats([{ time: 1, bpm: 120 }, { time: 1.5, bpm: 60 }], 2);
    expect(t).toEqual([1, 1.25, 1.5, 2]);
  });
});

describe('driftPer10s', () => {
  it('slope of offset over time in ms per 10 s', () => {
    const pts = Array.from({ length: 20 }, (_, i) => ({ t: i * 3, offsetMs: 2 + i * 1.5 })); // +0.5 ms/s
    expect(driftPer10s(pts)).toBe(5);
    expect(driftPer10s(pts.map((p) => ({ ...p, offsetMs: 7 })))).toBe(0);
    expect(driftPer10s(pts.map((p) => ({ ...p, offsetMs: -p.t * 0.2 })))).toBe(-2);
  });
  it('null with too few points or no time spread', () => {
    expect(driftPer10s([{ t: 0, offsetMs: 1 }])).toBeNull();
    expect(driftPer10s([1, 2, 3, 4].map((o) => ({ t: 5, offsetMs: o })))).toBeNull();
  });
});

describe('accentStats', () => {
  const A: Slot = { play: true, accent: true };
  const hitsAt = (vels: number[]) => vels.map((v, i) => ({ time: i * 0.125, velocity: v }));
  it('accent % = accent peak / mean peak of the other strums − 1; lands at +15%', () => {
    const pattern = [P, A, P, A];
    const rep = scoreRep(pattern, times(4), hitsAt([0.5, 0.6, 0.5, 0.57]), 15);
    const [a1, a2] = accentStats(pattern, rep.slots);
    expect(a1.index).toBe(1);
    expect(a1.pct).toBeCloseTo(0.2, 9);
    expect(a1.landed).toBe(true);
    expect(a2.pct).toBeCloseTo(0.14, 9);
    expect(a2.landed).toBe(false);
  });
  it('exactly +15% lands; ghosts and rests are not part of the mean', () => {
    const pattern = [P, G, A];
    const rep = scoreRep(pattern, times(3), [{ time: 0, velocity: 0.4 }, { time: 0.25, velocity: 0.46 }], 15);
    expect(accentStats(pattern, rep.slots)[0].landed).toBe(true);
  });
  it('a missed accent, or no velocity data, is not landed', () => {
    const pattern = [P, A];
    expect(accentStats(pattern, scoreRep(pattern, times(2), [{ time: 0, velocity: 0.5 }], 15).slots)[0]).toEqual({ index: 1, pct: null, landed: false });
    expect(accentStats(pattern, scoreRep(pattern, times(2), [0, 0.125], 15).slots)[0].pct).toBeNull();
  });
});

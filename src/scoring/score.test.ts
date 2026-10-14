import { describe, expect, it } from 'vitest';
import { rampStep, scoreRep, slotTimesFromBeats, updateCombo } from './score';
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

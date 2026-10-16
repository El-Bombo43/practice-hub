import { describe, expect, it } from 'vitest';
import { allowedSubdivisions, exerciseById, withSubdivision } from './exercises';

describe('re-counting exercises', () => {
  it('offers only counts whose cycle fits the bar', () => {
    expect(allowedSubdivisions(exerciseById('tremolo'))).toEqual([2, 3, 4, 6]);
    expect(allowedSubdivisions(exerciseById('pads-paradiddle'))).toEqual([2, 4, 6]);
    expect(allowedSubdivisions(exerciseById('pads-paradiddle-diddle'))).toEqual([3, 6]);
    expect(allowedSubdivisions(exerciseById('strum-8ths'))).toEqual([2]);
    expect(allowedSubdivisions(exerciseById('gallop'))).toEqual([4]);
  });
  it('rebuilds the pattern, keeps alternation, and keys PB/BPM separately', () => {
    const base = exerciseById('tremolo');
    const t = withSubdivision(base, 3);
    expect(t.pattern.length).toBe(12);
    expect(t.pattern.map((s) => s.lane).join()).toBe('down,up,down,up,down,up,down,up,down,up,down,up');
    expect(t.id).toBe('tremolo@3');
    expect(t.startBpm).toBe(107); // 80 × 4/3
    expect(withSubdivision(base, 4)).toBe(base);
    expect(withSubdivision(exerciseById('gallop'), 2).id).toBe('gallop');
  });
  it('keeps sticking continuous across the bar', () => {
    const p = withSubdivision(exerciseById('pads-paradiddle'), 6).pattern.map((s) => s.lane).join('');
    expect(p).toBe('RLRRLRLL'.repeat(3));
  });
});

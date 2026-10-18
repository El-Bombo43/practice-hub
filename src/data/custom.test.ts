import { describe, expect, it } from 'vitest';
import { blankCells, cycleCell, nextCell, parseCustom, serialize, toExercise, type CustomPattern } from './custom';

describe('slot cycling', () => {
  it('rest → strike → accent → ghost → rest', () => {
    expect(['-', 'x', 'X', 'g'].map(nextCell)).toEqual(['x', 'X', 'g', '-']);
    let c = blankCells(4);
    expect(c).toBe('----------------');
    c = cycleCell(c, 3);
    expect(c[3]).toBe('x');
    c = cycleCell(cycleCell(c, 3), 3);
    expect(c[3]).toBe('g');
    expect(cycleCell(c, 3)[3]).toBe('-');
    expect(cycleCell(c, 99)).toBe(c);
  });
});

describe('serialization', () => {
  const p: CustomPattern = { id: 'custom-a', name: 'Folk 8ths', subdivision: 2, cells: 'xgxxgxxx' };
  it('round-trips', () => {
    expect(parseCustom(serialize([p]))).toEqual([p]);
    expect(parseCustom([p])).toEqual([p]);
  });
  it('drops malformed entries instead of throwing', () => {
    const bad = [
      { ...p, cells: 'xgxx' }, // wrong length for 8ths
      { ...p, cells: '--------' }, // nothing to play
      { ...p, subdivision: 3 },
      { ...p, cells: 'xgxxgxxz' },
      { id: 1 }, null, 'nope',
    ];
    expect(parseCustom([...bad, p])).toEqual([p]);
    expect(parseCustom('not json')).toEqual([]);
    expect(parseCustom(undefined)).toEqual([]);
  });
});

describe('toExercise', () => {
  it('builds a strum exercise with parity directions, accents and ghosts', () => {
    const ex = toExercise({ id: 'custom-b', name: 'Test', subdivision: 4, cells: 'xgxxXxgxxgxxXxgx' });
    expect(ex.kind).toBe('strum');
    expect(ex.instrument).toBe('guitar');
    expect(ex.pattern.length).toBe(16);
    expect(ex.pattern.map((s) => s.lane).slice(0, 4)).toEqual(['down', 'up', 'down', 'up']);
    expect(ex.pattern[4]).toEqual({ play: true, lane: 'down', accent: true });
    expect(ex.pattern[1]).toEqual({ play: false, lane: 'up', ghost: true });
  });
  it('rests are neither played nor ghosts', () => {
    const s = toExercise({ id: 'c', name: 'r', subdivision: 2, cells: 'x-x-x-x-' }).pattern[1];
    expect(s.play).toBe(false);
    expect(s.ghost).toBeUndefined();
  });
});

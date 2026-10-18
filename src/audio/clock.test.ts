import { describe, expect, it } from 'vitest';
import { mad, median, perfToHeard } from './clock';

describe('perfToHeard', () => {
  it('maps performance.now ms onto the heard audio clock', () => {
    const offset = 5000 / 1000 - 10; // performance 5000 ms ↔ audio 10 s
    expect(perfToHeard(5000, offset)).toBe(10);
    expect(perfToHeard(5250, offset)).toBeCloseTo(10.25, 9);
    expect(perfToHeard(4900, offset)).toBeCloseTo(9.9, 9);
  });
});

describe('median / mad', () => {
  it('median ignores outliers that a mean would follow', () => {
    const readings = [0.2, 0.2, 0.21, 0.19, 0.2, 0.9, 0.2, 0.2]; // one stale reading
    expect(median(readings)).toBeCloseTo(0.2, 9);
    expect(readings.reduce((a, b) => a + b, 0) / readings.length).toBeGreaterThan(0.28);
  });
  it('mad is a robust spread', () => {
    expect(mad([1, 1, 1, 1, 50])).toBe(0);
    expect(mad([0, 10, 20])).toBe(10);
  });
});

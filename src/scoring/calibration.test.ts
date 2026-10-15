import { describe, expect, it } from 'vitest';
import { calibrationFromHits, median } from './calibration';

const clicks = Array.from({ length: 16 }, (_, i) => 1 + i * (60 / 90));

describe('median', () => {
  it('handles odd, even and empty', () => {
    expect(median([3, 1, 2])).toBe(2);
    expect(median([4, 1, 2, 3])).toBe(2.5);
    expect(median([])).toBeNull();
  });
});

describe('calibrationFromHits', () => {
  it('median offset, + = late', () => {
    const hits = clicks.map((c, i) => c + (30 + (i % 3) - 1) / 1000); // 29, 30, 31 ms late
    const r = calibrationFromHits(clicks, hits);
    expect(r.medianMs).toBe(30);
    expect(r.used).toBe(16);
    expect(r.ignored).toBe(0);
  });
  it('ignores hits more than 150 ms from any click, so strays do not skew the median', () => {
    const good = clicks.map((c) => c - 0.02);
    const stray = [clicks[3] + 0.3, clicks[5] + 0.2, clicks[7] + 0.151];
    const r = calibrationFromHits(clicks, [...good, ...stray]);
    expect(r.medianMs).toBe(-20);
    expect(r.ignored).toBe(3);
    expect(r.used).toBe(16);
  });
  it('is robust to a few wild in-range hits (median, not mean)', () => {
    const hits = [...clicks.map((c) => c + 0.01), clicks[0] + 0.14, clicks[1] + 0.14];
    expect(calibrationFromHits(clicks, hits).medianMs).toBe(10);
  });
  it('returns null with no usable hits', () => {
    expect(calibrationFromHits(clicks, []).medianMs).toBeNull();
    expect(calibrationFromHits(clicks, [clicks[0] + 0.4]).medianMs).toBeNull();
  });
});

import { describe, expect, it } from 'vitest';
import { drumHits, tapTempo } from './sounds';

describe('drum backing', () => {
  it('rock: kick 1 & 3, snare 2 & 4, hat on every tick', () => {
    expect([0, 1, 2, 3].map((b) => drumHits('rock', b, 0, 4))).toEqual([['hat', 'kick'], ['hat', 'snare'], ['hat', 'kick'], ['hat', 'snare']]);
    expect(drumHits('rock', 1, 2, 4)).toEqual(['hat']); // an off-beat tick is just the hat
  });
  it('half-time: kick on 1, snare on 3 only', () => {
    expect([0, 1, 2, 3].map((b) => drumHits('halftime', b, 0, 4))).toEqual([['hat', 'kick'], ['hat'], ['hat', 'snare'], ['hat']]);
  });
  it('odd bar lengths still get a kick and a snare', () => {
    expect(drumHits('rock', 4, 0, 5)).toContain('kick');
    expect(drumHits('halftime', 2, 0, 5)).toContain('snare');
  });
});

describe('tapTempo', () => {
  it('needs three taps', () => {
    expect(tapTempo([0])).toBeNull();
    expect(tapTempo([0, 500])).toBeNull();
  });
  it('uses the median gap, so one sloppy tap does not move it', () => {
    expect(tapTempo([0, 500, 1000, 1500])).toBe(120);
    expect(tapTempo([0, 500, 1000, 1700, 2200, 2700])).toBe(120);
  });
  it('a pause over 2 s starts a new count', () => {
    expect(tapTempo([0, 500, 1000, 1500, 9000, 9500])).toBeNull();
    expect(tapTempo([0, 500, 1000, 1500, 9000, 9600, 10200])).toBe(100);
  });
  it('clamps to 30–300 BPM', () => {
    expect(tapTempo([0, 100, 200, 300])).toBe(300);
    expect(tapTempo([0, 1900, 3800, 5700])).toBe(32);
  });
});

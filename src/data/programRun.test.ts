import { describe, expect, it } from 'vitest';
import { blockAt, chunkAt, fmtClock, itemIndex, testIdFor, totalSeconds } from './programRun';
import type { ProgramBlock, ProgramSession } from './program';

const b = (key: string, start: number, end: number, extra: Partial<ProgramBlock> = {}): ProgramBlock =>
  ({ key, start, end, kind: 'main', title: key, text: '', log: 'none', ...extra });
const session = { blocks: [b('warmup', 0, 3), b('main', 3, 11, { items: [{ ex: 'S0' }, { ex: 'S1' }], chunks: [{ i: 0, min: 2 }, { i: 1, min: 2 }, { i: 0, min: 2 }, { i: 1, min: 2 }] }), b('wrap', 11, 14)] } as ProgramSession;

describe('program run helpers', () => {
  it('blockAt follows the minute stamps and stays on the last block when time is up', () => {
    expect([0, 179, 180, 659, 660, 839, 840, 99999].map((s) => blockAt(session, s))).toEqual([0, 0, 1, 1, 2, 2, 2, 2]);
    expect(totalSeconds(session)).toBe(840);
  });
  it('chunkAt walks the A/B chunks from the block start', () => {
    const main = session.blocks[1];
    expect([180, 299, 300, 419, 420, 539, 540, 659, 9999].map((s) => chunkAt(main, s))).toEqual([0, 0, 1, 1, 2, 2, 3, 3, 3]);
    expect(chunkAt(session.blocks[0], 10)).toBeNull();
  });
  it('itemIndex uses the chunk item, or the pick when there are no chunks', () => {
    const main = session.blocks[1];
    expect(itemIndex(main, 200, 0)).toBe(0);
    expect(itemIndex(main, 320, 0)).toBe(1);
    expect(itemIndex(main, 450, 0)).toBe(0);
    const retest = b('retest', 0, 15, { items: [{ ex: 'A1' }, { ex: 'A2' }, { ex: 'A3' }], tests: ['T1', 'T2', 'T3'] });
    expect(itemIndex(retest, 10, 2)).toBe(2);
    expect(itemIndex(retest, 10, 9)).toBe(2); // clamped
  });
  it('test ids come from `test` or `tests[i]`', () => {
    expect(testIdFor(b('t', 0, 4, { test: 'T1' }), 0)).toBe('T1');
    expect(testIdFor(b('r', 0, 4, { tests: ['T1', 'T4'] }), 1)).toBe('T4');
    expect(testIdFor(b('m', 0, 4), 0)).toBeNull();
  });
  it('formats a clock', () => {
    expect(fmtClock(0)).toBe('0:00');
    expect(fmtClock(605)).toBe('10:05');
  });
});

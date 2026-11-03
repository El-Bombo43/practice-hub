import type { ProgramBlock, ProgramSession } from './program';

/** Which block is active at `elapsedSec` (the last one once the time is up). Blocks are contiguous minutes. */
export function blockAt(session: ProgramSession, elapsedSec: number): number {
  const bs = session.blocks;
  for (let i = 0; i < bs.length; i++) if (elapsedSec < bs[i].end * 60) return i;
  return Math.max(0, bs.length - 1);
}

/** Which A/B chunk of a block is active (index into `chunks`), or null when the block has none. */
export function chunkAt(block: ProgramBlock, elapsedSec: number): number | null {
  if (!block.chunks?.length) return null;
  let t = block.start * 60;
  for (let i = 0; i < block.chunks.length; i++) {
    t += block.chunks[i].min * 60;
    if (elapsedSec < t) return i;
  }
  return block.chunks.length - 1;
}

/**
 * Test blocks step through their tests on the clock. Each test gets an equal share of the block: the first
 * 60% is the clean-tempo attempt with the click, the rest is the motion ceiling with no click.
 */
export const CT_SHARE = 0.6;
export type TestPhase = { idx: number; phase: 'ct' | 'mc'; phaseSecLeft: number };
export function testPhaseAt(block: ProgramBlock, elapsedSec: number): TestPhase | null {
  if (block.kind !== 'test' || block.log !== 'test' || block.chunks?.length) return null;
  const n = Math.max(1, block.tests?.length ?? 1);
  const slot = ((block.end - block.start) * 60) / n;
  if (slot <= 0) return null;
  const rel = Math.max(0, elapsedSec - block.start * 60);
  const idx = Math.min(n - 1, Math.floor(rel / slot));
  const within = Math.min(slot, rel - idx * slot);
  const ctEnd = slot * CT_SHARE;
  return within < ctEnd
    ? { idx, phase: 'ct', phaseSecLeft: ctEnd - within }
    : { idx, phase: 'mc', phaseSecLeft: Math.max(0, slot - within) };
}

/** The active item index: the test or chunk's item when the clock decides, otherwise the player's pick. */
export function itemIndex(block: ProgramBlock, elapsedSec: number, picked: number): number {
  const tp = testPhaseAt(block, elapsedSec);
  if (tp && (block.tests?.length ?? 1) > 1) return Math.min(tp.idx, Math.max(0, (block.items?.length ?? 1) - 1));
  const c = chunkAt(block, elapsedSec);
  return c === null ? Math.min(picked, Math.max(0, (block.items?.length ?? 1) - 1)) : block.chunks![c].i;
}

// ---------- resume after a reload ----------

export type Resume = { date: string; elapsed: number; savedAt: number };
const RESUME_MAX_AGE_MS = 12 * 3600 * 1000;

/** Where to pick a session back up: the saved position for the same date, if recent and not finished. */
export function resumeElapsed(saved: Resume | null, viewDate: string, totalSec: number, nowMs: number): number {
  if (!saved || saved.date !== viewDate) return 0;
  if (saved.elapsed <= 0 || saved.elapsed >= totalSec || nowMs - saved.savedAt > RESUME_MAX_AGE_MS) return 0;
  return saved.elapsed;
}

/** Test id for an item of a test block (`test`, or `tests[i]`). */
export const testIdFor = (block: ProgramBlock, itemIdx: number): string | null =>
  block.test ?? block.tests?.[itemIdx] ?? null;

export const totalSeconds = (session: ProgramSession): number => (session.blocks.at(-1)?.end ?? 0) * 60;
export const fmtClock = (sec: number): string => `${Math.floor(sec / 60)}:${String(Math.floor(sec % 60)).padStart(2, '0')}`;

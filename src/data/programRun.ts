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

/** The active item index: the chunk's item when there are chunks, otherwise the player's pick. */
export function itemIndex(block: ProgramBlock, elapsedSec: number, picked: number): number {
  const c = chunkAt(block, elapsedSec);
  return c === null ? Math.min(picked, Math.max(0, (block.items?.length ?? 1) - 1)) : block.chunks![c].i;
}

/** Test id for an item of a test block (`test`, or `tests[i]`). */
export const testIdFor = (block: ProgramBlock, itemIdx: number): string | null =>
  block.test ?? block.tests?.[itemIdx] ?? null;

export const totalSeconds = (session: ProgramSession): number => (session.blocks.at(-1)?.end ?? 0) * 60;
export const fmtClock = (sec: number): string => `${Math.floor(sec / 60)}:${String(Math.floor(sec % 60)).padStart(2, '0')}`;

import { fromString } from './exercises';
import type { Exercise } from './types';

/** A pattern-builder strum: one 4/4 bar in 8ths or 16ths. Cells: '-' rest, 'x' strike, 'X' accent, 'g' ghost. */
export type CustomPattern = { id: string; name: string; subdivision: 2 | 4; cells: string };

const CYCLE = ['-', 'x', 'X', 'g'];
const BEATS = 4;

/** Click order: rest → strike → accent → ghost → rest. */
export const nextCell = (c: string): string => CYCLE[(CYCLE.indexOf(c) + 1) % CYCLE.length] ?? '-';
export const blankCells = (sub: 2 | 4): string => '-'.repeat(BEATS * sub);
export const cycleCell = (cells: string, i: number): string =>
  i < 0 || i >= cells.length ? cells : cells.slice(0, i) + nextCell(cells[i]) + cells.slice(i + 1);
export const hasStrike = (cells: string): boolean => /[xX]/.test(cells);

export const newCustomId = (): string => `custom-${Date.now().toString(36)}${Math.floor(Math.random() * 1296).toString(36)}`;

export function serialize(list: CustomPattern[]): string {
  return JSON.stringify(list);
}

/** Parse saved patterns, dropping anything malformed (an old or hand-edited backup must not crash the app). */
export function parseCustom(raw: unknown): CustomPattern[] {
  let list: unknown = raw;
  if (typeof raw === 'string') {
    try { list = JSON.parse(raw); } catch { return []; }
  }
  if (!Array.isArray(list)) return [];
  return list.filter((c): c is CustomPattern =>
    !!c && typeof c.id === 'string' && typeof c.name === 'string' && (c.subdivision === 2 || c.subdivision === 4)
    && typeof c.cells === 'string' && /^[-xXg]+$/.test(c.cells) && c.cells.length === BEATS * c.subdivision && hasStrike(c.cells));
}

/** Strum direction comes from slot parity: odd slots (1-based) are downstrokes, even are upstrokes. */
export function toExercise(cp: CustomPattern): Exercise {
  return {
    id: cp.id, name: cp.name, instrument: 'guitar', kind: 'strum', beatsPerBar: BEATS, subdivision: cp.subdivision,
    pattern: fromString(cp.cells, (i) => (i % 2 === 0 ? 'down' : 'up')),
    startBpm: cp.subdivision === 2 ? 80 : 70, ramp: { stepBpm: 4, cleanRepsToAdvance: 3 }, windowMs: 15, minutes: 4,
  };
}

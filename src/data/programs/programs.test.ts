import { describe, expect, it } from 'vitest';
import { dayFor, hasClick, toHubExercise } from '../program';
import { blockAt, totalSeconds } from '../programRun';
import { PROGRAMS } from './index';

// Validates every bundled program file. With no file present there is nothing to check (the app runs as the plain trainer).
describe.skipIf(PROGRAMS.length === 0)('bundled programs', () => {
  for (const p of PROGRAMS) {
    describe(p.program.id, () => {
      it('every exercise maps to a well-formed hub exercise', () => {
        for (const e of Object.values(p.exercises)) {
          const ex = toHubExercise(e);
          expect(ex.pattern.length, e.id).toBe(ex.beatsPerBar * ex.subdivision);
          expect(ex.beatsPerBar, e.id).toBeGreaterThanOrEqual(4);
          expect(ex.unscored === true, e.id).toBe(!e.hub.scorable);
        }
      });
      it('every session is contiguous, references real exercises and tests, and is found by its own date', () => {
        for (const s of p.sessions) {
          const d = dayFor(s.date, p, []);
          expect(d.state === 'during' && d.session?.date === s.date, s.date).toBe(true);
          let t = 0;
          for (const b of s.blocks) {
            expect(b.start, `${s.date} ${b.key}`).toBe(t);
            t = b.end;
            for (const it of b.items ?? []) expect(p.exercises[it.ex], `${s.date} ${b.key} ${it.ex}`).toBeTruthy();
            for (const c of b.chunks ?? []) expect(b.items?.[c.i], `${s.date} ${b.key} chunk`).toBeTruthy();
            for (const tid of [...(b.tests ?? []), ...(b.test ? [b.test] : [])]) expect(p.tests[tid], tid).toBeTruthy();
            if (b.chunks) expect(b.chunks.reduce((n, c) => n + c.min, 0), `${s.date} ${b.key}`).toBe(b.end - b.start);
            if (b.log === 'test') expect(hasClick(b)).toBe(true);
          }
          expect(totalSeconds(s)).toBe(s.minutes * 60);
          if (s.blocks.length) expect(blockAt(s, 0)).toBe(0);
        }
      });
    });
  }
});

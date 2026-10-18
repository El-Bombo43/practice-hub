import type { Practice, Snapshot } from '../game/practice';
import { fmtOffset } from './format';

const NAMES = { keyboard: 'keyboard', midi: 'pad', audio: 'guitar' } as const;

/**
 * Shown when an input's recent hits sit consistently away from the click. If the player was
 * actually in time, the offset is input latency, and this fixes it from real playing in one click.
 */
export function CenterBar({ snap, ctrl }: { snap: Snapshot; ctrl: Practice }) {
  const c = snap.centering;
  if (!c || Math.abs(c.medianMs) < 6) return null;
  const late = c.medianMs > 0;
  return (
    <section className="panel center-bar" role="status">
      <span>
        Your {NAMES[c.source]} hits sit {Math.abs(Math.round(c.medianMs))} ms {late ? 'late' : 'early'} of the click on average
        (last {c.n} hits). If you were playing with the click, that is input latency.
      </span>
      <button type="button" className="next-btn" style={{ margin: 0 }} onClick={() => ctrl.recenter()}>
        Re-center ({fmtOffset(-c.medianMs)} ms)
      </button>
    </section>
  );
}

import type { RepResult } from '../scoring/score';

/** Explains hits that landed too far from any slot to be scored, which is what a timing problem looks like. */
export function StrayNote({ rep, windowMs }: { rep: RepResult | undefined; windowMs: number }) {
  if (!rep || rep.strays.length === 0) return null;
  const offs = rep.strays.map((s) => s.offsetMs);
  const mean = offs.reduce((a, b) => a + b, 0) / offs.length;
  const sameSide = offs.every((o) => o > 0) || offs.every((o) => o < 0);
  const list = offs.slice(0, 6).map((o) => `${o > 0 ? '+' : '−'}${Math.abs(Math.round(o))}`).join(', ');
  const lean = sameSide
    ? `They all ran ${mean > 0 ? 'late' : 'early'} (about ${Math.abs(Math.round(mean))} ms): that is input latency, so re-run calibration.`
    : 'They fall on both sides: that is a doubled or jittery input, or notes that were meant for another slot.';
  return (
    <p className="tl-diag" role="status">
      {rep.strays.length} hit{rep.strays.length > 1 ? 's' : ''} landed too far from a slot to score (ring marks): {list} ms. {lean}
      {' '}Scoring window ±{Math.max(2 * windowMs, 60)} ms.
    </p>
  );
}

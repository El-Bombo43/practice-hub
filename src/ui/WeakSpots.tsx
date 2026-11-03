import type { Practice, Snapshot } from '../game/practice';
import { weakSpots } from '../game/progress';
import { fmtOffset } from './format';
import { countLabel } from './StrumTimeline';

const ISSUE = { late: 'late', early: 'early', miss: 'missed' } as const;

/** Slots that keep going wrong across the bars played so far, and a one-click way to slow down and work on them. */
export function WeakSpots({ snap, ctrl }: { snap: Snapshot; ctrl: Practice }) {
  const ex = snap.exercise;
  if (ex.unscored) return null;
  const spots = weakSpots(snap.live.slots);
  if (spots.length === 0) return null;
  const slowBy = Math.max(2, Math.round(snap.bpm * 0.1));
  return (
    <section className="panel weak" aria-label="Weak spots">
      <h2 className="section-title">Slots to watch</h2>
      <ul>
        {spots.map((s) => {
          const beat = Math.floor(s.slot / ex.subdivision) + 1;
          const count = countLabel(s.slot, ex.subdivision);
          return (
            <li key={s.slot}>
              <b className="mono">Beat {beat}{count && count !== String(beat) ? ` “${count}”` : ''}</b>
              {' '}{ISSUE[s.issue]} {Math.round(s.rate * 100)}% of the time
              {s.meanOffsetMs !== null && s.issue !== 'miss' ? ` (${fmtOffset(s.meanOffsetMs)} ms on average)` : ''}
              {ex.pattern[s.slot]?.lane ? ` · ${ex.pattern[s.slot].lane === 'down' ? '↓' : ex.pattern[s.slot].lane === 'up' ? '↑' : ex.pattern[s.slot].lane}` : ''}
            </li>
          );
        })}
      </ul>
      <p className="msg">Dashed columns on the timeline. Slow down until they are even, then bring the tempo back.</p>
      <button type="button" className="btn" onClick={() => ctrl.adjustBpm(-slowBy)}>Drop {slowBy} BPM to drill them</button>
    </section>
  );
}

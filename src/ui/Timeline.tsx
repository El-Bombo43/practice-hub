import { hasLanes } from '../data/exercises';
import type { Exercise, Lane } from '../data/types';
import type { LastRep } from '../game/practice';

const ARROW: Record<Lane, string> = { down: '↓', up: '↑', L: 'L', R: 'R' };
const TOP_LANES: Lane[] = ['down', 'L'];

export const fmtOffset = (ms: number): string => {
  const r = Math.round(ms);
  return r === 0 ? '0' : r > 0 ? `+${r}` : `−${Math.abs(r)}`;
};

export function Timeline({ exercise, last }: { exercise: Exercise; last: LastRep | null }) {
  const cols = exercise.pattern.length;
  const dense = cols > 8;
  const scale = dense ? 1 : 2; // px per ms
  const lanes = hasLanes(exercise);
  const cellH = lanes ? 96 : 56;
  const windowMs = last?.windowMs ?? 15;
  const bandW = Math.max(8, 2 * windowMs * scale);
  const rep = last?.rep;

  return (
    <section className="panel">
      <div className="tl-head">
        <h2>
          {rep
            ? `Last rep · ${rep.onTimeCount} of ${rep.expectedCount} inside ±${last.windowMs} ms${rep.extras ? ` · ${rep.extras} extra` : ''}`
            : 'Last rep · waiting for a bar'}
        </h2>
        <div className="legend">
          <span><i style={{ background: 'var(--early)' }} />Early</span>
          <span><i style={{ background: 'var(--on-time)' }} />On time</span>
          <span><i style={{ background: 'var(--late)' }} />Late</span>
        </div>
      </div>
      <div className="timeline" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {exercise.pattern.map((slot, i) => {
          const res = rep?.slots[i];
          const expected = slot.play && !slot.ghost;
          const top = !slot.lane || TOP_LANES.includes(slot.lane);
          const dotTop = lanes ? (top ? 16 : 66) : (cellH - 14) / 2;
          let color = 'var(--on-time)';
          let label = '';
          let labelColor = 'var(--muted)';
          if (res?.hit && res.offsetMs !== null) {
            if (!res.inWindow) color = res.offsetMs < 0 ? 'var(--early)' : 'var(--late)';
            if (!res.inWindow) labelColor = color;
            label = fmtOffset(res.offsetMs);
          } else if (rep && expected) label = 'miss';
          else if (slot.ghost) label = '·';
          else if (!slot.play) label = '–';
          const arrow = slot.lane ? ARROW[slot.lane] : '';
          const dx = res?.hit && res.offsetMs !== null ? res.offsetMs * scale : 0;
          return (
            <div key={i} className={`col${dense ? ' dense' : ''}${expected ? '' : ' rest'}`}>
              <div className="cell" style={{ height: cellH }}>
                <div className="band" style={{ width: bandW, marginLeft: -bandW / 2 }} />
                <div className="line" />
                {res?.hit && (
                  <div
                    className="dot"
                    style={{
                      top: dotTop, background: color,
                      left: `clamp(0px, calc(50% + ${dx}px - 7px), calc(100% - 14px))`,
                    }}
                  />
                )}
              </div>
              <span className="lbl" style={{ color: labelColor }}>{arrow}{dense ? '' : ' '}{label}</span>
            </div>
          );
        })}
      </div>
      {!rep && <p className="tl-empty">Start the click and play a bar. The first bar is a count-in.</p>}
    </section>
  );
}

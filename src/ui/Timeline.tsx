import { hasLanes } from '../data/exercises';
import type { Exercise, Lane } from '../data/types';
import type { LastRep } from '../game/practice';

const ARROW: Record<Lane, string> = { down: '↓', up: '↑', L: 'L', R: 'R' };
const TOP_LANES: Lane[] = ['down', 'R']; // down / right hand on the top row

export const fmtOffset = (ms: number): string => {
  const r = Math.round(ms);
  return r === 0 ? '0' : r > 0 ? `+${r}` : `−${Math.abs(r)}`;
};

export function Timeline({ exercise: fallback, last }: { exercise: Exercise; last: LastRep | null }) {
  const exercise = last?.exercise ?? fallback;
  const cols = exercise.pattern.length;
  const dense = cols > 8;
  const scale = dense ? 1 : 2; // px per ms
  const lanes = hasLanes(exercise);
  const pads = exercise.instrument === 'pads';
  const cellH = lanes ? 96 : 56;
  const windowMs = last?.windowMs ?? 15;
  const bandW = Math.max(8, 2 * windowMs * scale);
  const rep = last?.rep;
  const wrong = rep ? rep.slots.filter((s) => s.wrongLane).length : 0;
  const grid = { gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` };

  return (
    <section className="panel">
      <div className="tl-head">
        <h2>
          {rep
            ? `Last rep · ${rep.onTimeCount} of ${rep.expectedCount} inside ±${last.windowMs} ms${rep.extras ? ` · ${rep.extras} extra` : ''}${wrong ? ` · ${wrong} wrong hand` : ''}`
            : 'Last rep · waiting for a bar'}
        </h2>
        <div className="legend">
          <span><i style={{ background: 'var(--early)' }} />Early</span>
          <span><i style={{ background: 'var(--on-time)' }} />On time</span>
          <span><i style={{ background: 'var(--late)' }} />Late</span>
        </div>
      </div>
      <div className="timeline-wrap" style={{ background: 'var(--bg)', borderRadius: 'var(--r-card)', padding: '14px 0 12px' }}>
        {pads && (
          <div className={`sticking${dense ? ' dense' : ''}`} style={grid} aria-label="Sticking">
            {exercise.pattern.map((s, i) => (
              <span key={i} className={s.lane === 'L' ? 'l' : ''}>{s.play ? s.lane : ''}</span>
            ))}
          </div>
        )}
        <div className="timeline" style={{ ...grid, padding: 0, background: 'none' }}>
          {exercise.pattern.map((slot, i) => {
            const res = rep?.slots[i];
            const expected = slot.play && !slot.ghost;
            const rowLane = res?.wrongLane && res.hitLane ? res.hitLane : slot.lane;
            const top = !rowLane || TOP_LANES.includes(rowLane);
            const dotTop = lanes ? (top ? 16 : 66) : (cellH - 14) / 2;
            let color = 'var(--on-time)';
            let label = '';
            let labelColor = 'var(--muted)';
            if (res?.hit && res.offsetMs !== null) {
              if (res.wrongLane) { label = `✕${res.hitLane ?? ''}`; labelColor = 'var(--text)'; }
              else {
                if (!res.inWindow) color = res.offsetMs < 0 ? 'var(--early)' : 'var(--late)';
                if (!res.inWindow) labelColor = color;
                label = fmtOffset(res.offsetMs);
              }
            } else if (rep && expected) label = 'miss';
            else if (slot.ghost) label = '·';
            else if (!slot.play) label = '–';
            const arrow = pads ? '' : slot.lane ? ARROW[slot.lane] : '';
            const dx = res?.hit && res.offsetMs !== null ? res.offsetMs * scale : 0;
            const vel = res?.hit && res.velocity !== undefined ? res.velocity : null;
            return (
              <div key={i} className={`col${dense ? ' dense' : ''}${expected ? '' : ' rest'}`}>
                <div className="cell" style={{ height: cellH }}>
                  <div className="band" style={{ width: bandW, marginLeft: -bandW / 2 }} />
                  <div className="line" />
                  {res?.hit && (
                    <div
                      className={`dot${res.wrongLane ? ' wrong' : ''}`}
                      style={{
                        top: dotTop, background: color,
                        left: `clamp(0px, calc(50% + ${dx}px - 7px), calc(100% - 14px))`,
                      }}
                    />
                  )}
                </div>
                <span className="lbl" style={{ color: labelColor }}>{arrow}{dense || !arrow ? '' : ' '}{label}</span>
                {pads && (
                  <div className="vel">
                    {vel !== null && (
                      <div style={{
                        height: 4 + Math.round(vel * 24),
                        background: (res?.hitLane ?? slot.lane) === 'L' ? 'var(--vel-l)' : 'var(--vel-r)',
                      }} />
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {pads && (
          <div className="tl-foot">
            <span>Top row: right hand · bottom row: left hand</span>
            <span><i style={{ background: 'var(--vel-r)' }} />R velocity</span>
            <span><i style={{ background: 'var(--vel-l)' }} />L velocity</span>
          </div>
        )}
      </div>
      {!rep && <p className="tl-empty">Start the click and play a bar. The first bar is a count-in.</p>}
    </section>
  );
}

import type { Exercise, Slot } from '../data/types';
import type { LastRep, Practice } from '../game/practice';
import { accentStats } from '../scoring/score';
import { fmtOffset } from './format';
import { StrayNote } from './StrayNote';
import { usePlayhead } from './usePlayhead';

const COUNTS: Record<number, string[]> = { 2: ['', '&'], 3: ['', '&', 'a'], 4: ['', 'e', '&', 'a'], 6: ['', 'la', 'li', '&', 'la', 'li'] };

/** "1 e & a 2 e & a …": the beat number on each beat, syllables in between. */
export const countLabel = (i: number, sub: number): string => {
  const pos = i % sub;
  return pos === 0 ? String(Math.floor(i / sub) + 1) : (COUNTS[sub] ?? [])[pos] ?? '';
};

const arrowFor = (s: Slot, i: number): string => (s.lane ? (s.lane === 'down' ? '↓' : '↑') : i % 2 === 0 ? '↓' : '↑');

/** Strum screen: one column per slot with an arrow, a timing dot, the count, and accent loudness. */
export function StrumTimeline({ ctrl, exercise, last }: { ctrl: Practice; exercise: Exercise; last: LastRep | null }) {
  const cols = exercise.pattern.length;
  const rep = last?.rep;
  const windowMs = last?.windowMs ?? 15;
  const bandW = Math.max(8, 2 * windowMs); // 1 px per ms
  const head = usePlayhead(ctrl, cols);
  const accents = rep ? accentStats(exercise.pattern, rep.slots) : [];
  const accentAt = new Map(accents.map((a) => [a.index, a]));
  const accentTotal = exercise.pattern.filter((s) => s.accent && s.play).length;

  return (
    <section className="panel">
      <div className="tl-head">
        <h2>
          {rep
            ? `Last bar · ${rep.onTimeCount} of ${rep.expectedCount} strums on time · ${accents.filter((a) => a.landed).length} of ${accentTotal} accents`
            : 'Last bar · waiting for a bar'}
        </h2>
        <div className="legend">
          <span><b className="accent">&gt;</b>Accent</span>
          <span><b style={{ color: 'var(--arrow-ghost)' }}>↓</b>Ghost (miss the strings)</span>
          <span><i style={{ background: 'var(--late)' }} />Late</span>
        </div>
      </div>
      <div className="tl-stage strum-stage">
        <div className="strum" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
          {exercise.pattern.map((slot, i) => {
            const res = rep?.slots[i];
            const strike = slot.play && !slot.ghost;
            const acc = accentAt.get(i);
            let label = '';
            let labelColor = 'var(--late)';
            if (acc) {
              if (acc.pct !== null) {
                label = `${acc.pct >= 0 ? '+' : '−'}${Math.abs(Math.round(acc.pct * 100))}%`;
                labelColor = acc.landed ? 'var(--accent)' : 'var(--late)';
              } else if (rep) { label = 'miss'; labelColor = 'var(--muted)'; }
            } else if (res?.hit && res.offsetMs !== null && !res.inWindow) {
              label = `${fmtOffset(res.offsetMs)}ms`;
              labelColor = res.offsetMs < 0 ? 'var(--early)' : 'var(--late)';
            } else if (rep && strike && !res?.hit) { label = 'miss'; labelColor = 'var(--muted)'; }
            let dot = 'var(--on-time)';
            if (res?.hit && res.offsetMs !== null && !res.inWindow) dot = res.offsetMs < 0 ? 'var(--early)' : 'var(--late)';
            const dx = res?.hit && res.offsetMs !== null ? res.offsetMs : 0;
            const arrowCls = slot.accent && strike ? 'a-accent' : strike ? 'a-strike' : slot.ghost ? 'a-ghost' : 'a-rest';
            return (
              <div key={i} className={`scol${strike ? '' : ' faded'}`}>
                <span className="mark">{slot.accent && strike ? '>' : ''}</span>
                <span className={`arrow ${arrowCls}`}>{slot.play || slot.ghost ? arrowFor(slot, i) : ''}</span>
                <div className="cell" style={{ height: 56 }}>
                  {strike && <div className="band" style={{ width: bandW, marginLeft: -bandW / 2, top: 6, bottom: 6 }} />}
                  <div className="line" style={strike ? undefined : { background: 'var(--panel-border)' }} />
                  {rep?.strays.filter((s) => s.slot === i).map((s, k) => (
                    <div key={k} className="dot stray" title={`${fmtOffset(s.offsetMs)} ms from this slot`}
                      style={{ top: 22, width: 12, height: 12, left: `clamp(0px, calc(50% + ${s.offsetMs}px - 6px), calc(100% - 12px))` }} />
                  ))}
                  {res?.hit && (
                    <div className="dot" style={{
                      top: 22, width: 12, height: 12, background: dot,
                      left: `clamp(0px, calc(50% + ${dx}px - 6px), calc(100% - 12px))`,
                    }} />
                  )}
                </div>
                <span className={`count${i % exercise.subdivision === 0 ? ' beat' : ''}`}>{countLabel(i, exercise.subdivision)}</span>
                <span className="under" style={{ color: labelColor }}>{label}</span>
              </div>
            );
          })}
        </div>
        <div className="playhead" ref={head} aria-hidden="true" />
      </div>
      <StrayNote rep={rep} windowMs={windowMs} />
      <p className="tl-note">
        Big arrows: strike the strings · faded arrows: keep moving, miss the strings · % under an accent: how much louder it was than the strums around it
        {!rep && ' · start the click and play a bar (hold Shift on the keyboard for an accent)'}
      </p>
    </section>
  );
}

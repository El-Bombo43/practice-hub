import { useState } from 'react';
import { findExercise } from '../data/exercises';
import { baseId, calendarWeeks, cleanMaxSeries, exercisesWithHistory, weekMinutes } from '../game/history';
import { dateKey, streak, type Profile } from '../game/progress';

const SUB: Record<string, string> = { '2': '8ths', '3': 'triplets', '4': '16ths', '6': 'sextuplets' };

export const exerciseLabel = (key: string): string => {
  const ex = findExercise(baseId(key));
  const n = key.split('@')[1];
  return `${ex?.name ?? key}${n ? ` · ${SUB[n] ?? n}` : ''}`;
};
const instrumentOf = (key: string): 'guitar' | 'pads' | undefined => findExercise(baseId(key))?.instrument;

const W = 640;
const H = 220;
const PAD = { l: 44, r: 16, t: 14, b: 28 };

/** Hand-drawn SVG line: running clean max by day. */
function CleanMaxChart({ points, label }: { points: { date: string; bpm: number }[]; label: string }) {
  const day = (d: string) => Date.parse(`${d}T00:00:00`) / 86400000;
  const lo = Math.max(0, Math.floor((Math.min(...points.map((p) => p.bpm)) - 6) / 10) * 10);
  const hi = Math.ceil((Math.max(...points.map((p) => p.bpm)) + 6) / 10) * 10;
  const t0 = day(points[0].date);
  const span = Math.max(7, day(points[points.length - 1].date) - t0);
  const x = (d: string) => PAD.l + (points.length === 1 ? (W - PAD.l - PAD.r) / 2 : ((day(d) - t0) / span) * (W - PAD.l - PAD.r));
  const y = (bpm: number) => PAD.t + (1 - (bpm - lo) / (hi - lo)) * (H - PAD.t - PAD.b);
  const ticks = [0, 1, 2, 3].map((i) => Math.round(lo + ((hi - lo) * i) / 3));
  const last = points[points.length - 1];
  return (
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label={`Clean max over time for ${label}: now ${last.bpm} BPM`}>
      {ticks.map((t) => (
        <g key={t}>
          <line x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} stroke="var(--panel-border)" />
          <text x={PAD.l - 8} y={y(t) + 4} textAnchor="end">{t}</text>
        </g>
      ))}
      <text x={PAD.l} y={H - 8}>{points[0].date.slice(5)}</text>
      <text x={W - PAD.r} y={H - 8} textAnchor="end">{last.date.slice(5)}</text>
      {points.length > 1 && (
        <polyline fill="none" stroke="var(--accent)" strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round"
          points={points.map((p) => `${x(p.date)},${y(p.bpm)}`).join(' ')} />
      )}
      {points.map((p, i) => <circle key={p.date} cx={x(p.date)} cy={y(p.bpm)} r={i === points.length - 1 ? 5 : 3.5} fill="var(--accent)" />)}
      <text x={x(last.date)} y={y(last.bpm) - 10} textAnchor={points.length > 1 ? 'end' : 'middle'} style={{ fill: 'var(--text)', fontWeight: 700 }}>{last.bpm}</text>
    </svg>
  );
}

export function History({ profile }: { profile: Profile }) {
  const withHistory = exercisesWithHistory(profile.log);
  const [pick, setPick] = useState<string | null>(null);
  const current = pick && withHistory.includes(pick) ? pick : withHistory[0];
  const series = current ? cleanMaxSeries(profile.log, current) : [];
  const weeks = calendarWeeks(profile.dayMinutes);
  const today = dateKey();
  const week = weekMinutes(profile.log, instrumentOf);
  const recent = [...profile.log].reverse().slice(0, 12);
  const noSessions = profile.log.length === 0;

  return (
    <div className="history">
      <div className="hgrid">
        <section className="panel" style={{ gridColumn: '1 / -1' }}>
          <div className="tl-head">
            <h2>Clean max over time</h2>
            {withHistory.length > 0 && (
              <select aria-label="Exercise" value={current} onChange={(e) => setPick(e.target.value)}
                style={{ minHeight: 44, borderRadius: 12, border: '1px solid var(--off)', background: 'var(--bg)', padding: '0 10px' }}>
                {withHistory.map((k) => <option key={k} value={k}>{exerciseLabel(k)}</option>)}
              </select>
            )}
          </div>
          {series.length ? <CleanMaxChart points={series} label={exerciseLabel(current)} />
            : <p className="empty">No clean reps logged yet. Land a clean bar and your clean max will be charted here.</p>}
        </section>

        <section className="panel">
          <div className="tl-head"><h2>Last 8 weeks</h2><span className="small-mono">{streak(profile.dayMinutes)} day streak</span></div>
          <div className="cal" role="img" aria-label="Practice calendar, darker means more minutes">
            {weeks.map((w) => (
              <div className="wk" key={w[0].date}>
                {w.map((d) => (
                  <div key={d.date} title={`${d.date}: ${Math.round(d.minutes)} min`}
                    className={`day l${d.level}${d.future ? ' future' : ''}${d.date === today ? ' today' : ''}`} />
                ))}
              </div>
            ))}
          </div>
          <p className="tl-note" style={{ textAlign: 'left' }}>Rows are Monday to Sunday. Ten minutes or more keeps the streak.</p>
        </section>

        <section className="panel">
          <h2 className="section-title" style={{ marginBottom: 14 }}>This week</h2>
          <div className="stats">
            <div className="stat"><span className="k">Guitar</span><span className="v">{week.guitar} min</span></div>
            <div className="stat"><span className="k">Pads</span><span className="v">{week.pads} min</span></div>
          </div>
        </section>
      </div>

      <section className="panel">
        <h2 className="section-title" style={{ marginBottom: 8 }}>Recent sessions</h2>
        {noSessions ? <p className="empty">Nothing yet. Start the click on the Practice tab and your sessions will be listed here.</p> : (
          <table className="table">
            <thead><tr><th>Date</th><th>Exercise</th><th className="n">Min</th><th className="n">Best clean</th><th className="n">Accuracy</th></tr></thead>
            <tbody>
              {recent.map((e) => (
                <tr key={e.id}>
                  <td className="mono">{e.date.slice(5)}</td>
                  <td>{exerciseLabel(e.exerciseId)}</td>
                  <td className="n">{e.minutes}</td>
                  <td className="n">{e.bestCleanBpm ?? '–'}</td>
                  <td className="n">{e.accuracy === null ? '–' : `${e.accuracy}%`}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}

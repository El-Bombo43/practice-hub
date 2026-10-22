import { useState } from 'react';
import { findExercise } from '../data/exercises';
import type { Program } from '../data/program';
import { testRows, type ProgramLog } from '../data/programLog';
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

type Line = { id: string; label: string; points: { date: string; bpm: number }[] };
const TEST_STYLES = [
  { dash: '6 4', color: 'var(--text)' }, { dash: '2 4', color: 'var(--muted)' }, { dash: '10 4 2 4', color: 'var(--text)' },
  { dash: '4 4', color: 'var(--faded)' }, { dash: '1 5', color: 'var(--text)' }, { dash: '8 3', color: 'var(--muted)' },
];

/** Hand-drawn SVG: the running clean max (solid accent) plus one dashed CT line per program test. */
function CleanMaxChart({ points, label, tests }: { points: { date: string; bpm: number }[]; label: string; tests: Line[] }) {
  const day = (d: string) => Date.parse(`${d}T00:00:00`) / 86400000;
  const all = [...points, ...tests.flatMap((t) => t.points)].sort((a, b) => a.date.localeCompare(b.date));
  const lo = Math.max(0, Math.floor((Math.min(...all.map((p) => p.bpm)) - 6) / 10) * 10);
  const hi = Math.ceil((Math.max(...all.map((p) => p.bpm)) + 6) / 10) * 10;
  const t0 = day(all[0].date);
  const span = Math.max(7, day(all[all.length - 1].date) - t0);
  const x = (d: string) => PAD.l + (all.length === 1 ? (W - PAD.l - PAD.r) / 2 : ((day(d) - t0) / span) * (W - PAD.l - PAD.r));
  const y = (bpm: number) => PAD.t + (1 - (bpm - lo) / (hi - lo)) * (H - PAD.t - PAD.b);
  const ticks = [0, 1, 2, 3].map((i) => Math.round(lo + ((hi - lo) * i) / 3));
  const last = points[points.length - 1];
  return (
    <>
    <svg className="chart" viewBox={`0 0 ${W} ${H}`} width="100%" role="img"
      aria-label={`Clean max over time${last ? ` for ${label}: now ${last.bpm} BPM` : ''}${tests.length ? `, with ${tests.length} program test lines` : ''}`}>
      {ticks.map((t) => (
        <g key={t}>
          <line x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} stroke="var(--panel-border)" />
          <text x={PAD.l - 8} y={y(t) + 4} textAnchor="end">{t}</text>
        </g>
      ))}
      <text x={PAD.l} y={H - 8}>{all[0].date.slice(5)}</text>
      <text x={W - PAD.r} y={H - 8} textAnchor="end">{all[all.length - 1].date.slice(5)}</text>
      {tests.map((t, i) => {
        const st = TEST_STYLES[i % TEST_STYLES.length];
        const end = t.points[t.points.length - 1];
        return (
          <g key={t.id}>
            {t.points.length > 1 && <polyline fill="none" stroke={st.color} strokeWidth="1.5" strokeDasharray={st.dash} points={t.points.map((p) => `${x(p.date)},${y(p.bpm)}`).join(' ')} />}
            {t.points.map((p) => <circle key={p.date} cx={x(p.date)} cy={y(p.bpm)} r="2.5" fill={st.color} />)}
            <text x={x(end.date) + 6} y={y(end.bpm) + 4} style={{ fill: st.color }}>{t.id} {end.bpm}</text>
          </g>
        );
      })}
      {last && points.length > 1 && (
        <polyline fill="none" stroke="var(--accent)" strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round"
          points={points.map((p) => `${x(p.date)},${y(p.bpm)}`).join(' ')} />
      )}
      {points.map((p, i) => <circle key={p.date} cx={x(p.date)} cy={y(p.bpm)} r={i === points.length - 1 ? 5 : 3.5} fill="var(--accent)" />)}
      {last && <text x={x(last.date)} y={y(last.bpm) - 10} textAnchor={points.length > 1 ? 'end' : 'middle'} style={{ fill: 'var(--text)', fontWeight: 700 }}>{last.bpm}</text>}
    </svg>
    {tests.length > 0 && (
      <p className="tl-note" style={{ textAlign: 'left' }}>
        Solid: clean max from the app. Dashed: clean tempo (CT) from each program test: {tests.map((t) => `${t.id} ${t.label}`).join(' · ')}.
      </p>
    )}
    </>
  );
}

export function History({ profile, program, programLog }: { profile: Profile; program: Program | null; programLog: ProgramLog }) {
  const withHistory = exercisesWithHistory(profile.log);
  const [pick, setPick] = useState<string | null>(null);
  const current = pick && withHistory.includes(pick) ? pick : withHistory[0];
  const series = current ? cleanMaxSeries(profile.log, current) : [];
  const weeks = calendarWeeks(profile.dayMinutes);
  const today = dateKey();
  const week = weekMinutes(profile.log, instrumentOf);
  const recent = [...profile.log].reverse().slice(0, 12);
  const noSessions = profile.log.length === 0;
  const testLines: Line[] = program
    ? Object.values(program.tests)
        .map((t) => ({ id: t.id, label: t.name, rows: testRows(programLog, t.id) }))
        .filter((t) => t.rows.some((r) => r.ct !== null))
        .map((t) => ({ id: t.id, label: t.label, points: t.rows.flatMap((r) => (r.ct !== null ? [{ date: r.date, bpm: r.ct }] : [])) }))
    : [];
  const testTables = testLines.map((t) => ({ ...t, rows: testRows(programLog, t.id) }));

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
          {series.length || testLines.length ? <CleanMaxChart points={series} label={current ? exerciseLabel(current) : 'program tests'} tests={testLines} />
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

      {testTables.length > 0 && (
        <section className="panel">
          <h2 className="section-title" style={{ marginBottom: 8 }}>Program tests: clean tempo (CT) and motion ceiling (MC)</h2>
          {testTables.map((t) => (
            <details key={t.id} open style={{ marginBottom: 12 }}>
              <summary style={{ cursor: 'pointer', minHeight: 36, display: 'flex', alignItems: 'center' }}><b>{t.id}</b>&nbsp;{t.label}</summary>
              <table className="table">
                <thead><tr><th>Date</th><th className="n">CT</th><th className="n">MC</th><th className="n">MC − CT</th><th className="n">CT vs kickoff</th></tr></thead>
                <tbody>
                  {t.rows.map((r) => (
                    <tr key={r.date}>
                      <td className="mono">{r.date.slice(5)}</td>
                      <td className="n">{r.ct ?? '–'}</td>
                      <td className="n">{r.mc ?? '–'}</td>
                      <td className="n">{r.gap ?? '–'}</td>
                      <td className="n">{r.pctFromStart === null ? '–' : `${r.pctFromStart > 0 ? '+' : r.pctFromStart < 0 ? '−' : ''}${Math.abs(r.pctFromStart)}%`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </details>
          ))}
        </section>
      )}

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

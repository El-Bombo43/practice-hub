import { dateKey, streak, type Profile } from '../game/progress';
import { exerciseLabel } from './History';

/** End-of-routine summary built from today's log entries. */
export function SummaryDialog(props: { profile: Profile; sessionXp: number; sessionPbs: number; onClose: () => void }) {
  const today = dateKey();
  const entries = props.profile.log.filter((e) => e.date === today && e.minutes > 0);
  const minutes = Math.round(entries.reduce((s, e) => s + e.minutes, 0) * 10) / 10;
  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog wide" role="dialog" aria-label="Today's routine summary" onClick={(e) => e.stopPropagation()}>
        <h2>Today's routine</h2>
        <div className="stats">
          <div className="stat"><span className="k">Practised</span><span className="v">{minutes} min</span></div>
          <div className="stat"><span className="k">XP this session</span><span className="v accent">+{props.sessionXp}</span></div>
          <div className="stat"><span className="k">New personal bests</span><span className="v">{props.sessionPbs}</span></div>
          <div className="stat"><span className="k">Day streak</span><span className="v">{streak(props.profile.dayMinutes)}</span></div>
        </div>
        {entries.length === 0 ? <p className="empty">No practice logged yet today.</p> : (
          <table className="table">
            <thead><tr><th>Exercise</th><th className="n">Min</th><th className="n">Best clean</th><th className="n">Accuracy</th></tr></thead>
            <tbody>
              {entries.map((e) => (
                <tr key={e.id}>
                  <td>{exerciseLabel(e.exerciseId)}</td>
                  <td className="n">{e.minutes}</td>
                  <td className="n">{e.bestCleanBpm ?? '–'}</td>
                  <td className="n">{e.accuracy === null ? '–' : `${e.accuracy}%`}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

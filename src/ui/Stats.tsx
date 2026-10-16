import type { Practice, Snapshot } from '../game/practice';
import { accuracyPct, balance, drumTip, evenness, meanOffset, pickTip } from '../game/progress';
import { fmtOffset } from './Timeline';

export function Stats({ snap, ctrl }: { snap: Snapshot; ctrl: Practice }) {
  const { live, sessionXp } = snap;
  const pads = snap.exercise.instrument === 'pads';
  const acc = accuracyPct(live);
  const mean = meanOffset(live);
  const bal = balance(live);
  const t = pads ? drumTip(live, snap.leadWeaker) : pickTip(live);
  const ev = pads ? null : evenness(live);
  const r = snap.report;
  return (
    <>
    {r && (
      <section className="panel report" aria-label="Endurance report">
        <h2 className="section-title">Endurance · {r.seconds} s at {r.bpm} BPM</h2>
        <div className="grid">
          <div className="stat"><span className="k">Accuracy</span><span className="v">{r.accuracy === null ? '–' : `${r.accuracy}%`}</span></div>
          <div className="stat"><span className="k">Clean reps</span><span className="v">{r.cleanReps}/{r.reps}</span></div>
          <div className="stat"><span className="k">Avg offset</span><span className="v">{r.meanOffset === null ? '–' : `${fmtOffset(r.meanOffset)} ms`}</span></div>
          <div className="stat">
            <span className="k">Drift per 10 s</span>
            <span className="v">{r.drift === null ? '–' : `${r.drift > 0 ? '+' : r.drift < 0 ? '−' : ''}${Math.abs(r.drift)} ms`}</span>
          </div>
        </div>
        <span className="msg">
          {r.drift === null ? 'Not enough hits to measure drift.'
            : Math.abs(r.drift) < 1 ? 'Steady: your timing held for the whole run.'
            : r.drift > 0 ? 'You got later as the run went on. Check for tension, or drop the tempo a notch.'
            : 'You crept ahead of the click as the run went on.'}
        </span>
        <div className="row"><button type="button" className="btn" onClick={() => ctrl.toggle()}>Run again</button></div>
      </section>
    )}
    <section className="panel stats">
      <div className="stat"><span className="k">Accuracy</span><span className="v">{acc === null ? '–' : `${acc}%`}</span></div>
      <div className="stat"><span className="k">Combo · best {live.bestCombo}</span><span className="v">×{live.combo}</span></div>
      {pads ? (
        <div className="stat"><span className="k">L / R balance</span><span className="v">{bal ? `${bal.pct}%` : '–'}</span></div>
      ) : (
        <div className="stat"><span className="k">Avg offset</span><span className="v">{mean === null ? '–' : `${fmtOffset(mean)} ms`}</span></div>
      )}
      {ev && (
        <div className="stat">
          <span className="k">↓ / ↑ offset</span>
          <span className="v">{fmtOffset(ev.offD)} / {fmtOffset(ev.offU)}</span>
          <span className="k">↑ loudness {ev.loudD > 0 ? Math.round((100 * ev.loudU) / ev.loudD) : '–'}% of ↓</span>
        </div>
      )}
      <div className="stat"><span className="k">XP this session</span><span className="v accent">+{sessionXp}</span></div>
      <div className={`tip ${t.tone}`}>
        <b>{t.title}</b>
        <span>{t.text}</span>
        {t.offerLead && (
          <button type="button" className="tip-btn" onClick={() => ctrl.setLeadWeaker(true)}>Lead with weaker hand</button>
        )}
      </div>
    </section>
    </>
  );
}

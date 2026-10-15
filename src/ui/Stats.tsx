import type { Practice, Snapshot } from '../game/practice';
import { accuracyPct, balance, drumTip, meanOffset, tip } from '../game/progress';
import { fmtOffset } from './Timeline';

export function Stats({ snap, ctrl }: { snap: Snapshot; ctrl: Practice }) {
  const { live, sessionXp } = snap;
  const pads = snap.exercise.instrument === 'pads';
  const acc = accuracyPct(live);
  const mean = meanOffset(live);
  const bal = balance(live);
  const t = pads ? drumTip(live, snap.leadWeaker) : tip(mean);
  return (
    <section className="panel stats">
      <div className="stat"><span className="k">Accuracy</span><span className="v">{acc === null ? '–' : `${acc}%`}</span></div>
      <div className="stat"><span className="k">Combo · best {live.bestCombo}</span><span className="v">×{live.combo}</span></div>
      {pads ? (
        <div className="stat"><span className="k">L / R balance</span><span className="v">{bal ? `${bal.pct}%` : '–'}</span></div>
      ) : (
        <div className="stat"><span className="k">Avg offset</span><span className="v">{mean === null ? '–' : `${fmtOffset(mean)} ms`}</span></div>
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
  );
}

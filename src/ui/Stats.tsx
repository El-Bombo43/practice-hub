import { accuracyPct, meanOffset, tip, type Live } from '../game/progress';
import { fmtOffset } from './Timeline';

export function Stats({ live, sessionXp }: { live: Live; sessionXp: number }) {
  const acc = accuracyPct(live);
  const mean = meanOffset(live);
  const t = tip(mean);
  return (
    <section className="panel stats">
      <div className="stat"><span className="k">Accuracy</span><span className="v">{acc === null ? '–' : `${acc}%`}</span></div>
      <div className="stat"><span className="k">Combo · best {live.bestCombo}</span><span className="v">×{live.combo}</span></div>
      <div className="stat"><span className="k">Avg offset</span><span className="v">{mean === null ? '–' : `${fmtOffset(mean)} ms`}</span></div>
      <div className="stat"><span className="k">XP this session</span><span className="v accent">+{sessionXp}</span></div>
      <div className={`tip ${t.tone}`}><b>{t.title}</b><span>{t.text}</span></div>
    </section>
  );
}

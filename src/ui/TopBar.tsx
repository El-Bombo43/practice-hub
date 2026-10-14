import { level, xpInLevel, XP_PER_LEVEL } from '../game/progress';

export function TopBar(props: { xp: number; streak: number; step: number; steps: number; minLeft: number; onSettings: () => void }) {
  const xp = xpInLevel(props.xp);
  return (
    <header className="topbar">
      <span className="brand">Practice Hub</span>
      <div className="level">
        <span className="pill">Lv {level(props.xp)}</span>
        <div className="xpbar" role="img" aria-label={`${xp} of ${XP_PER_LEVEL} XP`}>
          <div style={{ width: `${(100 * xp) / XP_PER_LEVEL}%` }} />
        </div>
        <span className="small-mono">{xp}/{XP_PER_LEVEL} XP</span>
      </div>
      <div className="streak">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--late)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M12 3c1 3 5 5 5 10a5 5 0 0 1-10 0c0-2 1-3.5 2-4.5.3 2 1.3 3 2.5 3C11 8.5 11 6 12 3z" />
        </svg>
        <b>{props.streak}</b><span>day streak</span>
      </div>
      <span className="step-info">Step {props.step}/{props.steps} · {props.minLeft} min left</span>
      <button type="button" className="icon-btn" onClick={props.onSettings} aria-label="Settings">⚙</button>
    </header>
  );
}

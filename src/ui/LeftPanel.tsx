import { useEffect, useState } from 'react';
import type { Practice, Snapshot } from '../game/practice';
import type { MidiManager } from '../input/midi';
import { useMidi } from './useMidi';

const SUB_NAMES: Record<number, string> = { 2: '8ths', 3: 'triplets', 4: '16ths', 6: 'sextuplets' };
const SUB_LABELS: Record<number, string> = { 2: '8ths', 3: 'Triplets', 4: '16ths', 6: 'Sextuplets' };
const KIND_NAMES = { picking: 'Alternate picking', strum: 'Strumming', rudiment: 'Rudiments' } as const;

export function LeftPanel(props: {
  ctrl: Practice; snap: Snapshot; nextBtn: React.ReactNode; midi: MidiManager; onRemap: () => void;
}) {
  const { ctrl, snap, nextBtn, onRemap } = props;
  const midi = useMidi(props.midi);
  const { exercise: ex, bpm, live, running, settings } = snap;
  const [beat, setBeat] = useState(-1);

  useEffect(() => {
    let raf = 0;
    const loop = () => { setBeat(ctrl.currentBeat()); raf = requestAnimationFrame(loop); };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [ctrl]);

  const pb = snap.profile.personalBests[ex.id];
  const need = ex.ramp.cleanRepsToAdvance;
  const more = Math.max(0, need - live.cleanStreak);
  const diff = pb === undefined ? null : bpm - pb;

  return (
    <section className="left">
      <div>
        <span className="eyebrow">{ex.instrument === 'guitar' ? 'Guitar' : 'Pads'} · {KIND_NAMES[ex.kind]}</span>
        <h1 className="ex-name">{ex.name}</h1>
      </div>
      <div>
        <div className="tempo">{bpm}</div>
        <div className="tempo-sub">
          BPM · {SUB_NAMES[ex.subdivision]}
          {diff !== null && (
            <> · <span className="accent">{diff === 0 ? 'at your clean max' : diff > 0 ? `${diff} over your clean max` : `${-diff} under your clean max`}</span></>
          )}
          {diff === null && <> · no clean max yet</>}
        </div>
      </div>
      {snap.subOptions.length > 1 && (
        <div className="toggles" role="group" aria-label="Count">
          <span className="hint">Count</span>
          {snap.subOptions.map((n) => (
            <button type="button" key={n} className="toggle" aria-pressed={ex.subdivision === n}
              onClick={() => ctrl.setSubdivision(n)}>{SUB_LABELS[n]}</button>
          ))}
        </div>
      )}
      <div className="beats" role="img" aria-label={beat >= 0 ? `Beat ${beat + 1} of ${ex.beatsPerBar}` : 'Stopped'}>
        {Array.from({ length: ex.beatsPerBar }, (_, i) => <div key={i} className={i === beat ? 'on' : ''} />)}
      </div>
      <div className="reps">
        <span className="label">Clean reps</span>
        <div className="pips" role="img" aria-label={`${live.cleanStreak} of ${need} clean reps`}>
          {Array.from({ length: need }, (_, i) => <div key={i} className={i < live.cleanStreak ? 'on' : ''} />)}
        </div>
        <span className="mono accent" style={{ fontSize: 14 }}>{more} more → {Math.min(300, bpm + ex.ramp.stepBpm)}</span>
      </div>
      {ex.instrument === 'pads' && (
        <div className="chips">
          <span>MPK pads:</span>
          <span className="chip">L = note {settings.midi.left}</span>
          <span className="chip">R = note {settings.midi.right}</span>
          <span className={`chip${midi.connected ? '' : ' warn'}`}>
            {midi.devices().find((d) => d.id === settings.midi.deviceId)?.name ?? (midi.connected ? 'any device' : 'MIDI off · keyboard works')}
          </span>
          {!midi.connected && <button type="button" className="link-btn" onClick={() => void midi.init()}>Connect MIDI</button>}
          <button type="button" className="link-btn" onClick={onRemap}>Remap</button>
        </div>
      )}
      <div className="controls">
        <button type="button" className="round" onClick={() => ctrl.adjustBpm(-4)} aria-label="Tempo down 4">−4</button>
        <button type="button" className="primary" onClick={() => ctrl.toggle()}>
          {running ? (
            <><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" aria-hidden="true"><line x1="8" y1="5" x2="8" y2="19" /><line x1="16" y1="5" x2="16" y2="19" /></svg>Pause</>
          ) : (
            <><svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M7 4l13 8-13 8z" /></svg>Start</>
          )}
        </button>
        <button type="button" className="round" onClick={() => ctrl.adjustBpm(4)} aria-label="Tempo up 4">+4</button>
      </div>
      <div className="toggles">
        <button type="button" className="toggle" aria-pressed={settings.gapClick}
          onClick={() => ctrl.applySettings({ ...settings, gapClick: !settings.gapClick })}>Gap click</button>
        {ex.instrument === 'pads' && (
          <button type="button" className="toggle" aria-pressed={snap.leadWeaker}
            onClick={() => ctrl.setLeadWeaker(!snap.leadWeaker)}>Lead with weaker hand</button>
        )}
        <button type="button" className="toggle" aria-pressed={snap.burst}
          onClick={() => ctrl.setBurst(!snap.burst)}>Burst mode</button>
        <label className="vol">Click
          <input type="range" min={0} max={1} step={0.05} value={settings.clickVolume} aria-label="Click volume"
            onChange={(e) => ctrl.applySettings({ ...settings, clickVolume: Number(e.target.value) })} />
        </label>
      </div>
      {snap.toast && <div className="toast" role="status">{snap.toast}</div>}
      {nextBtn}
      <span className="hint">
        {ex.instrument === 'guitar' ? 'F = downstroke · J = upstroke' : 'F = left · J = right'} · Space = start / pause · BPM 30–300
      </span>
    </section>
  );
}

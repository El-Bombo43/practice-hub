import { useEffect, useRef, useState } from 'react';
import type { Settings } from '../data/store';
import type { Practice } from '../game/practice';
import type { AudioDevice, AudioInput } from '../input/audioOnset';
import { CalibrationSection } from './CalibrationSection';
import { useAudio } from './useAudio';

const dbToPct = (db: number): number => Math.max(0, Math.min(100, ((db + 60) / 60) * 100));

export function AudioDialog(props: { ctrl: Practice; audio: AudioInput; settings: Settings; onClose: () => void }) {
  const { ctrl, settings: s } = props;
  const audio = useAudio(props.audio);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const bar = useRef<HTMLDivElement>(null);
  const peak = useRef<HTMLDivElement>(null);
  const readout = useRef<HTMLSpanElement>(null);

  const setAudio = (patch: Partial<Settings['audio']>) => {
    const cur = ctrl.getSnapshot().settings;
    ctrl.applySettings({ ...cur, audio: { ...cur.audio, ...patch } });
  };
  const refresh = () => void audio.devices().then(setDevices);
  useEffect(() => { if (audio.active) refresh(); }, [audio, audio.active]); // eslint-disable-line react-hooks/exhaustive-deps

  // level meter, updated per frame without re-rendering
  useEffect(() => {
    let raf = 0;
    const loop = () => {
      const db = (v: number) => (v > 0 ? 20 * Math.log10(v) : -120);
      if (bar.current) {
        bar.current.style.width = `${dbToPct(db(audio.rms))}%`;
        bar.current.style.background = performance.now() - audio.lastOnsetAt < 150 ? 'var(--accent)' : 'var(--grid)';
      }
      if (readout.current) {
        const d = audio.lastPeak > 0 ? Math.round(20 * Math.log10(audio.lastPeak)) : null;
        readout.current.textContent = `Picks detected: ${audio.picks}${d === null ? '' : ` · last pick ${d} dB`}`;
      }
      if (peak.current) peak.current.style.left = `${dbToPct(db(audio.peak))}%`;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [audio]);

  const connect = async (deviceId: string | null) => {
    setAudio({ deviceId });
    if (await audio.start(deviceId, ctrl.getSnapshot().settings.audio.sensitivity)) refresh();
  };

  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog" role="dialog" aria-label="Guitar input" onClick={(e) => e.stopPropagation()}>
        <h2>Guitar input</h2>
        <span className="msg">Close Ableton first — it can hold the interface.</span>

        <div className="field">
          <span className="k">Audio input (clean DI from the interface)</span>
          {!audio.active ? (
            <div className="row">
              <button type="button" className="btn" onClick={() => void connect(s.audio.deviceId)}>Connect input</button>
            </div>
          ) : (
            <div className="row">
              <select value={s.audio.deviceId ?? ''} aria-label="Audio input"
                onChange={(e) => void connect(e.target.value || null)}>
                <option value="">Default input</option>
                {devices.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
              </select>
              <button type="button" className="btn" onClick={() => audio.stop()}>Disconnect</button>
            </div>
          )}
          {audio.error && <span className="msg" role="alert">{audio.error}</span>}
        </div>

        <div className="field">
          <span className="k">Level — the bar lights up on each detected pick</span>
          <div className="meter" aria-hidden="true"><div ref={bar} className="meter-bar" /><div ref={peak} className="meter-peak" /></div>
          <span className="msg" ref={readout} role="status" />
          <span className="msg">Pick each string in turn: every pick should add one to the count. If a string is missed, raise Sensitivity; if one pick counts twice, lower it.</span>
          <div className="learn-row">
            <span className="k">Channel</span>
            <span className="v">{audio.channel === null ? (audio.active ? 'measuring…' : '–') : `${audio.channel + 1}`}</span>
            <button type="button" className="btn" disabled={!audio.active} onClick={() => audio.relock()}>Detect channel</button>
            <span className="msg">Picks the loudest channel over one second of playing.</span>
          </div>
        </div>

        <div className="field">
          <span className="k">Sensitivity</span>
          <input type="range" min={0} max={1} step={0.05} value={s.audio.sensitivity} aria-label="Sensitivity"
            style={{ accentColor: 'var(--accent)' }}
            onChange={(e) => { const v = Number(e.target.value); setAudio({ sensitivity: v }); audio.setSensitivity(v); }} />
          <span className="msg">Raise it if soft strokes are missed; lower it if you see extra detections.</span>
        </div>

        <hr />
        <CalibrationSection ctrl={ctrl} settings={s} sources={['audio']} hint="Mute the strings and pick on each click." />
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

import { useEffect, useRef, useState } from 'react';
import { clockStats } from '../audio/clock';
import { Calibrator } from '../audio/calibrator';
import type { Settings } from '../data/store';
import type { Practice } from '../game/practice';
import type { MidiManager } from '../input/midi';
import { CALIBRATION_CLICKS } from '../scoring/calibration';

export type CalSource = 'keyboard' | 'midi' | 'audio';
const LABELS: Record<CalSource, string> = { keyboard: 'Keyboard (F / J)', midi: 'MIDI pads', audio: 'Guitar input' };

/** 16 clicks at 90 BPM per input source; the median offset is stored and subtracted from every hit. */
export function CalibrationSection(props: {
  ctrl: Practice; settings: Settings; sources: CalSource[]; midi?: MidiManager; hint?: string;
}) {
  const { ctrl, settings: s, midi } = props;
  const [cal, setCal] = useState<{ source: CalSource; clicks: number } | null>(null);
  const [msg, setMsg] = useState<Partial<Record<CalSource, string>>>({});
  const calibrator = useRef(new Calibrator());
  useEffect(() => () => calibrator.current.cancel(), []);

  const run = async (source: CalSource) => {
    ctrl.pause();
    if (source === 'midi' && !(await midi?.init())) { setMsg((m) => ({ ...m, midi: 'MIDI is not connected.' })); return; }
    setCal({ source, clicks: 0 });
    setMsg((m) => ({ ...m, [source]: '' }));
    const r = await calibrator.current.run(
      source, ctrl.getSnapshot().settings.clickVolume,
      (n) => setCal({ source, clicks: n }),
    );
    setCal(null);
    const med = r.medianMs;
    if (med === null) {
      setMsg((m) => ({ ...m, [source]: `No usable hits (${r.ignored} ignored). Hit along with the click and try again.` }));
      return;
    }
    const now = ctrl.getSnapshot().settings;
    ctrl.applySettings({ ...now, calibrationMs: { ...now.calibrationMs, [source]: med } });
    const warn = r.used < 8 ? ` Only ${r.used} usable hits: redo it and hit every click.`
      : (r.spreadMs ?? 0) > 20 ? ` Your hits spread ±${r.spreadMs} ms, so the input timing itself is unsteady; calibration can only fix a constant lean.` : '';
    setMsg((m) => ({ ...m, [source]: `Median ${med > 0 ? '+' : ''}${med} ms from ${r.used} hits (${r.ignored} ignored, spread ±${r.spreadMs} ms). Saved.${warn}` }));
  };
  const nudge = (source: CalSource, d: number) => {
    const now = ctrl.getSnapshot().settings;
    ctrl.applySettings({ ...now, calibrationMs: { ...now.calibrationMs, [source]: Math.round((now.calibrationMs[source] + d) * 10) / 10 } });
  };
  const [stats, setStats] = useState(clockStats());
  useEffect(() => { const t = setInterval(() => setStats(clockStats()), 1000); return () => clearInterval(t); }, []);

  return (
    <div className="field">
      <span className="k">Latency calibration: {CALIBRATION_CLICKS} clicks at 90 BPM, hit along with each one{props.hint ? `. ${props.hint}` : ''}</span>
      {props.sources.map((src) => (
        <div className="field" key={src} style={{ gap: 6 }}>
          <div className="learn-row">
            <span className="k">{LABELS[src]}</span>
            <span className="v">{s.calibrationMs[src]} ms</span>
            {cal?.source === src ? (
              <><span className="msg">Hit along… {cal.clicks}/{CALIBRATION_CLICKS}</span>
                <button type="button" className="btn" onClick={() => { calibrator.current.cancel(); setCal(null); }}>Cancel</button></>
            ) : (
              <>
                <button type="button" className="btn" disabled={cal !== null} onClick={() => void run(src)}>
                  {msg[src] ? 'Redo' : 'Calibrate'}
                </button>
                <span className="nudge" role="group" aria-label={`Nudge ${LABELS[src]} calibration`}>
                  {[-5, -1, 1, 5].map((d) => (
                    <button key={d} type="button" onClick={() => nudge(src, d)}>{d > 0 ? `+${d}` : `−${-d}`}</button>
                  ))}
                </span>
              </>
            )}
          </div>
          {msg[src] && <span className="cal-result msg" role="status">{msg[src]}</span>}
        </div>
      ))}
      <span className="msg">
        Positive = your hits came late, so they are moved earlier. Nudge ±1 / ±5 ms to fine-tune.{' '}
        {stats.samples >= 3
          ? `Audio output lag ${Math.round(stats.lagMs)} ms, clock jitter ±${stats.jitterMs.toFixed(1)} ms.`
          : 'Start the click once to measure the audio clock.'}
        {stats.underruns > 0 ? ` The click was scheduled late ${stats.underruns} time(s): the tab was busy.` : ''}
      </span>
    </div>
  );
}

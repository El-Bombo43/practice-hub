import { useMemo, useState } from 'react';
import type { AudioInput } from '../input/audioOnset';
import { detectAll, encodeWav, kFromSensitivity, sensitivityFor } from '../input/onsetCore';

const W = 640;
const H = 90;

/** Record a few seconds of your guitar, see exactly which picks the detector finds, and let it pick the sensitivity. */
export function CapturePanel(props: { audio: AudioInput; sensitivity: number; refractoryMs: number; onSensitivity: (s: number) => void }) {
  const { audio, sensitivity, refractoryMs } = props;
  const [cap, setCap] = useState<{ samples: Float32Array; sampleRate: number } | null>(null);
  const [left, setLeft] = useState<number | null>(null);
  const [played, setPlayed] = useState('');
  const [msg, setMsg] = useState('');

  const onsets = useMemo(
    () => (cap ? detectAll(cap.samples, cap.sampleRate, kFromSensitivity(sensitivity), refractoryMs) : []),
    [cap, sensitivity, refractoryMs],
  );
  const peaks = useMemo(() => {
    if (!cap) return [];
    const per = Math.max(1, Math.floor(cap.samples.length / W));
    return Array.from({ length: W }, (_, i) => {
      let m = 0;
      for (let j = i * per; j < Math.min(cap.samples.length, (i + 1) * per); j++) m = Math.max(m, Math.abs(cap.samples[j]));
      return m;
    });
  }, [cap]);
  const top = Math.max(0.01, ...peaks);
  const dur = cap ? cap.samples.length / cap.sampleRate : 1;

  const record = async () => {
    setMsg('');
    try {
      setLeft(10);
      const c = await audio.capture(10, setLeft);
      setCap(c);
      setMsg(c.samples.length ? '' : 'Nothing was recorded. Is the input connected?');
    } catch (e) {
      setMsg(e instanceof Error ? e.message : 'Recording failed');
    } finally {
      setLeft(null);
    }
  };
  const tune = () => {
    const n = Number(played);
    if (!cap || !Number.isFinite(n) || n < 1) { setMsg('Enter how many picks you played.'); return; }
    setMsg('Working…');
    setTimeout(() => { // let the message paint before the search runs
      const r = sensitivityFor(cap.samples, cap.sampleRate, n, refractoryMs);
      props.onSensitivity(r.sensitivity);
      setMsg(r.count === n ? `Set sensitivity to ${Math.round(r.sensitivity * 100)}%: it finds all ${n} picks.` : `Closest match is ${r.count} detections at ${Math.round(r.sensitivity * 100)}%. Try a cleaner take, or check the channel.`);
    }, 30);
  };
  const download = () => {
    if (!cap) return;
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([encodeWav(cap.samples, cap.sampleRate)], { type: 'audio/wav' }));
    a.download = `guitar-capture-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.wav`;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  return (
    <div className="field">
      <span className="k">Capture and auto-tune</span>
      <span className="msg">Record 10 seconds of normal picking (a steady count, say 12 picks), then tell the app how many you played.</span>
      <div className="row">
        <button type="button" className="btn" disabled={!audio.active || left !== null} onClick={() => void record()}>
          {left === null ? 'Record 10 s' : `Recording… ${Math.ceil(left)}`}
        </button>
        {cap && <button type="button" className="btn" onClick={download}>Download WAV</button>}
      </div>
      {cap && (
        <>
          <svg viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label={`Recording with ${onsets.length} detected picks`} className="chart">
            <rect x="0" y="0" width={W} height={H} fill="var(--bg)" rx="10" />
            {peaks.map((p, i) => <line key={i} x1={i} x2={i} y1={H / 2 - (p / top) * (H / 2 - 4)} y2={H / 2 + (p / top) * (H / 2 - 4)} stroke="var(--grid)" />)}
            {onsets.map((o, i) => <line key={i} x1={(o.time / dur) * W} x2={(o.time / dur) * W} y1="2" y2={H - 2} stroke="var(--accent)" strokeWidth="2" />)}
          </svg>
          <span className="msg" role="status">{onsets.length} pick{onsets.length === 1 ? '' : 's'} detected at sensitivity {Math.round(sensitivity * 100)}%. Lines show where; compare with what you played.</span>
          <div className="row">
            <label className="learn-row"><span className="k">I played</span>
              <input type="number" min={1} max={200} value={played} onChange={(e) => setPlayed(e.target.value)} aria-label="Picks played" style={{ width: 80, minHeight: 44, borderRadius: 12, border: '1px solid var(--off)', background: 'var(--bg)', padding: '0 10px' }} />
              <span className="k">picks</span></label>
            <button type="button" className="btn" onClick={tune}>Find best sensitivity</button>
          </div>
        </>
      )}
      {msg && <span className="msg" role="status">{msg}</span>}
    </div>
  );
}

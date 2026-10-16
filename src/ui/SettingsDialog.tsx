import { useRef, useState } from 'react';
import { ACCENTS, exportJson, parseImport, saveData, type AppData, type Settings } from '../data/store';

export function SettingsDialog(props: {
  settings: Settings;
  getData: () => AppData;
  onChange: (s: Settings) => void;
  onClose: () => void;
  freeze: () => void; // stop autosave before a reload
  onMidi: () => void;
  onAudio: () => void;
}) {
  const { settings: s, onChange } = props;
  const [msg, setMsg] = useState('');
  const file = useRef<HTMLInputElement>(null);

  const doExport = () => {
    const blob = new Blob([exportJson(props.getData())], { type: 'application/json' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `practice-hub-backup-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(a.href);
  };
  const doImport = async (f: File) => {
    try {
      const data = parseImport(await f.text());
      props.freeze();
      await saveData(data);
      location.reload();
    } catch (e) {
      setMsg(`Import failed: ${e instanceof Error ? e.message : 'invalid file'}`);
    }
  };

  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog" role="dialog" aria-label="Settings" onClick={(e) => e.stopPropagation()}>
        <h2>Settings</h2>
        <div className="field">
          <span className="k">Accent color</span>
          <div className="row">
            {ACCENTS.map((c) => (
              <button key={c} type="button" className="swatch" style={{ background: c }} aria-label={`Accent ${c}`}
                aria-pressed={s.accent === c} onClick={() => onChange({ ...s, accent: c })} />
            ))}
          </div>
        </div>
        <div className="field">
          <span className="k">Click volume</span>
          <input type="range" min={0} max={1} step={0.05} value={s.clickVolume} aria-label="Click volume"
            style={{ accentColor: 'var(--accent)' }}
            onChange={(e) => onChange({ ...s, clickVolume: Number(e.target.value) })} />
        </div>
        <div className="field">
          <span className="k">Timing window (± ms)</span>
          <div className="row">
            {([10, 15, 25] as const).map((w) => (
              <button key={w} type="button" className="seg" aria-pressed={s.windowMs === w}
                onClick={() => onChange({ ...s, windowMs: w })}>{w}</button>
            ))}
          </div>
        </div>
        <div className="field">
          <span className="k">Input</span>
          <div className="row"><button type="button" className="btn" onClick={props.onMidi}>MIDI pads &amp; calibration…</button>
            <button type="button" className="btn" onClick={props.onAudio}>Guitar input…</button></div>
        </div>
        <div className="field">
          <span className="k">Backup</span>
          <div className="row">
            <button type="button" className="btn" onClick={doExport}>Export JSON</button>
            <button type="button" className="btn" onClick={() => file.current?.click()}>Import JSON</button>
            <input ref={file} type="file" accept="application/json,.json" hidden
              onChange={(e) => { const f = e.target.files?.[0]; if (f) void doImport(f); e.target.value = ''; }} />
          </div>
          {msg && <span className="msg" role="alert">{msg}</span>}
        </div>
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

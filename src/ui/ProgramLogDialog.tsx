import { useRef, useState } from 'react';
import { exportPracticeLog, mergeLogs, parsePracticeLog, type ProgramLog } from '../data/programLog';

/** Import a practice-log/v1 (from the artifact's "Copy log as JSON") or export this app's log in the same shape. */
export function ProgramLogDialog(props: {
  programId: string; log: ProgramLog; onMerge: (f: (l: ProgramLog) => ProgramLog) => void; onClose: () => void;
}) {
  const [text, setText] = useState('');
  const [msg, setMsg] = useState('');
  const file = useRef<HTMLInputElement>(null);

  const doImport = (raw: string) => {
    try {
      const parsed = parsePracticeLog(raw);
      props.onMerge((l) => mergeLogs(l, parsed));
      setMsg(`Imported ${parsed.sessions.length} session${parsed.sessions.length === 1 ? '' : 's'} and ${parsed.shifts.length} repeat-week shift${parsed.shifts.length === 1 ? '' : 's'}. Newer entries won where both had the same one.`);
      setText('');
    } catch (e) {
      setMsg(`Import failed: ${e instanceof Error ? e.message : 'invalid log'}`);
    }
  };
  const json = () => JSON.stringify(exportPracticeLog(props.log, props.programId), null, 2);
  const download = () => {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([json()], { type: 'application/json' }));
    a.download = `practice-log-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(a.href);
  };
  const copy = async () => {
    try { await navigator.clipboard.writeText(json()); setMsg('Copied the log as JSON.'); } catch { setMsg('Could not copy. Use Download instead.'); }
  };

  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog wide" role="dialog" aria-label="Program log" onClick={(e) => e.stopPropagation()}>
        <h2>Program log</h2>
        <div className="field">
          <span className="k">Import a log (practice-log/v1, from the artifact's “Copy log as JSON”)</span>
          <textarea value={text} placeholder="Paste the JSON here" aria-label="Log JSON" onChange={(e) => setText(e.target.value)} />
          <div className="row">
            <button type="button" className="btn" disabled={!text.trim()} onClick={() => doImport(text)}>Import pasted log</button>
            <button type="button" className="btn" onClick={() => file.current?.click()}>Import from file…</button>
            <input ref={file} type="file" accept="application/json,.json" hidden
              onChange={(e) => { const f = e.target.files?.[0]; if (f) void f.text().then(doImport); e.target.value = ''; }} />
          </div>
        </div>
        <div className="field">
          <span className="k">Export (same format, so the two stay interchangeable)</span>
          <div className="row">
            <button type="button" className="btn" onClick={() => void copy()}>Copy log as JSON</button>
            <button type="button" className="btn" onClick={download}>Download</button>
          </div>
        </div>
        {msg && <span className="msg" role="status">{msg}</span>}
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

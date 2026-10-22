import { useEffect, useMemo, useRef, useState } from 'react';
import { chime } from '../audio/chime';
import {
  addDays, addRepeat, dayFor, hasClick, hasRepeat, offersRepeat, removeRepeat, toHubExercise,
  type Program, type ProgramBlock, type ProgramExercise, type ProgramSession as Session,
} from '../data/program';
import {
  exerciseKey, lastFor, noteKey, readyForNext, setEntry, setSessionFields, tensionTooHigh, testKey, type Entry, type ProgramLog,
} from '../data/programLog';
import { blockAt, chunkAt, fmtClock, itemIndex, testIdFor, totalSeconds } from '../data/programRun';
import type { Practice, Snapshot } from '../game/practice';
import type { AudioInput } from '../input/audioOnset';
import type { MidiManager } from '../input/midi';
import { LeftPanel } from './LeftPanel';
import { Stats } from './Stats';
import { Timeline } from './Timeline';
import { useSessionClock } from './useSessionClock';

export type ProgramSessionProps = {
  program: Program; log: ProgramLog; onLog: (f: (l: ProgramLog) => ProgramLog) => void;
  viewDate: string; today: string; onViewDate: (d: string) => void;
  ctrl: Practice; snap: Snapshot; midi: MidiManager; audio: AudioInput;
  onRemap: () => void; onAudioSetup: () => void; onBuilder: () => void; onImport: () => void;
  active: boolean; registerToggle: (fn: (() => void) | null) => void;
};

const KIND_LABEL: Record<string, string> = {
  warmup: 'Warm-up', lab: 'Motion lab', main: 'Main', secondary: 'Secondary', combo: 'Combo', wrap: 'Log', test: 'Test', break: 'Break', music: 'Music', rest: 'Rest',
};
const num = (v: string): number | undefined => (v.trim() === '' || Number.isNaN(Number(v)) ? undefined : Number(v));

export function ProgramSessionView(props: ProgramSessionProps) {
  const { program, log, viewDate, today } = props;
  const day = dayFor(viewDate, program, log.shifts);
  const nav = (
    <div className="prog-nav">
      <button type="button" className="btn" onClick={() => props.onViewDate(addDays(viewDate, -1))} aria-label="Previous day">‹</button>
      <strong className="mono">{viewDate}</strong>
      <button type="button" className="btn" onClick={() => props.onViewDate(addDays(viewDate, 1))} aria-label="Next day">›</button>
      {viewDate !== today && <button type="button" className="link-btn" onClick={() => props.onViewDate(today)}>Today</button>}
      <span className="grow" />
      <button type="button" className="link-btn muted" onClick={props.onImport}>Import / export log</button>
    </div>
  );
  const shell = (body: React.ReactNode) => (
    <div className="program">
      <section className="panel"><h2 className="section-title">{program.program.title}</h2>{nav}{body}</section>
    </div>
  );
  if (day.state === 'before') return shell(<p className="empty">The program starts on {program.program.kickoff}, in {day.daysUntil} day{day.daysUntil === 1 ? '' : 's'}.</p>);
  if (day.state === 'after') return shell(<p className="empty">The program ended on {program.program.end}. Re-baseline and start the next block.</p>);
  if (!day.session) return shell(<p className="empty">No session is scheduled for this date.</p>);
  return <SessionRun key={viewDate} {...props} session={day.session} programDate={day.programDate} nav={nav} />;
}

function SessionRun(props: ProgramSessionProps & { session: Session; programDate: string; nav: React.ReactNode }) {
  const { program, log, onLog, viewDate, ctrl, snap, session, programDate, active } = props;
  const blocks = session.blocks;
  const clock = useSessionClock(totalSeconds(session), viewDate);
  const blockIdx = blockAt(session, clock.elapsed);
  const block = blocks[blockIdx];
  const [picked, setPicked] = useState(0);
  const itemIdx = itemIndex(block, clock.elapsed, picked);
  const chunk = chunkAt(block, clock.elapsed);
  const item = block.items?.[itemIdx];
  const activeEx: ProgramExercise | undefined = item ? program.exercises[item.ex] : undefined;
  const meta = { programDate, week: session.week };
  const phase = program.program.phases.find((p) => p.n === session.phase);

  const hub = useMemo(() => (activeEx ? toHubExercise(activeEx, Math.max(1, block.end - block.start)) : null), [activeEx, block.end, block.start]);
  const clickOn = clock.running && hasClick(block) && !!hub;

  // keep the controller in step with the session clock: click and scoring only in click blocks
  useEffect(() => {
    if (!active) return;
    if (!clickOn || !hub) { ctrl.pause(); return; }
    if (ctrl.getSnapshot().exercise.id !== hub.id) ctrl.setExercise(hub);
    void ctrl.start();
  }, [active, clickOn, hub, ctrl]);
  useEffect(() => { if (!active) clock.pause(); }, [active]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { setPicked(0); }, [blockIdx]);
  useEffect(() => { // Space and the footswitch drive the session while this screen is showing
    if (!active) return;
    props.registerToggle(() => clock.toggle());
    return () => props.registerToggle(null);
  }, [active, clock.toggle]); // eslint-disable-line react-hooks/exhaustive-deps

  // chimes: two tones at a block change, one tone and a flash at a chunk switch
  const prev = useRef({ b: blockIdx, c: chunk });
  const [flash, setFlash] = useState(0);
  useEffect(() => {
    const p = prev.current;
    prev.current = { b: blockIdx, c: chunk };
    if (!clock.running) return;
    if (p.b !== blockIdx) chime('block');
    else if (p.c !== chunk) { chime('chunk'); setFlash((f) => f + 1); }
  }, [blockIdx, chunk, clock.running]);

  const entry = (key: string): Entry | undefined => log.sessions[viewDate]?.entries[key];
  const update = (key: string, patch: Partial<Entry>) => onLog((l) => setEntry(l, viewDate, meta, key, patch));
  const keyFor = (b: ProgramBlock, i: number): string => {
    const it = b.items?.[i];
    const t = b.log === 'test' ? testIdFor(b, i) : null;
    return t ? testKey(t) : exerciseKey(b.key, it?.ex ?? '', it?.focus);
  };
  const activeKey = block.log === 'tempo' || block.log === 'test' ? keyFor(block, itemIdx) : null;

  // auto-fill CT from the session's clean max, until the player edits it
  const best = snap.live.bestCleanBpm;
  useEffect(() => {
    if (!hub || hub.unscored || !activeKey || !best || snap.exercise.id !== hub.id) return;
    const e = log.sessions[viewDate]?.entries[activeKey];
    if (e?.ct !== undefined && !e.auto) return;
    if (e?.ct !== undefined && best <= e.ct) return;
    onLog((l) => setEntry(l, viewDate, { programDate, week: session.week }, activeKey, { ct: best, auto: true }));
  }, [best, hub, activeKey, snap.exercise.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const total = totalSeconds(session);
  const repeating = hasRepeat(log.shifts, viewDate);

  return (
    <div className="program">
      <section className="panel">
        <div className="prog-head">
          <div>
            <span className="eyebrow">Week {session.week} · {phase ? `Phase ${phase.n}: ${phase.name}` : `Phase ${session.phase}`}</span>
            <h2 className="ex-name" style={{ fontSize: 22 }}>
              {session.type === 'rest' ? 'Rest day' : session.type === 'kickoff' ? 'Kickoff: baseline' : `${session.main ?? 'Session'}${session.secondary ? ` + ${session.secondary}` : ''}`}
            </h2>
            {programDate !== viewDate && <span className="msg">Repeating a week: today replays program day {programDate}.</span>}
          </div>
          {total > 0 && (
            <div className="prog-clock">
              <span className="mono big">{fmtClock(clock.elapsed)} <span className="faded">/ {fmtClock(total)}</span></span>
              <button type="button" className="primary" style={{ flex: '0 0 auto', padding: '0 24px' }} onClick={() => clock.toggle()}>
                {clock.running ? 'Pause' : clock.elapsed > 0 && clock.elapsed < total ? 'Resume' : 'Start session'}
              </button>
              <button type="button" className="btn" onClick={() => clock.seek((blocks[Math.min(blocks.length - 1, blockIdx + 1)].start) * 60)}
                disabled={blockIdx >= blocks.length - 1}>Skip block ›</button>
            </div>
          )}
        </div>
        {props.nav}
        {phase && <p className="msg" style={{ margin: 0 }}>{phase.goal}</p>}
        <p className="msg" style={{ margin: 0 }}>{program.program.safety[0]}</p>
      </section>

      <div className="prog-grid">
        <ol className="blocks" aria-label="Session blocks">
          {blocks.map((b, i) => (
            <li key={b.key + i}>
              <button type="button" className={`blockbtn${i === blockIdx ? ' now' : i < blockIdx ? ' done' : ''}`} onClick={() => clock.seek(b.start * 60)}>
                <span className="mono">{b.start}–{b.end} min</span>
                <span className="t">{b.title}</span>
                <span className="s">{KIND_LABEL[b.kind] ?? b.kind}{hasClick(b) ? ' · click' : ' · no click'}</span>
              </button>
            </li>
          ))}
        </ol>

        <div className="prog-main">
          <section className={`panel blockcard${flash ? ' flash' : ''}`} key={`${block.key}-${flash}`}>
            <span className="eyebrow">{KIND_LABEL[block.kind]} · {block.start}–{block.end} min</span>
            <h3 className="ex-name" style={{ fontSize: 20 }}>{block.title}</h3>
            <p className="blocktext">{block.text}</p>
            {!hasClick(block) && block.kind !== 'rest' && block.kind !== 'wrap' && block.kind !== 'music' && (
              <p className="msg">No click and no scoring in this block.</p>
            )}

            {(block.items ?? []).map((it, i) => {
              const ex = program.exercises[it.ex];
              if (!ex) return null;
              const on = i === itemIdx;
              const key = keyFor(block, i);
              const e = entry(key);
              const lastKey = block.log === 'test' && testIdFor(block, i) ? testKey(testIdFor(block, i) as string) : it.ex;
              const last = lastFor(log, lastKey, viewDate);
              return (
                <article key={i} className={`excard${on ? ' on' : ''}`}>
                  <div className="exhead">
                    <strong>{ex.id} · {ex.name}{it.focus ? ` (${it.focus})` : ''}</strong>
                    {!block.chunks && (block.items?.length ?? 0) > 1 && !on && (
                      <button type="button" className="link-btn" onClick={() => setPicked(i)}>Use this</button>
                    )}
                    {on && block.chunks && <span className="pill">now</span>}
                  </div>
                  <p className="msg">{ex.purpose}</p>
                  <details open={on}>
                    <summary>How, tempo and cue</summary>
                    <p className="msg"><b>How:</b> {ex.how}</p>
                    <p className="msg"><b>Tempo:</b> {ex.tempo}</p>
                    <p className="msg"><b>Cue:</b> {ex.cue}</p>
                    {ex.next && <p className="msg"><b>Next:</b> {ex.next}</p>}
                  </details>
                  {ex.tabs.map((t) => (
                    <div key={t.label}><span className="k">{t.label}</span><pre className="tabbox">{t.tab}</pre></div>
                  ))}
                  <div className="hints">
                    {last && <span className="chip">Last: {last.bpm} bpm ({last.date.slice(5)})</span>}
                    {readyForNext(log, it.ex) && ex.next && <span className="chip ready">3/3 clean on 2 days: ready to move on. {ex.next}</span>}
                    {tensionTooHigh(e) && <span className="chip drop">Tension above 5: stop, shake out, drop the tempo back</span>}
                  </div>
                  {(block.log === 'tempo' || block.log === 'test') && (
                    <EntryForm test={block.log === 'test'} entry={e} onChange={(p) => update(key, p)} />
                  )}
                </article>
              );
            })}

            {block.log === 'note' && (
              <label className="field"><span className="k">Note</span>
                <input type="text" value={entry(noteKey(block.key))?.note ?? ''} aria-label="Note"
                  onChange={(e) => update(noteKey(block.key), { note: e.target.value })} /></label>
            )}
            {block.log === 'session' && (
              <div className="field">
                <label className="k">What felt hardest, or anything to remember
                  <input type="text" value={log.sessions[viewDate]?.note ?? ''} onChange={(e) => onLog((l) => setSessionFields(l, viewDate, meta, { note: e.target.value }))} />
                </label>
                <label className="check"><input type="checkbox" checked={log.sessions[viewDate]?.done === true}
                  onChange={(e) => onLog((l) => setSessionFields(l, viewDate, meta, { done: e.target.checked }))} /> Session done</label>
              </div>
            )}
            {block.kind === 'wrap' && offersRepeat(session) && (
              <div className="reps">
                <span className="label">{repeating ? 'Next week will replay this week.' : 'Missed the targets? Replay this week next week.'}</span>
                <button type="button" className="next-btn" style={{ margin: 0 }}
                  onClick={() => onLog((l) => ({ ...l, shifts: repeating ? removeRepeat(l.shifts, viewDate) : addRepeat(l.shifts, viewDate) }))}>
                  {repeating ? 'Undo repeat' : 'Repeat this week'}
                </button>
              </div>
            )}
          </section>

          {hasClick(block) && hub && (
            hub.unscored ? (
              <div className="main">
                <LeftPanel ctrl={ctrl} snap={snap} nextBtn={null} midi={props.midi} onRemap={props.onRemap} audio={props.audio} onAudioSetup={props.onAudioSetup} onBuilder={props.onBuilder} />
                <section className="panel right"><h2 className="section-title">Click and timer only</h2>
                  <p className="msg">Sweeps and legato aren't scored from audio. Set the click, play, and log your clean tempo and tension above.</p></section>
              </div>
            ) : (
              <div className="main">
                <LeftPanel ctrl={ctrl} snap={snap} nextBtn={null} midi={props.midi} onRemap={props.onRemap} audio={props.audio} onAudioSetup={props.onAudioSetup} onBuilder={props.onBuilder} />
                <div className="right"><Timeline ctrl={ctrl} exercise={snap.exercise} last={snap.lastRep} /><Stats snap={snap} ctrl={ctrl} /></div>
              </div>
            )
          )}
        </div>
      </div>
    </div>
  );
}

function EntryForm(props: { test: boolean; entry: Entry | undefined; onChange: (p: Partial<Entry>) => void }) {
  const { entry: e, onChange } = props;
  return (
    <div className="entry">
      <label>{props.test ? 'Clean tempo (CT)' : 'Clean bpm'}
        <input type="number" inputMode="numeric" min={0} max={400} value={e?.ct ?? ''} onChange={(ev) => onChange({ ct: num(ev.target.value), auto: false })} /></label>
      {props.test && (
        <label>Motion ceiling (MC)
          <input type="number" inputMode="numeric" min={0} max={500} value={e?.mc ?? ''} onChange={(ev) => onChange({ mc: num(ev.target.value) })} /></label>
      )}
      <label>Tension /10
        <input type="number" inputMode="numeric" min={0} max={10} value={e?.tension ?? ''} onChange={(ev) => onChange({ tension: num(ev.target.value) })} /></label>
      {props.test ? (
        <label className="check"><input type="checkbox" checked={e?.video === true} onChange={(ev) => onChange({ video: ev.target.checked })} /> Video</label>
      ) : (
        <label className="check"><input type="checkbox" checked={e?.clean3 === true} onChange={(ev) => onChange({ clean3: ev.target.checked })} /> 3/3 clean</label>
      )}
      <label className="wide">Note
        <input type="text" value={e?.note ?? ''} onChange={(ev) => onChange({ note: ev.target.value })} /></label>
    </div>
  );
}


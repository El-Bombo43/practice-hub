import { useState } from 'react';
import { EXERCISES, exerciseById } from '../data/exercises';
import { newUid, type Routine, type RoutineItem } from '../data/routine';

export function RoutineBar(props: {
  routine: Routine;
  onSelect: (i: number) => void;
  onChange: (items: RoutineItem[], step: number) => void;
}) {
  const { routine, onSelect, onChange } = props;
  const [editing, setEditing] = useState(false);
  const [adding, setAdding] = useState(false);
  const { items, step } = routine;

  const move = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= items.length) return;
    const next = [...items];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next, step === i ? j : step === j ? i : step);
  };
  const remove = (i: number) => {
    if (items.length <= 1) return;
    onChange(items.filter((_, k) => k !== i), i < step ? step - 1 : step);
  };
  const swap = (i: number, exerciseId: string) =>
    onChange(items.map((it, k) => (k === i ? { ...it, exerciseId, burst: undefined } : it)), step);
  const add = (exerciseId: string) => {
    onChange([...items, { uid: newUid(), exerciseId, minutes: exerciseById(exerciseId).minutes }], step);
    setAdding(false);
  };

  return (
    <section>
      <div className="routine-head">
        <h2 className="section-title">Today's routine</h2>
        <div>
          <button type="button" className="link-btn muted" onClick={() => setEditing(!editing)}>{editing ? 'Done editing' : 'Edit'}</button>
          <button type="button" className="link-btn" onClick={() => setAdding(!adding)}>+ Add exercise</button>
        </div>
      </div>
      {adding && (
        <div className="add-list">
          {EXERCISES.map((e) => (
            <button type="button" key={e.id} onClick={() => add(e.id)}>
              {e.name} <span className="mono" style={{ color: 'var(--muted)' }}>· {e.instrument === 'guitar' ? 'Guitar' : 'Pads'} · {e.minutes}m</span>
            </button>
          ))}
        </div>
      )}
      <ol className="cards">
        {items.map((it, i) => {
          const ex = exerciseById(it.exerciseId);
          const state = i < step ? 'done' : i === step ? 'now' : '';
          const tag = i < step ? 'done' : i === step ? 'now' : it.burst ? 'boss round' : '';
          const title = it.burst ? 'Burst round' : ex.name;
          const body = (
            <>
              <span className="n">{String(i + 1).padStart(2, '0')}{tag ? ` · ${tag}` : ''}</span>
              <span className="t">{title}</span>
              <span className="s">{ex.instrument === 'guitar' ? 'Guitar' : 'Pads'} · {it.minutes}m</span>
            </>
          );
          return (
            <li key={it.uid}>
              {editing ? (
                <div className={`card ${state}`}>
                  {body}
                  <div className="card-tools">
                    <button type="button" onClick={() => move(i, -1)} disabled={i === 0} aria-label="Move up">↑</button>
                    <button type="button" onClick={() => move(i, 1)} disabled={i === items.length - 1} aria-label="Move down">↓</button>
                    <button type="button" onClick={() => remove(i)} disabled={items.length <= 1} aria-label="Remove">✕</button>
                    <select value={it.exerciseId} aria-label="Swap exercise" onChange={(e) => swap(i, e.target.value)}>
                      {EXERCISES.filter((e) => e.instrument === ex.instrument).map((e) => (
                        <option key={e.id} value={e.id}>{e.name}</option>
                      ))}
                    </select>
                  </div>
                </div>
              ) : (
                <button type="button" className={`card ${state}`} onClick={() => onSelect(i)}>{body}</button>
              )}
            </li>
          );
        })}
      </ol>
    </section>
  );
}

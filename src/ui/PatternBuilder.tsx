import { useState } from 'react';
import { blankCells, cycleCell, hasStrike, newCustomId, type CustomPattern } from '../data/custom';
import { countLabel } from './StrumTimeline';

const SYMBOL: Record<string, string> = { '-': '·', x: '', X: '', g: '' };

export function PatternBuilder(props: {
  saved: CustomPattern[];
  onSave: (p: CustomPattern) => void;
  onDelete: (id: string) => void;
  onClose: () => void;
}) {
  const [id, setId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [sub, setSub] = useState<2 | 4>(4);
  const [cells, setCells] = useState(blankCells(4));
  const canSave = name.trim().length > 0 && hasStrike(cells);

  const chooseSub = (n: 2 | 4) => { if (n !== sub) { setSub(n); setCells(blankCells(n)); } };
  const load = (p: CustomPattern) => { setId(p.id); setName(p.name); setSub(p.subdivision); setCells(p.cells); };
  const reset = () => { setId(null); setName(''); setCells(blankCells(sub)); };
  const save = () => {
    props.onSave({ id: id ?? newCustomId(), name: name.trim(), subdivision: sub, cells });
    reset();
  };

  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog wide" role="dialog" aria-label="Pattern builder" onClick={(e) => e.stopPropagation()}>
        <h2>Pattern builder</h2>
        <span className="msg">One bar of 4/4. Click a slot to cycle: rest → strike → accent → ghost. Direction follows the slot: odd = down, even = up.</span>

        <div className="field">
          <span className="k">Name</span>
          <input type="text" value={name} maxLength={40} placeholder="e.g. Folk strum" aria-label="Pattern name"
            onChange={(e) => setName(e.target.value)} />
        </div>
        <div className="field">
          <span className="k">Count</span>
          <div className="row">
            {([2, 4] as const).map((n) => (
              <button key={n} type="button" className="seg" aria-pressed={sub === n} onClick={() => chooseSub(n)}>{n === 2 ? '8ths' : '16ths'}</button>
            ))}
          </div>
        </div>
        <div className="cells" role="group" aria-label="Pattern slots">
          {[...cells].map((c, i) => (
            <button key={i} type="button" className={`cell-btn ${c === '-' ? 'dash' : c}${i % sub === 0 ? ' beat' : ''}`}
              aria-label={`Slot ${i + 1}: ${{ '-': 'rest', x: 'strike', X: 'accent', g: 'ghost' }[c]}`}
              onClick={() => setCells(cycleCell(cells, i))}>
              <span className="m">{c === 'X' ? '>' : ''}</span>
              <span className="g">{c === '-' ? SYMBOL['-'] : i % 2 === 0 ? '↓' : '↑'}</span>
              <span className="c">{countLabel(i, sub)}</span>
            </button>
          ))}
        </div>
        <div className="row">
          <button type="button" className="btn" disabled={!canSave} onClick={save}>{id ? 'Update pattern' : 'Save pattern'}</button>
          {(id || cells !== blankCells(sub)) && <button type="button" className="btn" onClick={reset}>Clear</button>}
          {!hasStrike(cells) && <span className="msg">Add at least one strike.</span>}
        </div>

        <hr />
        <div className="field">
          <span className="k">Saved patterns</span>
          {props.saved.length === 0 ? (
            <span className="empty">None yet. Saved patterns show up in the exercise library and can be added to your routine.</span>
          ) : (
            <div className="saved">
              {props.saved.map((p) => (
                <div key={p.id}>
                  <span className="name">{p.name} <span className="mono" style={{ color: 'var(--muted)' }}>· {p.subdivision === 2 ? '8ths' : '16ths'}</span></span>
                  <button type="button" className="btn" onClick={() => load(p)}>Edit</button>
                  <button type="button" className="btn" onClick={() => { if (id === p.id) reset(); props.onDelete(p.id); }}>Delete</button>
                </div>
              ))}
            </div>
          )}
        </div>
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

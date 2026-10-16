import { useEffect } from 'react';
import type { Settings } from '../data/store';
import type { Practice } from '../game/practice';
import { midiSupported, type LearnKind, type MidiManager, type MidiMapping } from '../input/midi';
import { CalibrationSection } from './CalibrationSection';
import { useMidi } from './useMidi';

const LEARN_ROWS: { kind: LearnKind; label: string; field: keyof MidiMapping; hit: string; optional?: boolean }[] = [
  { kind: 'left', label: 'Left hand pad', field: 'left', hit: 'Hit the pad for LEFT hand' },
  { kind: 'right', label: 'Right hand pad', field: 'right', hit: 'Hit the pad for RIGHT hand' },
  { kind: 'ccToggle', label: 'Footswitch: Start / Pause', field: 'ccToggle', hit: 'Press the footswitch', optional: true },
  { kind: 'ccNext', label: 'Footswitch: Next exercise', field: 'ccNext', hit: 'Press the other switch', optional: true },
];

export function MidiDialog(props: { ctrl: Practice; midi: MidiManager; settings: Settings; onClose: () => void }) {
  const { ctrl, settings: s } = props;
  const midi = useMidi(props.midi);
  const learning = midi.learningKind;
  useEffect(() => () => { midi.cancelLearn(); }, [midi]);

  const setMidi = (patch: Partial<MidiMapping>) => {
    const cur = ctrl.getSnapshot().settings;
    ctrl.applySettings({ ...cur, midi: { ...cur.midi, ...patch } });
  };
  const learn = async (row: (typeof LEARN_ROWS)[number]) => {
    const n = await midi.learn(row.kind);
    setMidi({ [row.field]: n });
  };
  const devices = midi.devices();
  return (
    <div className="overlay" onClick={props.onClose}>
      <div className="dialog" role="dialog" aria-label="MIDI and calibration" onClick={(e) => e.stopPropagation()}>
        <h2>MIDI pads &amp; calibration</h2>

        <div className="field">
          <span className="k">MIDI device (pads)</span>
          {!midiSupported() ? (
            <span className="msg">Web MIDI isn't available in this browser. Use Chrome over HTTPS.</span>
          ) : !midi.connected ? (
            <div className="row"><button type="button" className="btn" onClick={() => void midi.init()}>Connect MIDI</button></div>
          ) : (
            <select value={s.midi.deviceId ?? ''} aria-label="MIDI device"
              onChange={(e) => setMidi({ deviceId: e.target.value || null })}>
              <option value="">{devices.length ? 'Any device' : 'No devices found'}</option>
              {devices.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
            </select>
          )}
          {midi.error && <span className="msg" role="alert">{midi.error}</span>}
        </div>

        <div className="field">
          <span className="k">Pad mapping and footswitch (learn mode)</span>
          {LEARN_ROWS.map((row) => {
            const val = s.midi[row.field] as number | null;
            const active = learning === row.kind;
            return (
              <div className="learn-row" key={row.kind}>
                <span className="k">{row.label}</span>
                <span className="v">{active ? '…' : val === null ? 'not set' : row.field === 'left' || row.field === 'right' ? `note ${val}` : `CC ${val}`}</span>
                {active ? (
                  <><span className="msg">{row.hit}</span><button type="button" className="btn" onClick={() => midi.cancelLearn()}>Cancel</button></>
                ) : (
                  <>
                    <button type="button" className="btn" disabled={!midi.connected || learning !== null} onClick={() => void learn(row)}>Learn</button>
                    {row.optional && val !== null && <button type="button" className="btn" onClick={() => setMidi({ [row.field]: null })}>Clear</button>}
                  </>
                )}
              </div>
            );
          })}
          <span className="msg">Footswitch acts on press only (CC value ≥ 64). It works from any connected MIDI device.</span>
        </div>

        <hr />
        <CalibrationSection ctrl={ctrl} settings={s} midi={props.midi} sources={['keyboard', 'midi']} />
        <div className="row"><button type="button" className="btn" onClick={props.onClose}>Close</button></div>
      </div>
    </div>
  );
}

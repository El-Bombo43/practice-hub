import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { exerciseById } from '../data/exercises';
import { defaultRoutine, type Routine, type RoutineItem } from '../data/routine';
import { loadData, saveData, type AppData } from '../data/store';
import { Practice } from '../game/practice';
import { dateKey, streak } from '../game/progress';
import { inputBus } from '../input/bus';
import { attachKeyboard } from '../input/keyboard';
import { MidiManager } from '../input/midi';
import { LeftPanel } from './LeftPanel';
import { MidiDialog } from './MidiDialog';
import { RoutineBar } from './RoutineBar';
import { SettingsDialog } from './SettingsDialog';
import { Stats } from './Stats';
import { Timeline } from './Timeline';
import { TopBar } from './TopBar';

export function App() {
  const [data, setData] = useState<AppData | null>(null);
  useEffect(() => { void loadData().then(setData); }, []);
  return data ? <Loaded initial={data} /> : <div className="app" />;
}

const hexToRgba = (hex: string, a: number): string => {
  const n = parseInt(hex.slice(1), 16);
  return `rgba(${n >> 16}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
};

function Loaded({ initial }: { initial: AppData }) {
  const [routine, setRoutine] = useState<Routine>(() => {
    const r = initial.routine ?? defaultRoutine();
    const step = r.stepDate === dateKey() ? Math.min(r.step, r.items.length - 1) : 0;
    return { ...r, step };
  });
  const [ctrl] = useState(() => {
    const it = routine.items[routine.step];
    return new Practice(initial.profile, initial.settings, exerciseById(it.exerciseId), !!it.burst);
  });
  const snap = useSyncExternalStore(ctrl.subscribe, ctrl.getSnapshot);
  const [dialog, setDialog] = useState<null | 'settings' | 'midi'>(null);
  const [midi] = useState(() => new MidiManager());
  const nextRef = useRef<() => void>(() => {});
  const frozen = useRef(false);
  const latest = useRef<AppData>(initial);
  latest.current = { settings: snap.settings, profile: snap.profile, routine };

  useEffect(() => attachKeyboard({
    instrument: () => ctrl.getSnapshot().exercise.instrument,
    onHit: (e) => inputBus.emit(e),
    onToggle: () => ctrl.toggle(),
  }), [ctrl]);
  useEffect(() => () => ctrl.dispose(), [ctrl]);

  // Every input source (keyboard, MIDI) arrives on the bus.
  useEffect(() => inputBus.subscribe((e) => ctrl.hit(e)), [ctrl]);
  useEffect(() => {
    midi.onAction = (a) => { if (a === 'toggle') ctrl.toggle(); else nextRef.current(); };
    midi.onDeviceChosen = (id) => {
      const cur = ctrl.getSnapshot().settings;
      if (cur.midi.deviceId !== id) ctrl.applySettings({ ...cur, midi: { ...cur.midi, deviceId: id } });
    };
    ctrl.onBarline = () => { if (midi.connected) midi.refreshClock(); };
    if (initial.settings.midi.deviceId) void midi.init(); // reconnect the remembered device
  }, [ctrl, midi, initial.settings.midi.deviceId]);
  useEffect(() => { midi.setMapping(snap.settings.midi); }, [midi, snap.settings.midi]);

  useEffect(() => {
    const r = document.documentElement.style;
    r.setProperty('--accent', snap.settings.accent);
    r.setProperty('--accent-glow', hexToRgba(snap.settings.accent, 0.5));
  }, [snap.settings.accent]);

  // Autosave (debounced) + best-effort flush when the page is hidden.
  useEffect(() => {
    const t = setTimeout(() => { if (!frozen.current) void saveData(latest.current); }, 800);
    return () => clearTimeout(t);
  }, [snap.profile, snap.settings, routine]);
  useEffect(() => {
    const flush = () => { if (!frozen.current) void saveData(latest.current); };
    window.addEventListener('pagehide', flush);
    document.addEventListener('visibilitychange', flush);
    return () => {
      window.removeEventListener('pagehide', flush);
      document.removeEventListener('visibilitychange', flush);
    };
  }, []);

  const startItem = (it: RoutineItem) => ctrl.setExercise(exerciseById(it.exerciseId), !!it.burst);

  const commit = (items: RoutineItem[], step: number) => {
    const s = Math.max(0, Math.min(items.length - 1, step));
    const prev = routine.items[routine.step];
    const next = items[s];
    setRoutine({ items, step: s, stepDate: dateKey() });
    if (!prev || prev.uid !== next.uid || prev.exerciseId !== next.exerciseId) startItem(next);
  };
  const select = (i: number) => {
    if (i === routine.step) return;
    setRoutine({ ...routine, step: i, stepDate: dateKey() });
    startItem(routine.items[i]);
  };

  nextRef.current = () => { if (routine.step < routine.items.length - 1) select(routine.step + 1); };

  const cur = routine.items[routine.step];
  const stepSec = cur.minutes * 60;
  const timeUp = snap.elapsedSec >= stepSec;
  const isLast = routine.step === routine.items.length - 1;
  const restMin = routine.items.slice(routine.step + 1).reduce((s, i) => s + i.minutes, 0);
  const minLeft = Math.ceil(Math.max(0, stepSec - snap.elapsedSec) / 60) + restMin;

  const nextBtn = timeUp ? (
    isLast ? (
      <div className="toast" role="status">Routine complete. Keep playing or pick a step.</div>
    ) : (
      <button type="button" className="next-btn" onClick={() => select(routine.step + 1)}>Time's up · Next →</button>
    )
  ) : null;

  return (
    <div className="app">
      <div className="wrap">
        <TopBar
          xp={snap.profile.xp}
          streak={streak(snap.profile.dayMinutes)}
          step={routine.step + 1}
          steps={routine.items.length}
          minLeft={minLeft}
          onSettings={() => setDialog('settings')}
        />
        <div className="main">
          <LeftPanel ctrl={ctrl} snap={snap} nextBtn={nextBtn} midi={midi} onRemap={() => setDialog('midi')} />
          <div className="right">
            <Timeline ctrl={ctrl} exercise={snap.exercise} last={snap.lastRep} />
            <Stats snap={snap} ctrl={ctrl} />
          </div>
        </div>
        <RoutineBar routine={routine} onSelect={select} onChange={commit} />
      </div>
      {dialog === 'settings' && (
        <SettingsDialog
          settings={snap.settings}
          getData={() => latest.current}
          onChange={(s) => ctrl.applySettings(s)}
          onClose={() => setDialog(null)}
          freeze={() => { frozen.current = true; }}
          onMidi={() => setDialog('midi')}
        />
      )}
      {dialog === 'midi' && <MidiDialog ctrl={ctrl} midi={midi} settings={snap.settings} onClose={() => setDialog(null)} />}
    </div>
  );
}

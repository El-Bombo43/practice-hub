import { get, set } from 'idb-keyval';
import { defaultRoutine, type Routine } from './routine';
import { emptyProfile, type Profile } from '../game/progress';
import { parseCustom, type CustomPattern } from './custom';
import { emptyProgramLog, type ProgramLog } from './programLog';
import { defaultMidiMapping, type MidiMapping } from '../input/midi';

export type Settings = {
  accent: string;
  clickVolume: number; // 0-1
  windowMs: 10 | 15 | 25;
  gapClick: boolean;
  calibrationMs: { keyboard: number; midi: number; audio: number };
  midi: MidiMapping;
  audio: { deviceId: string | null; sensitivity: number }; // guitar DI input; sensitivity 0–1
};

export const ACCENTS = ['#d4ff3a', '#3ee6d4', '#ff4fd8'];

export const defaultSettings = (): Settings => ({
  accent: ACCENTS[0], clickVolume: 0.6, windowMs: 15, gapClick: false,
  calibrationMs: { keyboard: 0, midi: 0, audio: 0 },
  midi: defaultMidiMapping(),
  audio: { deviceId: null, sensitivity: 0.6 },
});

export type AppData = { settings: Settings; profile: Profile; routine: Routine; custom: CustomPattern[]; program: ProgramLog };

const KEY = 'practice-hub/v1';

/** Merge loaded data over defaults so older/partial saves keep working. */
export function normalize(raw: unknown): AppData {
  const r = (raw && typeof raw === 'object' ? raw : {}) as Partial<AppData>;
  const settings = { ...defaultSettings(), ...r.settings };
  settings.calibrationMs = { ...defaultSettings().calibrationMs, ...r.settings?.calibrationMs };
  settings.midi = { ...defaultMidiMapping(), ...r.settings?.midi };
  settings.audio = { ...defaultSettings().audio, ...r.settings?.audio };
  const profile = { ...emptyProfile(), ...r.profile };
  const routine = r.routine && Array.isArray(r.routine.items) && r.routine.items.length
    ? { ...defaultRoutine(), ...r.routine }
    : defaultRoutine();
  const pl = (r.program ?? {}) as Partial<ProgramLog>;
  const program: ProgramLog = {
    shifts: Array.isArray(pl.shifts) ? pl.shifts : emptyProgramLog().shifts,
    sessions: pl.sessions && typeof pl.sessions === 'object' ? pl.sessions : {},
  };
  return { settings, profile, routine, custom: parseCustom(r.custom), program };
}

export async function loadData(): Promise<AppData> {
  try {
    return normalize(await get(KEY));
  } catch {
    return normalize(null);
  }
}

export async function saveData(data: AppData): Promise<void> {
  try {
    await set(KEY, data);
  } catch {
    /* storage unavailable (private mode): keep running in memory */
  }
}

export const exportJson = (data: AppData): string =>
  JSON.stringify({ app: 'practice-hub', version: 1, ...data }, null, 2);

export function parseImport(text: string): AppData {
  const raw = JSON.parse(text);
  if (!raw || typeof raw !== 'object' || (!raw.profile && !raw.settings && !raw.routine)) {
    throw new Error('Not a Practice Hub backup');
  }
  return normalize(raw);
}

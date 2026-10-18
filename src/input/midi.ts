import { heardTimeOfPerf } from '../audio/clock';
import type { Lane } from '../data/types';
import type { InputEvent } from './types';
import { inputBus } from './bus';

export type MidiMapping = {
  deviceId: string | null; // pad device (note-ons); CCs are accepted from any device
  left: number; // note numbers
  right: number;
  ccToggle: number | null; // footswitch CC → Start/Pause
  ccNext: number | null; // footswitch CC → Next exercise
};
export const defaultMidiMapping = (): MidiMapping => ({ deviceId: null, left: 36, right: 37, ccToggle: null, ccNext: null });

export type MidiMessage =
  | { type: 'noteon'; note: number; velocity: number } // velocity 1-127
  | { type: 'cc'; cc: number; value: number };

/** Parse raw MIDI bytes. Note-on with velocity 0 is a note-off and yields null. */
export function parseMidi(data: ArrayLike<number>): MidiMessage | null {
  if (data.length < 3) return null;
  const kind = data[0] & 0xf0;
  if (kind === 0x90 && data[2] > 0) return { type: 'noteon', note: data[1], velocity: data[2] };
  if (kind === 0xb0) return { type: 'cc', cc: data[1], value: data[2] };
  return null;
}

/** Pad hit → lane, or null for a note that isn't mapped. */
export function laneForNote(note: number, map: Pick<MidiMapping, 'left' | 'right'>): Lane | null {
  return note === map.left ? 'L' : note === map.right ? 'R' : null;
}

export type MidiDevice = { id: string; name: string };
export type LearnKind = 'left' | 'right' | 'ccToggle' | 'ccNext';
export type MidiAction = 'toggle' | 'next';

export const midiSupported = (): boolean => typeof navigator !== 'undefined' && 'requestMIDIAccess' in navigator;

/** Web MIDI input: device list, pad hits → InputEvents on the bus, footswitch CCs → actions, learn mode. */
export class MidiManager {
  private access: MIDIAccess | null = null;
  private mapping: MidiMapping = defaultMidiMapping();
  private learning: { kind: LearnKind; resolve: (n: number) => void } | null = null;
  private listeners = new Set<() => void>();
  error: string | null = null;
  onAction: (a: MidiAction) => void = () => {};
  onDeviceChosen: (id: string) => void = () => {};

  subscribe = (l: () => void): (() => void) => {
    this.listeners.add(l);
    return () => { this.listeners.delete(l); };
  };
  private emit(): void { this.listeners.forEach((l) => l()); }

  get connected(): boolean { return this.access !== null; }

  async init(): Promise<boolean> {
    if (this.access) return true;
    if (!midiSupported()) { this.error = 'Web MIDI is not available in this browser.'; this.emit(); return false; }
    try {
      this.access = await navigator.requestMIDIAccess();
    } catch {
      this.error = 'MIDI access was blocked. Allow it in the site permissions and retry.';
      this.emit();
      return false;
    }
    this.error = null;
    this.access.onstatechange = () => { this.attach(); this.emit(); };
    this.attach();
    this.emit();
    return true;
  }

  devices(): MidiDevice[] {
    if (!this.access) return [];
    return [...this.access.inputs.values()].map((i) => ({ id: i.id, name: i.name || i.id }));
  }

  setMapping(m: MidiMapping): void {
    const changed = m.deviceId !== this.mapping.deviceId;
    this.mapping = m;
    if (changed) this.emit();
  }

  /** Resolve with the next note number (pads) or CC number (footswitch). */
  learn(kind: LearnKind): Promise<number> {
    return new Promise((resolve) => {
      this.learning = { kind, resolve };
      this.emit();
    });
  }
  cancelLearn(): void { this.learning = null; this.emit(); }
  get learningKind(): LearnKind | null { return this.learning?.kind ?? null; }

  private attach(): void {
    if (!this.access) return;
    for (const input of this.access.inputs.values()) {
      input.onmidimessage = (e: MIDIMessageEvent) => this.handle(input.id, e);
    }
    if (!this.mapping.deviceId) {
      const first = [...this.access.inputs.values()].find((i) => i.state === 'connected');
      if (first && this.access.inputs.size === 1) this.onDeviceChosen(first.id);
    }
  }

  private handle(deviceId: string, e: MIDIMessageEvent): void {
    if (!e.data) return;
    const msg = parseMidi(e.data);
    if (!msg) return;
    const l = this.learning;
    const isLearnNote = l && (l.kind === 'left' || l.kind === 'right');
    if (l && ((msg.type === 'noteon' && isLearnNote) || (msg.type === 'cc' && !isLearnNote && msg.value >= 64))) {
      this.learning = null;
      if (msg.type === 'noteon') this.onDeviceChosen(deviceId);
      l.resolve(msg.type === 'noteon' ? msg.note : msg.cc);
      this.emit();
      return;
    }
    if (msg.type === 'cc') {
      if (msg.value < 64) return; // footswitch: act on press only
      if (msg.cc === this.mapping.ccToggle) this.onAction('toggle');
      else if (msg.cc === this.mapping.ccNext) this.onAction('next');
      return;
    }
    if (this.mapping.deviceId && deviceId !== this.mapping.deviceId) return;
    const lane = laneForNote(msg.note, this.mapping);
    if (!lane) return;
    const ev: InputEvent = {
      time: heardTimeOfPerf(e.timeStamp), // smoothed performance↔audio mapping (audio/clock.ts)
      source: 'midi', lane, velocity: msg.velocity / 127,
    };
    inputBus.emit(ev);
  }
}

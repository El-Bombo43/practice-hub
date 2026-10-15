import type { InputEvent } from './types';

type Listener = (e: InputEvent) => void;
const listeners = new Set<Listener>();

/** All input sources (keyboard, MIDI, later audio) publish here; scoring and calibration subscribe. */
export const inputBus = {
  emit: (e: InputEvent): void => listeners.forEach((l) => l(e)),
  subscribe: (l: Listener): (() => void) => {
    listeners.add(l);
    return () => { listeners.delete(l); };
  },
};

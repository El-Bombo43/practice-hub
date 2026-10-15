import { useEffect, useReducer } from 'react';
import type { MidiManager } from '../input/midi';

/** Re-render when the MIDI manager's devices, error or learn state change. */
export function useMidi(midi: MidiManager): MidiManager {
  const [, bump] = useReducer((n: number) => n + 1, 0);
  useEffect(() => midi.subscribe(bump), [midi]);
  return midi;
}

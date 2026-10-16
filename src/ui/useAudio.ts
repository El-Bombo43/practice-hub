import { useEffect, useReducer } from 'react';
import type { AudioInput } from '../input/audioOnset';

/** Re-render when the audio input starts, stops, errors or locks a channel. */
export function useAudio(audio: AudioInput): AudioInput {
  const [, bump] = useReducer((n: number) => n + 1, 0);
  useEffect(() => audio.subscribe(bump), [audio]);
  return audio;
}

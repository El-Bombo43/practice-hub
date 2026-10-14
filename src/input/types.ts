import type { Lane } from '../data/types';

export type InputEvent = {
  time: number; // audio-clock seconds
  source: 'keyboard' | 'midi' | 'audio';
  lane?: Lane;
  velocity: number; // 0-1
};

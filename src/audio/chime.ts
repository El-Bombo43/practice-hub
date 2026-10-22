import { getCtx } from './clock';

function tone(freq: number, at: number, dur = 0.16, level = 0.18): void {
  const ctx = getCtx();
  const osc = ctx.createOscillator();
  const g = ctx.createGain();
  osc.type = 'sine';
  osc.frequency.value = freq;
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(level, at + 0.01);
  g.gain.exponentialRampToValueAtTime(0.0001, at + dur);
  osc.connect(g).connect(ctx.destination);
  osc.start(at);
  osc.stop(at + dur + 0.02);
}

/** Two rising tones at a block change, one tone at a chunk switch. Softer and higher than the click, so they don't read as beats. */
export function chime(kind: 'block' | 'chunk'): void {
  const t = getCtx().currentTime + 0.02;
  if (kind === 'block') { tone(659.3, t); tone(880, t + 0.2); } else tone(784, t, 0.2);
}

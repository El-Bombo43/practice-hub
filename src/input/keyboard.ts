import { heardTimeOfPerf } from '../audio/clock';
import type { Lane } from '../data/types';
import type { InputEvent } from './types';

/**
 * Keyboard simulator, always on: F = left / downstroke, J = right / upstroke (hold Shift for an accent), Space = start/pause.
 * Lanes are informational in session 1.
 */
export function attachKeyboard(opts: {
  instrument: () => 'guitar' | 'pads';
  onHit: (e: InputEvent) => void;
  onToggle: () => void;
}): () => void {
  const typing = (t: EventTarget | null) =>
    t instanceof HTMLElement && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT');

  const down = (e: KeyboardEvent) => {
    if (typing(e.target) || e.ctrlKey || e.metaKey || e.altKey) return;
    if (e.code === 'Space') {
      e.preventDefault();
      if (!e.repeat) opts.onToggle();
      return;
    }
    if (e.repeat) return;
    const key = e.key.toLowerCase();
    if (key !== 'f' && key !== 'j') return;
    const pads = opts.instrument() === 'pads';
    const lane: Lane = key === 'f' ? (pads ? 'L' : 'down') : pads ? 'R' : 'up';
    // Shift = a harder hit (accents in strum patterns); 0.8 otherwise
    // event.timeStamp is when the key went down, not when this handler got to run
    opts.onHit({ time: heardTimeOfPerf(e.timeStamp), source: 'keyboard', lane, velocity: e.shiftKey ? 1 : 0.8 });
  };
  // Stop a focused button from also "clicking" on Space keyup.
  const up = (e: KeyboardEvent) => { if (e.code === 'Space' && !typing(e.target)) e.preventDefault(); };

  window.addEventListener('keydown', down);
  window.addEventListener('keyup', up);
  return () => {
    window.removeEventListener('keydown', down);
    window.removeEventListener('keyup', up);
  };
}

import { useEffect, useRef } from 'react';
import type { Practice } from '../game/practice';

/**
 * Drives the faint playhead that sweeps one bar per cycle in step with the click. Returns the ref
 * for the playhead element; it's moved off-React once per frame. Slot i sits at its column centre.
 */
export function usePlayhead(ctrl: Practice, cols: number) {
  const head = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let raf = 0;
    const loop = () => {
      const el = head.current;
      if (el) {
        const p = ctrl.position();
        const x = (p * cols + 0.5) / cols;
        el.style.visibility = p < 0 || x > 1 ? 'hidden' : 'visible';
        el.style.left = `${x * 100}%`;
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [ctrl, cols]);
  return head;
}

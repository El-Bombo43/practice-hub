import { useCallback, useEffect, useRef, useState } from 'react';

/** One timer for a whole session (seconds). Stops itself at `totalSec`. */
export function useSessionClock(totalSec: number, resetKey: string) {
  const [elapsed, setElapsed] = useState(0);
  const [running, setRunning] = useState(false);
  const last = useRef(0);

  useEffect(() => { setElapsed(0); setRunning(false); }, [resetKey]);
  useEffect(() => {
    if (!running) return;
    last.current = performance.now();
    const t = setInterval(() => {
      const now = performance.now();
      const dt = (now - last.current) / 1000;
      last.current = now;
      setElapsed((e) => {
        const next = Math.min(totalSec, e + dt);
        if (next >= totalSec) setRunning(false);
        return next;
      });
    }, 250);
    return () => clearInterval(t);
  }, [running, totalSec]);

  const start = useCallback(() => { setElapsed((e) => (e >= totalSec ? 0 : e)); setRunning(true); }, [totalSec]);
  const pause = useCallback(() => setRunning(false), []);
  const toggle = useCallback(() => setRunning((r) => { if (!r) setElapsed((e) => (e >= totalSec ? 0 : e)); return !r; }), [totalSec]);
  const seek = useCallback((sec: number) => setElapsed(Math.max(0, Math.min(totalSec, sec))), [totalSec]);
  return { elapsed, running, start, pause, toggle, seek };
}

let ctx: AudioContext | null = null;

/** Lazily create the AudioContext. Call from a user gesture (Start / Space) so it can run. */
export function getCtx(): AudioContext {
  if (!ctx) ctx = new AudioContext({ latencyHint: 'interactive' });
  return ctx;
}

export async function resumeCtx(): Promise<AudioContext> {
  const c = getCtx();
  if (c.state !== 'running') await c.resume();
  return c;
}

/**
 * "Heard" audio-clock time in seconds: currentTime minus output latency, so a key pressed in sync
 * with an audible click compares against that click's scheduled time.
 */
export function heardNow(): number {
  if (!ctx) return 0;
  const lat = ctx.outputLatency || ctx.baseLatency || 0;
  return ctx.currentTime - lat;
}

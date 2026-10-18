let ctx: AudioContext | null = null;
let sampler: ReturnType<typeof setInterval> | null = null;

const WINDOW = 60; // samples kept (~9 s)
const offsets: number[] = []; // performance seconds − "heard" audio-clock seconds
const lags: number[] = []; // render time − heard time = how far the speaker is behind the render head
let offsetMed: number | null = null;
let lagMed = 0;
let offsetJitter = 0;

export const median = (xs: number[]): number => {
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
};
/** Median absolute deviation: a robust spread. */
export const mad = (xs: number[]): number => {
  const m = median(xs);
  return median(xs.map((x) => Math.abs(x - m)));
};
/** performance.now()-style milliseconds → "heard" audio-clock seconds, given the performance↔audio offset. */
export const perfToHeard = (perfMs: number, offsetSec: number): number => perfMs / 1000 - offsetSec;

const push = (arr: number[], v: number): void => {
  arr.push(v);
  if (arr.length > WINDOW) arr.shift();
};

/**
 * One reading of the audio↔performance clock relation. A single reading of getOutputTimestamp()
 * or currentTime is only good to about one audio buffer (5–20 ms) and the browser's latency
 * estimate wobbles, so everything uses the median of many readings instead.
 */
export function sampleClock(): void {
  if (!ctx || ctx.state !== 'running') return;
  const t = ctx.getOutputTimestamp();
  if (!t.contextTime || !t.performanceTime) return;
  push(offsets, t.performanceTime / 1000 - t.contextTime);
  push(lags, Math.max(0, ctx.currentTime - t.contextTime));
  if (offsets.length >= 3) {
    offsetMed = median(offsets);
    lagMed = median(lags);
    offsetJitter = mad(offsets);
  }
}

/** Lazily create the AudioContext. Call from a user gesture (Start / Space) so it can run. */
export function getCtx(): AudioContext {
  if (!ctx) {
    ctx = new AudioContext({ latencyHint: 'interactive' });
    sampler = setInterval(sampleClock, 150);
  }
  return ctx;
}

export async function resumeCtx(): Promise<AudioContext> {
  const c = getCtx();
  if (c.state !== 'running') await c.resume();
  sampleClock();
  return c;
}

/** Fallback before enough readings exist: browser-reported latency. */
const reportedLag = (): number => (ctx ? ctx.outputLatency || ctx.baseLatency || 0 : 0);

/** How far behind the render head the sound is when you hear it, in seconds. */
export function outputLag(): number {
  return offsetMed !== null ? lagMed : reportedLag();
}

/**
 * Convert an event timestamp (performance.now() domain: keyboard and MIDI event.timeStamp) to
 * "heard" audio-clock seconds, the domain the metronome's scheduled click times are in.
 */
export function heardTimeOfPerf(perfMs: number): number {
  if (!ctx) return 0;
  if (offsetMed !== null) return perfToHeard(perfMs, offsetMed);
  return ctx.currentTime - (performance.now() - perfMs) / 1000 - reportedLag();
}

export const heardNow = (): number => heardTimeOfPerf(performance.now());

/** For the diagnostics readout. */
export function clockStats(): { samples: number; lagMs: number; jitterMs: number; calibrated: boolean; underruns: number } {
  return { samples: offsets.length, lagMs: outputLag() * 1000, jitterMs: offsetJitter * 1000, calibrated: offsetMed !== null, underruns };
}

let underruns = 0;
let lastUnderrunAt = 0;
/** The metronome scheduled a click after its time had already passed (the page was busy). */
export function noteUnderrun(): void { underruns++; lastUnderrunAt = performance.now(); }
export const recentUnderrun = (withinMs = 4000): boolean => underruns > 0 && performance.now() - lastUnderrunAt < withinMs;

export function stopClock(): void {
  if (sampler) clearInterval(sampler);
  sampler = null;
}

/**
 * Pure onset detector for a clean DI guitar signal. The AudioWorklet is a thin wrapper around this.
 *
 * One-pole high-pass (~80 Hz) → 256-sample frames every 128 samples → log-energy (dB).
 * An onset is a frame whose energy is above (median of the last 1 s) + k, rose at least `riseDb`
 * over the previous frame, and is outside the refractory period. In dense playing the median sits
 * close to the peaks, so the threshold is also capped at `CAP_DB` below the loudest frame of the
 * last second; that keeps even softer strokes (an upstroke at half the level) detectable. Its time is the first sample in
 * the frame whose amplitude exceeds the threshold; its peak is the largest frame RMS over the
 * next 30 ms.
 */
export const FRAME = 256;
export const HOP = 128;
export const REFRACTORY_MS = 35;
export const PEAK_WINDOW_MS = 30;
export const HIGHPASS_HZ = 80;
export const CAP_DB = 15;

export type OnsetConfig = {
  sampleRate: number;
  k: number; // dB above the 1 s median energy
  riseDb?: number; // minimum frame-to-frame rise
  minDb?: number; // absolute floor (dBFS) so room noise never triggers
};

export type Onset = { sample: number; peak: number }; // absolute sample index; peak frame RMS (0–1)

/** Sensitivity slider (0 = least, 1 = most sensitive) → k in dB. */
export const kFromSensitivity = (s: number): number => 12 - 9 * Math.min(1, Math.max(0, s));

const medianAndMax = (xs: Float32Array, n: number): { median: number; max: number } => {
  const a = Array.from(xs.subarray(0, n)).sort((x, y) => x - y);
  return { median: a[n >> 1], max: a[n - 1] };
};

export class OnsetDetector {
  private readonly hpA: number;
  private px = 0;
  private py = 0;
  private readonly win = new Float32Array(FRAME);
  private pos = 0; // next write index in the ring
  private filled = 0;
  private sinceHop = 0;
  private readonly hist: Float32Array;
  private histN = 0;
  private histPos = 0;
  private prevDb = -120;
  private refractoryUntil = -1;
  private pending: { sample: number; peak: number; end: number } | null = null;
  private frameCount = 0;
  private cfg: Required<OnsetConfig>;

  constructor(cfg: OnsetConfig) {
    this.cfg = { riseDb: 6, minDb: -50, ...cfg };
    this.hpA = 1 / (1 + (2 * Math.PI * HIGHPASS_HZ) / cfg.sampleRate);
    this.hist = new Float32Array(Math.ceil(cfg.sampleRate / HOP)); // 1 s of frames
  }

  setSensitivityDb(k: number): void { this.cfg = { ...this.cfg, k }; }

  /** Feed mono samples; `startSample` is the absolute index of samples[0]. Returns finished onsets. */
  process(samples: ArrayLike<number>, startSample: number): Onset[] {
    const out: Onset[] = [];
    const a = this.hpA;
    for (let i = 0; i < samples.length; i++) {
      const x = samples[i];
      const y = a * (this.py + x - this.px);
      this.px = x;
      this.py = y;
      this.win[this.pos] = y;
      this.pos = (this.pos + 1) % FRAME;
      if (this.filled < FRAME) this.filled++;
      if (++this.sinceHop === HOP) {
        this.sinceHop = 0;
        if (this.filled === FRAME) this.frame(startSample + i, out);
      }
    }
    return out;
  }

  private frame(n: number, out: Onset[]): void {
    let sum = 0;
    for (let j = 0; j < FRAME; j++) sum += this.win[j] * this.win[j];
    const e = sum / FRAME;
    const rms = Math.sqrt(e);
    const db = 10 * Math.log10(e + 1e-12);
    this.frameCount++;

    // keep the 1 s history *before* adding this frame, so a loud frame can't raise its own threshold
    if (this.histN >= 8 && n > this.refractoryUntil) {
      const { median, max } = medianAndMax(this.hist, this.histN);
      const thr = Math.max(Math.min(median + this.cfg.k, max - CAP_DB), this.cfg.minDb);
      if (db > thr && db - this.prevDb >= this.cfg.riseDb) {
        // locate on the attack, not on a previous note's tail: above the threshold and above the frame RMS
        const ampThr = Math.max(Math.sqrt(Math.pow(10, thr / 10)), rms);
        let first = FRAME - 1;
        for (let j = 0; j < FRAME; j++) {
          // oldest sample sits at `pos` (next write index)
          if (Math.abs(this.win[(this.pos + j) % FRAME]) > ampThr) { first = j; break; }
        }
        const sample = n - (FRAME - 1 - first);
        this.refractoryUntil = sample + Math.round((REFRACTORY_MS / 1000) * this.cfg.sampleRate);
        this.pending = { sample, peak: rms, end: sample + Math.round((PEAK_WINDOW_MS / 1000) * this.cfg.sampleRate) };
      }
    }
    const p = this.pending;
    if (p) {
      if (rms > p.peak) p.peak = rms;
      if (n >= p.end) { out.push({ sample: p.sample, peak: p.peak }); this.pending = null; }
    }

    this.hist[this.histPos] = db;
    this.histPos = (this.histPos + 1) % this.hist.length;
    if (this.histN < this.hist.length) this.histN++;
    this.prevDb = db;
  }
}

/** Mono rule: the channel with the highest energy (RMS) over the first second. Ties/silence → channel 0. */
export function pickChannel(energies: number[]): number {
  let best = 0;
  for (let i = 1; i < energies.length; i++) if (energies[i] > energies[best]) best = i;
  return best;
}

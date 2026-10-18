/**
 * Pure onset detector for a clean DI guitar signal. The AudioWorklet is a thin wrapper around this.
 *
 * Signal path: one-pole high-pass (~80 Hz) → loudness frames (peak RMS), and from there a
 * 2nd-order high-pass (~1 kHz) → detection frames. Detection runs on the upper band because on a
 * real string the low fundamentals ring for a long time and their 256-sample energy flutters by
 * several dB as the waveform cycles; a pick's attack is the broadband click, which the upper band
 * keeps and the ringing mostly loses. (A plain full-band detector double-triggered on low notes
 * and missed picks over ringing strings.)
 *
 * Frames are 256 samples every 128. An onset is a frame whose detection energy is above
 * (median of the last 1 s) + k, at least `riseDb` above the recent average (the mean of the
 * frames 3–6 hops earlier), rising, and outside the refractory period. Its time is the first
 * sample in the frame whose amplitude exceeds the threshold; its peak is the largest loudness
 * RMS over the next 30 ms. The threshold is also capped at CAP_DB below the loudest frame of the
 * last second, so softer strokes stay detectable in dense playing.
 */
export const FRAME = 256;
export const HOP = 128;
export const REFRACTORY_MS = 35;
export const PEAK_WINDOW_MS = 30;
export const HIGHPASS_HZ = 80;
export const DETECT_HZ = 1000;
export const CAP_DB = 15;

export type OnsetConfig = {
  sampleRate: number;
  k: number; // dB above the 1 s median energy
  riseDb?: number; // minimum rise above the recent average energy
  minDb?: number; // absolute floor (dBFS of the detection band) so room noise never triggers
  refractoryMs?: number; // dead time after an onset
};

export type Onset = { sample: number; peak: number }; // absolute sample index; peak loudness RMS (0–1)

/** Sensitivity slider (0 = least, 1 = most sensitive) → k in dB. */
export const kFromSensitivity = (s: number): number => 12 - 9 * Math.min(1, Math.max(0, s));

const medianAndMax = (xs: Float32Array, n: number): { median: number; max: number } => {
  const a = Array.from(xs.subarray(0, n)).sort((x, y) => x - y);
  return { median: a[n >> 1], max: a[n - 1] };
};

/** 2nd-order Butterworth high-pass (RBJ cookbook), direct form I. */
class Biquad {
  private x1 = 0; private x2 = 0; private y1 = 0; private y2 = 0;
  private readonly b0: number; private readonly b1: number; private readonly b2: number;
  private readonly a1: number; private readonly a2: number;
  constructor(sampleRate: number, fc: number) {
    const w = (2 * Math.PI * Math.min(fc, sampleRate * 0.45)) / sampleRate;
    const alpha = Math.sin(w) / (2 * Math.SQRT1_2);
    const c = Math.cos(w);
    const a0 = 1 + alpha;
    this.b0 = (1 + c) / 2 / a0;
    this.b1 = -(1 + c) / a0;
    this.b2 = (1 + c) / 2 / a0;
    this.a1 = (-2 * c) / a0;
    this.a2 = (1 - alpha) / a0;
  }
  process(x: number): number {
    const y = this.b0 * x + this.b1 * this.x1 + this.b2 * this.x2 - this.a1 * this.y1 - this.a2 * this.y2;
    this.x2 = this.x1; this.x1 = x; this.y2 = this.y1; this.y1 = y;
    return y;
  }
}

export class OnsetDetector {
  private readonly hpA: number;
  private readonly band: Biquad;
  private px = 0;
  private py = 0;
  private readonly win = new Float32Array(FRAME); // loudness signal (80 Hz high-passed)
  private readonly det = new Float32Array(FRAME); // detection band (1 kHz high-passed)
  private pos = 0; // next write index in the rings
  private filled = 0;
  private sinceHop = 0;
  private readonly hist: Float32Array; // detection dB, last 1 s
  private histN = 0;
  private histPos = 0;
  private prevDb = -120;
  private readonly recent = new Float32Array(8); // linear detection energies of the last 8 frames
  private recentPos = 0;
  private recentN = 0;
  private refractoryUntil = -1;
  private pending: { sample: number; peak: number; end: number } | null = null;
  private cfg: Required<OnsetConfig>;

  constructor(cfg: OnsetConfig) {
    this.cfg = { riseDb: 8, minDb: -65, refractoryMs: REFRACTORY_MS, ...cfg };
    this.hpA = 1 / (1 + (2 * Math.PI * HIGHPASS_HZ) / cfg.sampleRate);
    this.band = new Biquad(cfg.sampleRate, DETECT_HZ);
    this.hist = new Float32Array(Math.ceil(cfg.sampleRate / HOP)); // 1 s of frames
  }

  setSensitivityDb(k: number): void { this.cfg = { ...this.cfg, k }; }
  /** Dead time after an onset; the app sets it to about half a slot so fast playing still gets through. */
  setRefractoryMs(ms: number): void { this.cfg = { ...this.cfg, refractoryMs: Math.max(REFRACTORY_MS, ms) }; }

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
      this.det[this.pos] = this.band.process(y);
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
    let sumLoud = 0;
    let sumDet = 0;
    for (let j = 0; j < FRAME; j++) {
      sumLoud += this.win[j] * this.win[j];
      sumDet += this.det[j] * this.det[j];
    }
    const rms = Math.sqrt(sumLoud / FRAME);
    const e = sumDet / FRAME;
    const detRms = Math.sqrt(e);
    const db = 10 * Math.log10(e + 1e-12);

    // average detection energy of the frames 3–6 hops back (skips the 2 newest, which can hold the attack's first samples)
    let refE = 0;
    if (this.recentN >= 6) {
      for (let j = 3; j <= 6; j++) refE += this.recent[(this.recentPos - j + 16) % 8];
      refE /= 4;
    }
    const refDb = this.recentN >= 6 ? 10 * Math.log10(refE + 1e-12) : db;

    // the 1 s history is read *before* this frame is added, so a loud frame can't raise its own threshold
    if (this.histN >= 8 && this.recentN >= 6 && n > this.refractoryUntil) {
      const { median, max } = medianAndMax(this.hist, this.histN);
      const thr = Math.max(Math.min(median + this.cfg.k, max - CAP_DB), this.cfg.minDb);
      if (db > thr && db - refDb >= this.cfg.riseDb && db > this.prevDb) {
        // locate on the attack, not on a previous note's tail: above the threshold and above the frame RMS
        const ampThr = Math.max(Math.sqrt(Math.pow(10, thr / 10)), detRms);
        let first = FRAME - 1;
        for (let j = 0; j < FRAME; j++) {
          // the oldest sample sits at `pos` (next write index)
          if (Math.abs(this.det[(this.pos + j) % FRAME]) > ampThr) { first = j; break; }
        }
        const sample = n - (FRAME - 1 - first);
        this.refractoryUntil = sample + Math.round((this.cfg.refractoryMs / 1000) * this.cfg.sampleRate);
        this.pending = { sample, peak: rms, end: sample + Math.round((PEAK_WINDOW_MS / 1000) * this.cfg.sampleRate) };
      }
    }
    const p = this.pending;
    if (p) {
      if (rms > p.peak) p.peak = rms;
      if (n >= p.end) { out.push({ sample: p.sample, peak: p.peak }); this.pending = null; }
    }

    this.recent[this.recentPos] = e;
    this.recentPos = (this.recentPos + 1) % 8;
    if (this.recentN < 8) this.recentN++;
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

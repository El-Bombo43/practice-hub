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
export const BAND_HZ = [300, 1200]; // band edges: low < 300 Hz, mid 300–1200 Hz, high > 1200 Hz
export const CAP_DB = 24;

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

/** 2nd-order Butterworth filter (RBJ cookbook), direct form I. */
class Biquad {
  private x1 = 0; private x2 = 0; private y1 = 0; private y2 = 0;
  private readonly b0: number; private readonly b1: number; private readonly b2: number;
  private readonly a1: number; private readonly a2: number;
  constructor(sampleRate: number, fc: number, type: 'hp' | 'lp') {
    const w = (2 * Math.PI * Math.min(fc, sampleRate * 0.45)) / sampleRate;
    const alpha = Math.sin(w) / (2 * Math.SQRT1_2);
    const c = Math.cos(w);
    const a0 = 1 + alpha;
    const k = type === 'hp' ? 1 + c : 1 - c;
    this.b0 = k / 2 / a0;
    this.b1 = (type === 'hp' ? -k : k) / a0;
    this.b2 = k / 2 / a0;
    this.a1 = (-2 * c) / a0;
    this.a2 = (1 - alpha) / a0;
  }
  process(x: number): number {
    const y = this.b0 * x + this.b1 * this.x1 + this.b2 * this.x2 - this.a1 * this.y1 - this.a2 * this.y2;
    this.x2 = this.x1; this.x1 = x; this.y2 = this.y1; this.y1 = y;
    return y;
  }
}

/**
 * One detection band: its own filter chain, its own frame length, and its own 1 s history, so each
 * band judges an onset against *its own* level. A wound low E has a dull attack that is small in
 * the high band but plain in the low one; a loud high E must not hide it, and vice versa.
 */
class Band {
  readonly win: Float32Array;
  private pos = 0;
  private filled = 0;
  private readonly hist: Float32Array;
  private histN = 0;
  private histPos = 0;
  private readonly recent = new Float32Array(16); // linear energies of the last 16 frames
  private recentPos = 0;
  private recentN = 0;
  private prevDb = -120;
  e = 0;
  db = -120;
  rms = 0;
  constructor(readonly frame: number, private readonly chain: Biquad[], sampleRate: number) {
    this.win = new Float32Array(frame);
    this.hist = new Float32Array(Math.ceil(sampleRate / HOP));
  }
  push(x: number): void {
    let y = x;
    for (const f of this.chain) y = f.process(y);
    this.win[this.pos] = y;
    this.pos = (this.pos + 1) % this.frame;
    if (this.filled < this.frame) this.filled++;
  }
  get ready(): boolean { return this.filled === this.frame; }
  /** Index into `win` of the sample `j` places after the oldest. */
  at(j: number): number { return this.win[(this.pos + j) % this.frame]; }

  /** Measure this frame. Must be followed by `candidate` (optionally) and then `commit`. */
  measure(): void {
    let sum = 0;
    for (let j = 0; j < this.frame; j++) sum += this.win[j] * this.win[j];
    this.e = sum / this.frame;
    this.rms = Math.sqrt(this.e);
    this.db = 10 * Math.log10(this.e + 1e-12);
  }

  /** The threshold and the "recent average" this band's frame must beat, or null while warming up. */
  reference(k: number, minDb: number): { thr: number; refDb: number } | null {
    // frames far enough back that they can't contain the attack that this frame is seeing
    const lo = Math.ceil(this.frame / HOP) + 1;
    if (this.histN < 8 || this.recentN < lo + 3) return null;
    let refE = 0;
    for (let j = lo; j < lo + 4; j++) refE += this.recent[(this.recentPos - j + 32) % 16];
    refE /= 4;
    const { median, max } = medianAndMax(this.hist, this.histN);
    return { thr: Math.max(Math.min(median + k, max - CAP_DB), minDb), refDb: 10 * Math.log10(refE + 1e-12) };
  }

  isRising(): boolean { return this.db > this.prevDb; }

  commit(): void {
    this.recent[this.recentPos] = this.e;
    this.recentPos = (this.recentPos + 1) % 16;
    if (this.recentN < 16) this.recentN++;
    this.hist[this.histPos] = this.db;
    this.histPos = (this.histPos + 1) % this.hist.length;
    if (this.histN < this.hist.length) this.histN++;
    this.prevDb = this.db;
  }
}

export class OnsetDetector {
  private readonly hpA: number;
  private px = 0;
  private py = 0;
  private readonly loud = new Float32Array(FRAME); // loudness signal (80 Hz high-passed)
  private loudPos = 0;
  private loudFilled = 0;
  private readonly bands: Band[]; // highest band first: its timing is the sharpest
  private sinceHop = 0;
  private refractoryUntil = -1;
  private pending: { sample: number; peak: number; end: number } | null = null;
  private cfg: Required<OnsetConfig>;

  constructor(cfg: OnsetConfig) {
    this.cfg = { riseDb: 8, minDb: -65, refractoryMs: REFRACTORY_MS, ...cfg };
    const sr = cfg.sampleRate;
    this.hpA = 1 / (1 + (2 * Math.PI * HIGHPASS_HZ) / sr);
    this.bands = [
      new Band(256, [new Biquad(sr, BAND_HZ[1], 'hp')], sr), // high: the pick's click
      new Band(512, [new Biquad(sr, BAND_HZ[0], 'hp'), new Biquad(sr, BAND_HZ[1], 'lp')], sr), // mid: wound-string attack
      new Band(768, [new Biquad(sr, BAND_HZ[0], 'lp')], sr), // low: the fundamental's onset; long frame averages out its cycle
    ];
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
      this.loud[this.loudPos] = y;
      this.loudPos = (this.loudPos + 1) % FRAME;
      if (this.loudFilled < FRAME) this.loudFilled++;
      for (const b of this.bands) b.push(y);
      if (++this.sinceHop === HOP) {
        this.sinceHop = 0;
        if (this.loudFilled === FRAME && this.bands.every((b) => b.ready)) this.frame(startSample + i, out);
      }
    }
    return out;
  }

  private frame(n: number, out: Onset[]): void {
    let sumLoud = 0;
    for (let j = 0; j < FRAME; j++) sumLoud += this.loud[j] * this.loud[j];
    const rms = Math.sqrt(sumLoud / FRAME);

    for (const b of this.bands) b.measure();

    if (n > this.refractoryUntil && !this.pending) {
      for (const b of this.bands) {
        const ref = b.reference(this.cfg.k, this.cfg.minDb);
        if (!ref || !(b.db > ref.thr && b.db - ref.refDb >= this.cfg.riseDb && b.isRising())) continue;
        // locate on the attack, not on a previous note's tail: above the threshold and above the frame RMS
        const ampThr = Math.max(Math.sqrt(Math.pow(10, ref.thr / 10)), b.rms);
        let first = b.frame - 1;
        for (let j = 0; j < b.frame; j++) {
          if (Math.abs(b.at(j)) > ampThr) { first = j; break; }
        }
        const sample = n - (b.frame - 1 - first);
        this.refractoryUntil = sample + Math.round((this.cfg.refractoryMs / 1000) * this.cfg.sampleRate);
        this.pending = { sample, peak: rms, end: sample + Math.round((PEAK_WINDOW_MS / 1000) * this.cfg.sampleRate) };
        break;
      }
    }
    const p = this.pending;
    if (p) {
      if (rms > p.peak) p.peak = rms;
      if (n >= p.end) { out.push({ sample: p.sample, peak: p.peak }); this.pending = null; }
    }
    for (const b of this.bands) b.commit();
  }
}

/** Mono rule: the channel with the highest energy (RMS) over the first second. Ties/silence → channel 0. */
export function pickChannel(energies: number[]): number {
  let best = 0;
  for (let i = 1; i < energies.length; i++) if (energies[i] > energies[best]) best = i;
  return best;
}

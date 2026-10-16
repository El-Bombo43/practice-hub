import { getCtx, heardNow } from './clock';

export type BeatEvent = {
  time: number; // audio-clock seconds the beat is scheduled for
  bpm: number;
  bar: number; // 0 = count-in bar
  beat: number; // 0-based within the bar
};

export type MetronomeConfig = {
  bpm: number;
  beatsPerBar: number;
  subdivision: number;
  volume: number; // 0-1
  gapClick: boolean; // 2 bars on, 1 bar muted (after the count-in)
};

const LOOKAHEAD_S = 0.1;
const INTERVAL_MS = 25;

export class Metronome {
  private timer: ReturnType<typeof setInterval> | null = null;
  private nextTime = 0;
  private sub = 0;
  private beat = 0;
  private bar = 0;
  private bpm: number;
  private pending: { bpm: number; atBar: boolean } | null = null;
  private log: BeatEvent[] = [];

  constructor(private cfg: MetronomeConfig, private onBeat: (e: BeatEvent) => void) {
    this.bpm = cfg.bpm;
  }

  get running(): boolean { return this.timer !== null; }

  configure(patch: Partial<MetronomeConfig>): void {
    this.cfg = { ...this.cfg, ...patch };
  }

  /** Change tempo at the next beat, or at the next bar line when `atBar`. */
  setBpm(bpm: number, atBar = false): void {
    this.cfg.bpm = bpm;
    if (!this.running) { this.bpm = bpm; this.pending = null; return; }
    this.pending = { bpm, atBar };
  }

  start(): void {
    if (this.timer) return;
    const ctx = getCtx();
    this.bpm = this.cfg.bpm;
    this.pending = null;
    this.nextTime = ctx.currentTime + 0.08;
    this.sub = 0; this.beat = 0; this.bar = 0; this.log = [];
    this.schedule();
    this.timer = setInterval(() => this.schedule(), INTERVAL_MS);
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
    this.log = [];
  }

  /** Beat index (0-based) currently audible, or -1 before the first click. */
  currentBeat(): number {
    const now = heardNow();
    let cur = -1;
    for (const e of this.log) if (e.time <= now) cur = e.beat;
    return cur;
  }

  /** Bar number of the audible click (0 = count-in), or -1 before the first click. */
  currentBar(): number {
    const now = heardNow();
    let cur = -1;
    for (const e of this.log) if (e.time <= now) cur = e.bar;
    return cur;
  }

  /** Where the audible click is within the bar, 0–1, or -1 before the first click. */
  position(): number {
    const now = heardNow();
    let cur: BeatEvent | null = null;
    for (const e of this.log) if (e.time <= now) cur = e;
    if (!cur) return -1;
    const frac = Math.min(1, (now - cur.time) / (60 / cur.bpm));
    return (cur.beat + frac) / this.cfg.beatsPerBar;
  }

  private schedule(): void {
    const ctx = getCtx();
    while (this.nextTime < ctx.currentTime + LOOKAHEAD_S) {
      this.scheduleTick(this.nextTime);
      this.nextTime += 60 / this.bpm / this.cfg.subdivision;
      this.advance();
    }
  }

  private advance(): void {
    if (++this.sub < this.cfg.subdivision) return;
    this.sub = 0;
    if (++this.beat < this.cfg.beatsPerBar) return;
    this.beat = 0;
    this.bar++;
  }

  private scheduleTick(t: number): void {
    if (this.sub === 0 && this.pending && (!this.pending.atBar || this.beat === 0)) {
      this.bpm = this.pending.bpm;
      this.pending = null;
    }
    // The tick duration used by schedule() reads this.bpm after this call, which is already updated.
    if (this.sub === 0) {
      const e: BeatEvent = { time: t, bpm: this.bpm, bar: this.bar, beat: this.beat };
      this.log.push(e);
      if (this.log.length > 16) this.log.shift();
      this.onBeat(e);
    }
    const muted = this.cfg.gapClick && this.bar >= 1 && (this.bar - 1) % 3 === 2;
    if (muted) return;
    const accent = this.sub === 0 && this.beat === 0;
    const main = this.sub === 0;
    const freq = accent ? 1500 : main ? 1000 : 800;
    const level = (accent ? 1 : main ? 0.7 : 0.3) * this.cfg.volume;
    this.click(t, freq, level);
  }

  private click(t: number, freq: number, level: number): void {
    if (level <= 0) return;
    const ctx = getCtx();
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.frequency.value = freq;
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(level, t + 0.002);
    g.gain.exponentialRampToValueAtTime(0.0001, t + 0.05);
    osc.connect(g).connect(ctx.destination);
    osc.start(t);
    osc.stop(t + 0.06);
  }
}

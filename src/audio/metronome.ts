import { getCtx, heardNow, noteUnderrun } from './clock';
import { drumHits, isDrums, type Drum, type Sound } from './sounds';

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
  sound?: Sound;
  gapClick: boolean; // 2 bars on, 1 bar muted (after the count-in)
};

const LOOKAHEAD_S = 0.15; // a little over the 100 ms plan: survives longer main-thread stalls
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
    if (t < getCtx().currentTime - 0.003) noteUnderrun(); // this click will sound late
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
    const level = (accent ? 1 : main ? 0.7 : 0.3) * this.cfg.volume;
    const sound = this.cfg.sound ?? 'click';
    if (isDrums(sound)) {
      for (const d of drumHits(sound, this.beat, this.sub, this.cfg.beatsPerBar)) this.drum(t, d, this.cfg.volume, main);
    } else if (sound === 'wood') this.tone(t, accent ? 900 : main ? 650 : 500, level, 0.035, 'triangle');
    else if (sound === 'beep') this.tone(t, accent ? 1000 : main ? 800 : 640, level * 0.8, 0.07, 'sine');
    else this.click(t, accent ? 1500 : main ? 1000 : 800, level);
  }

  private tone(t: number, freq: number, level: number, dur: number, type: OscillatorType): void {
    if (level <= 0) return;
    const ctx = getCtx();
    const osc = ctx.createOscillator();
    const g = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freq, t);
    osc.frequency.exponentialRampToValueAtTime(freq * 0.8, t + dur);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(level, t + 0.002);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    osc.connect(g).connect(ctx.destination);
    osc.start(t);
    osc.stop(t + dur + 0.02);
  }

  private noise: AudioBuffer | null = null;
  private noiseSource(t: number): AudioBufferSourceNode {
    const ctx = getCtx();
    if (!this.noise) {
      this.noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
      const d = this.noise.getChannelData(0);
      for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
    }
    const src = ctx.createBufferSource();
    src.buffer = this.noise;
    src.start(t);
    return src;
  }

  /** Synthesised kick, snare and hat; quiet enough to sit under the playing. */
  private drum(t: number, d: Drum, volume: number, onBeat: boolean): void {
    if (volume <= 0) return;
    const ctx = getCtx();
    const g = ctx.createGain();
    g.connect(ctx.destination);
    if (d === 'kick') {
      const osc = ctx.createOscillator();
      osc.frequency.setValueAtTime(140, t);
      osc.frequency.exponentialRampToValueAtTime(45, t + 0.12);
      g.gain.setValueAtTime(volume * 1.1, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.22);
      osc.connect(g);
      osc.start(t);
      osc.stop(t + 0.25);
    } else if (d === 'snare') {
      const src = this.noiseSource(t);
      const hp = ctx.createBiquadFilter();
      hp.type = 'highpass';
      hp.frequency.value = 1500;
      g.gain.setValueAtTime(volume * 0.7, t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.16);
      src.connect(hp).connect(g);
      src.stop(t + 0.18);
      const body = ctx.createOscillator();
      const bg = ctx.createGain();
      body.type = 'triangle';
      body.frequency.value = 190;
      bg.gain.setValueAtTime(volume * 0.5, t);
      bg.gain.exponentialRampToValueAtTime(0.0001, t + 0.09);
      body.connect(bg).connect(ctx.destination);
      body.start(t);
      body.stop(t + 0.1);
    } else {
      const src = this.noiseSource(t);
      const hp = ctx.createBiquadFilter();
      hp.type = 'highpass';
      hp.frequency.value = 7500;
      g.gain.setValueAtTime(volume * (onBeat ? 0.28 : 0.16), t);
      g.gain.exponentialRampToValueAtTime(0.0001, t + 0.045);
      src.connect(hp).connect(g);
      src.stop(t + 0.06);
    }
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

import { heardNow, resumeCtx } from '../audio/clock';
import { Metronome, type BeatEvent } from '../audio/metronome';
import type { Exercise } from '../data/types';
import type { Settings } from '../data/store';
import type { InputEvent } from '../input/types';
import { scoreRep, slotTimesFromBeats, type RepResult } from '../scoring/score';
import {
  applyRep, clampBpm, dateKey, emptyLive, upsertLog, accuracyPct, type Live, type Profile,
} from './progress';

export type LastRep = { rep: RepResult; bpm: number; windowMs: number };

export type Snapshot = {
  running: boolean;
  exercise: Exercise;
  burst: boolean; // stub toggle only in session 1
  bpm: number;
  settings: Settings;
  live: Live;
  lastRep: LastRep | null;
  profile: Profile;
  sessionXp: number;
  elapsedSec: number; // practice time on the current routine step
  toast: string | null;
};

type Listener = () => void;

/**
 * Owns the metronome, the hit buffer and the per-rep scoring loop. Everything that needs the
 * browser is here; the maths lives in scoring/ and game/progress.ts.
 */
export class Practice {
  private snap: Snapshot;
  private listeners = new Set<Listener>();
  private metro: Metronome;
  private bars = new Map<number, BeatEvent[]>();
  private hits: number[] = [];
  private scorer: ReturnType<typeof setInterval> | null = null;
  private clock: ReturnType<typeof setInterval> | null = null;
  private lastClock = 0;
  private stintId = '';
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(profile: Profile, settings: Settings, exercise: Exercise, burst = false) {
    this.snap = {
      running: false, exercise, burst, bpm: profile.lastBpm[exercise.id] ?? exercise.startBpm, settings,
      live: emptyLive(), lastRep: null, profile, sessionXp: 0, elapsedSec: 0, toast: null,
    };
    this.stintId = String(Date.now());
    this.metro = new Metronome(this.metroConfig(), (e) => this.onBeat(e));
  }

  subscribe = (l: Listener): (() => void) => {
    this.listeners.add(l);
    return () => { this.listeners.delete(l); };
  };
  getSnapshot = (): Snapshot => this.snap;
  currentBeat = (): number => (this.snap.running ? this.metro.currentBeat() : -1);

  private set(patch: Partial<Snapshot>): void {
    this.snap = { ...this.snap, ...patch };
    this.listeners.forEach((l) => l());
  }

  private metroConfig() {
    const { exercise, bpm, settings } = this.snap;
    return {
      bpm, beatsPerBar: exercise.beatsPerBar, subdivision: exercise.subdivision,
      volume: settings.clickVolume, gapClick: settings.gapClick,
    };
  }

  applySettings(settings: Settings): void {
    this.metro.configure({ volume: settings.clickVolume, gapClick: settings.gapClick });
    this.set({ settings });
  }

  /** Switch exercise (a routine step). Keeps playing if it was playing. */
  setExercise(exercise: Exercise, burst = false): void {
    const wasRunning = this.snap.running;
    this.halt();
    const bpm = this.snap.profile.lastBpm[exercise.id] ?? exercise.startBpm;
    this.stintId = String(Date.now());
    this.set({ exercise, burst, bpm, live: emptyLive(), lastRep: null, elapsedSec: 0 });
    this.metro.configure(this.metroConfig());
    this.metro.setBpm(bpm);
    if (wasRunning) void this.start();
  }

  setBurst(burst: boolean): void { this.set({ burst }); }

  async start(): Promise<void> {
    if (this.snap.running) return;
    await resumeCtx();
    this.bars.clear();
    this.hits = [];
    this.metro.configure(this.metroConfig());
    this.metro.start();
    this.lastClock = performance.now();
    this.scorer = setInterval(() => this.finalizeBars(), 30);
    this.clock = setInterval(() => this.tickClock(), 1000);
    this.set({ running: true });
  }

  private halt(): void {
    this.metro.stop();
    if (this.scorer) clearInterval(this.scorer);
    if (this.clock) clearInterval(this.clock);
    this.scorer = this.clock = null;
    this.bars.clear();
    this.hits = [];
    if (this.snap.running) this.set({ running: false });
  }

  pause(): void {
    if (this.snap.running) { this.tickClock(); this.halt(); }
  }

  toggle(): void {
    if (this.snap.running) this.pause(); else void this.start();
  }

  adjustBpm(delta: number): void {
    const bpm = clampBpm(this.snap.bpm + delta);
    if (bpm === this.snap.bpm) return;
    const id = this.snap.exercise.id;
    this.metro.setBpm(bpm);
    this.set({
      bpm,
      live: { ...this.snap.live, cleanStreak: 0 },
      profile: { ...this.snap.profile, lastBpm: { ...this.snap.profile.lastBpm, [id]: bpm } },
    });
  }

  hit(ev: InputEvent): void {
    if (!this.snap.running) return;
    const cal = this.snap.settings.calibrationMs[ev.source] ?? 0;
    this.hits.push(ev.time - cal / 1000);
  }

  private onBeat(e: BeatEvent): void {
    const beats = this.bars.get(e.bar) ?? [];
    beats.push(e);
    this.bars.set(e.bar, beats);
    for (const k of this.bars.keys()) if (k < e.bar - 3) this.bars.delete(k);
  }

  private finalizeBars(): void {
    const { exercise: ex, settings } = this.snap;
    const now = heardNow();
    for (const [bar, beats] of [...this.bars]) {
      if (beats.length < ex.beatsPerBar) continue;
      const last = beats[beats.length - 1];
      const slotDur = 60 / last.bpm / ex.subdivision;
      const margin = slotDur / 2;
      const end = last.time + 60 / last.bpm;
      if (now < end + margin + 0.03) continue;
      this.bars.delete(bar);
      if (bar < 1) { // count-in: not scored
        this.hits = this.hits.filter((h) => h >= end - margin);
        continue;
      }
      const slotTimes = slotTimesFromBeats(beats, ex.subdivision);
      const from = slotTimes[0] - margin;
      const to = end - margin;
      const repHits = this.hits.filter((h) => h >= from && h < to);
      this.hits = this.hits.filter((h) => h >= to);
      this.score(scoreRep(ex.pattern, slotTimes, repHits, settings.windowMs), beats[0].bpm);
    }
  }

  private score(rep: RepResult, repBpm: number): void {
    const { exercise, live, profile, settings } = this.snap;
    const out = applyRep(live, profile, exercise, repBpm, rep);
    let toast: string | null = null;
    if (out.newBest) toast = `New personal best · ${repBpm} BPM`;
    else if (out.stepped) toast = `Tempo up → ${out.bpm} BPM`;
    if (out.bpm !== this.snap.bpm) this.metro.setBpm(out.bpm, true);
    this.set({
      live: out.live,
      profile: this.withLog(out.profile, out.live),
      bpm: out.bpm,
      lastRep: { rep, bpm: repBpm, windowMs: settings.windowMs },
      sessionXp: this.snap.sessionXp + out.xpGain,
      toast,
    });
    if (toast) {
      if (this.toastTimer) clearTimeout(this.toastTimer);
      this.toastTimer = setTimeout(() => this.set({ toast: null }), 2500);
    }
  }

  private tickClock(): void {
    const now = performance.now();
    const dt = (now - this.lastClock) / 1000;
    this.lastClock = now;
    if (dt <= 0) return;
    const today = dateKey();
    const p = this.snap.profile;
    const profile: Profile = {
      ...p, dayMinutes: { ...p.dayMinutes, [today]: (p.dayMinutes[today] ?? 0) + dt / 60 },
    };
    this.set({ elapsedSec: this.snap.elapsedSec + dt, profile: this.withLog(profile, this.snap.live) });
  }

  private withLog(profile: Profile, live: Live): Profile {
    const acc = accuracyPct(live);
    return upsertLog(profile, {
      id: this.stintId,
      date: dateKey(),
      exerciseId: this.snap.exercise.id,
      minutes: Math.round((this.snap.elapsedSec / 60) * 10) / 10,
      bestCleanBpm: live.bestCleanBpm,
      accuracy: acc,
    });
  }

  dispose(): void {
    this.halt();
    this.listeners.clear();
  }
}

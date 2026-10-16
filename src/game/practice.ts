import { heardNow, resumeCtx } from '../audio/clock';
import { Metronome, type BeatEvent } from '../audio/metronome';
import { allowedSubdivisions, firstLane, mirrorExercise, withSubdivision } from '../data/exercises';
import type { Exercise } from '../data/types';
import type { Settings } from '../data/store';
import type { InputEvent } from '../input/types';
import { scoreRep, slotTimesFromBeats, type Hit, type RepResult } from '../scoring/score';
import {
  applyRep, balance, clampBpm, dateKey, emptyLive, upsertLog, accuracyPct, type Hand, type Live, type Profile,
} from './progress';

export type LastRep = { rep: RepResult; bpm: number; windowMs: number; exercise: Exercise };

export type Snapshot = {
  running: boolean;
  exercise: Exercise;
  burst: boolean; // stub toggle only in session 1
  subOptions: Exercise['subdivision'][]; // counts this exercise can be played in
  leadWeaker: boolean; // pads: start the sticking on the weaker hand (mirrors it when needed)
  bpm: number;
  settings: Settings;
  live: Live;
  lastRep: LastRep | null;
  profile: Profile;
  sessionXp: number;
  elapsedSec: number; // practice time on the current routine step
  toast: string | null;
};
// `exercise` in the snapshot is the effective one: the routine's exercise, mirrored when leading with the weaker hand.

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
  private hits: Hit[] = [];
  private base: Exercise;
  private sub: Exercise['subdivision'];
  private mirrored = false;
  private barEx = new Map<number, Exercise>(); // pattern each bar started with (mirroring can change between bars)
  /** Called on the first beat of every bar (MIDI clock re-sync). */
  onBarline: (() => void) | null = null;
  private scorer: ReturnType<typeof setInterval> | null = null;
  private clock: ReturnType<typeof setInterval> | null = null;
  private lastClock = 0;
  private stintId = '';
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(profile: Profile, settings: Settings, exercise: Exercise, burst = false) {
    this.base = exercise;
    this.sub = this.chosenSub(exercise, profile);
    const eff = withSubdivision(exercise, this.sub);
    this.snap = {
      running: false, leadWeaker: false, exercise: eff, subOptions: allowedSubdivisions(exercise), burst,
      bpm: profile.lastBpm[eff.id] ?? eff.startBpm, settings,
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
  /** Position within the bar, 0–1 (−1 when stopped), for the sweeping playhead. */
  position = (): number => (this.snap.running ? this.metro.position() : -1);

  private chosenSub(ex: Exercise, profile: Profile): Exercise['subdivision'] {
    const want = profile.subdivision[ex.id] as Exercise['subdivision'] | undefined;
    return want && allowedSubdivisions(ex).includes(want) ? want : ex.subdivision;
  }

  /** The exercise as played: re-counted, then mirrored when leading with the weaker hand. */
  private build(): Exercise {
    const ex = withSubdivision(this.base, this.sub);
    return this.mirrored ? mirrorExercise(ex) : ex;
  }

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
    this.base = exercise;
    this.sub = this.chosenSub(exercise, this.snap.profile);
    this.mirrored = false;
    this.stintId = String(Date.now());
    this.applyShape({ burst, leadWeaker: false, elapsedSec: 0 });
    if (wasRunning) void this.start();
  }

  /** Re-count the current exercise (8ths, triplets, 16ths, sextuplets). Keeps playing if it was playing. */
  setSubdivision(sub: Exercise['subdivision']): void {
    if (sub === this.sub || !allowedSubdivisions(this.base).includes(sub)) return;
    const wasRunning = this.snap.running;
    this.halt();
    this.sub = sub;
    this.mirrored = false;
    this.stintId = String(Date.now());
    const p = this.snap.profile;
    this.snap = { ...this.snap, profile: { ...p, subdivision: { ...p.subdivision, [this.base.id]: sub } } };
    this.applyShape({});
    if (wasRunning) void this.start();
  }

  /** Rebuild the effective exercise, reset the per-exercise state and re-point the metronome. */
  private applyShape(patch: Partial<Snapshot>): void {
    const exercise = this.build();
    const bpm = this.snap.profile.lastBpm[exercise.id] ?? exercise.startBpm;
    this.set({
      ...patch, exercise, subOptions: allowedSubdivisions(this.base), bpm, live: emptyLive(), lastRep: null,
    });
    this.metro.configure(this.metroConfig());
    this.metro.setBpm(bpm);
  }

  setBurst(burst: boolean): void { this.set({ burst }); }

  setLeadWeaker(leadWeaker: boolean): void {
    this.set({ leadWeaker });
    this.syncMirror(leadWeaker, balance(this.snap.live)?.weaker ?? null);
  }

  /** Mirror the sticking so it starts on the weaker hand; takes effect from the next bar. */
  private syncMirror(lead: boolean, weaker: Hand | null): void {
    const first = firstLane(withSubdivision(this.base, this.sub));
    const mirror = lead && weaker !== null && this.base.instrument === 'pads' && first !== weaker;
    if (mirror !== this.mirrored) {
      this.mirrored = mirror;
      this.set({ exercise: this.build() });
    }
  }

  async start(): Promise<void> {
    if (this.snap.running) return;
    await resumeCtx();
    this.bars.clear();
    this.barEx.clear();
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
    this.barEx.clear();
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
    this.hits.push({ time: ev.time - cal / 1000, lane: ev.lane, velocity: ev.velocity });
  }

  private onBeat(e: BeatEvent): void {
    const beats = this.bars.get(e.bar) ?? [];
    if (beats.length === 0) this.barEx.set(e.bar, this.snap.exercise);
    beats.push(e);
    this.bars.set(e.bar, beats);
    for (const k of this.bars.keys()) if (k < e.bar - 3) { this.bars.delete(k); this.barEx.delete(k); }
    if (e.beat === 0) this.onBarline?.();
  }

  private finalizeBars(): void {
    const { settings } = this.snap;
    const now = heardNow();
    for (const [bar, beats] of [...this.bars]) {
      const ex = this.barEx.get(bar) ?? this.snap.exercise;
      if (beats.length < ex.beatsPerBar) continue;
      const last = beats[beats.length - 1];
      const slotDur = 60 / last.bpm / ex.subdivision;
      const margin = slotDur / 2;
      const end = last.time + 60 / last.bpm;
      if (now < end + margin + 0.03) continue;
      this.bars.delete(bar);
      this.barEx.delete(bar);
      if (bar < 1) { // count-in: not scored
        this.hits = this.hits.filter((h) => h.time >= end - margin);
        continue;
      }
      const slotTimes = slotTimesFromBeats(beats, ex.subdivision);
      const from = slotTimes[0] - margin;
      const to = end - margin;
      const repHits = this.hits.filter((h) => h.time >= from && h.time < to);
      this.hits = this.hits.filter((h) => h.time >= to);
      // Lanes are enforced for pad exercises; for guitar they stay informational.
      const rep = scoreRep(ex.pattern, slotTimes, repHits, settings.windowMs, { enforceLanes: ex.instrument === 'pads' });
      this.score(rep, beats[0].bpm, ex);
    }
  }

  private score(rep: RepResult, repBpm: number, exercise: Exercise): void {
    const { live, profile, settings } = this.snap;
    const out = applyRep(live, profile, exercise, repBpm, rep);
    let toast: string | null = null;
    if (out.newBest) toast = `New personal best · ${repBpm} BPM`;
    else if (out.stepped) toast = `Tempo up → ${out.bpm} BPM`;
    if (out.bpm !== this.snap.bpm) this.metro.setBpm(out.bpm, true);
    this.set({
      live: out.live,
      profile: this.withLog(out.profile, out.live),
      bpm: out.bpm,
      lastRep: { rep, bpm: repBpm, windowMs: settings.windowMs, exercise },
      sessionXp: this.snap.sessionXp + out.xpGain,
      toast,
    });
    this.syncMirror(this.snap.leadWeaker, balance(out.live)?.weaker ?? null);
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

import type { Lane, Slot } from '../data/types';

/** A scored input. Plain numbers (seconds) are accepted too, for lane-less callers. */
export type Hit = { time: number; lane?: Lane; velocity?: number };
export type ScoreOptions = { enforceLanes?: boolean };

export type SlotResult = {
  index: number;
  expected: boolean; // a hit is expected here (play && !ghost)
  lane?: Lane; // expected lane
  hit: boolean;
  hitLane?: Lane; // lane the matched hit came from
  velocity?: number; // 0-1
  wrongLane: boolean; // enforced lanes only: right time, wrong hand → counts as out of window
  offsetMs: number | null; // + = late, - = early
  inWindow: boolean;
};

export type RepResult = {
  slots: SlotResult[];
  extras: number; // hits matching no expected slot (or doubling one)
  expectedCount: number;
  hitCount: number; // matched hits (in or out of window)
  onTimeCount: number;
  clean: boolean;
  meanOffsetMs: number | null;
  /** Outcomes in time order: true = on-time hit, false = miss / out-of-window / extra. */
  sequence: boolean[];
};

export const matchRangeMs = (windowMs: number): number => Math.max(2 * windowMs, 60);
const round2 = (n: number): number => Math.round(n * 100) / 100;

/**
 * Score one rep (one bar). `slotTimes` and `hits` are absolute audio-clock seconds with input
 * calibration already applied. Each hit matches the nearest expected slot within
 * ±max(2×window, 60 ms); a hit whose slot is already taken, or that matches nothing, is an extra.
 */
export function scoreRep(
  pattern: Slot[], slotTimes: number[], hits: (number | Hit)[], windowMs: number, opts: ScoreOptions = {},
): RepResult {
  const range = matchRangeMs(windowMs);
  const expectedIdx: number[] = [];
  pattern.forEach((s, i) => { if (s.play && !s.ghost) expectedIdx.push(i); });

  const slots: SlotResult[] = pattern.map((s, index) => ({
    index, expected: !!s.play && !s.ghost, lane: s.lane, hit: false, offsetMs: null, inWindow: false, wrongLane: false,
  }));
  const timed: { t: number; ok: boolean }[] = [];
  let extras = 0;

  const norm: Hit[] = hits.map((h) => (typeof h === 'number' ? { time: h } : h));
  for (const hit of norm.sort((a, b) => a.time - b.time)) {
    const h = hit.time;
    let best = -1;
    let bestDt = Infinity;
    for (const i of expectedIdx) {
      const dt = Math.abs(h - slotTimes[i]) * 1000;
      if (dt < bestDt) { bestDt = dt; best = i; }
    }
    if (best < 0 || round2(bestDt) > range || slots[best].hit) {
      extras++;
      timed.push({ t: h, ok: false });
      continue;
    }
    const off = round2((h - slotTimes[best]) * 1000);
    const r = slots[best];
    r.hit = true;
    r.offsetMs = off;
    r.hitLane = hit.lane;
    r.velocity = hit.velocity;
    r.wrongLane = !!opts.enforceLanes && !!pattern[best].lane && !!hit.lane && hit.lane !== pattern[best].lane;
    r.inWindow = !r.wrongLane && Math.abs(off) <= windowMs;
  }

  let hitCount = 0;
  let onTimeCount = 0;
  let sum = 0;
  for (const i of expectedIdx) {
    const r = slots[i];
    if (r.hit) { hitCount++; sum += r.offsetMs as number; }
    if (r.inWindow) onTimeCount++;
    timed.push({ t: slotTimes[i], ok: r.inWindow });
  }
  timed.sort((a, b) => a.t - b.t);

  return {
    slots,
    extras,
    expectedCount: expectedIdx.length,
    hitCount,
    onTimeCount,
    clean: expectedIdx.length > 0 && onTimeCount === expectedIdx.length && extras === 0,
    meanOffsetMs: hitCount ? round2(sum / hitCount) : null,
    sequence: timed.map((x) => x.ok),
  };
}

/** Combo: consecutive on-time hits; any miss, out-of-window hit or extra resets it. */
export function updateCombo(combo: number, best: number, sequence: boolean[]): { combo: number; best: number } {
  let c = combo;
  let b = best;
  for (const ok of sequence) {
    if (ok) { c++; if (c > b) b = c; } else c = 0;
  }
  return { combo: c, best: b };
}

/** Ramp: after `cleanRepsToAdvance` clean reps in a row, add `stepBpm`; any unclean rep resets the run. */
export function rampStep(
  cleanStreak: number, clean: boolean, ramp: { stepBpm: number; cleanRepsToAdvance: number },
): { cleanStreak: number; advance: boolean } {
  if (!clean) return { cleanStreak: 0, advance: false };
  const next = cleanStreak + 1;
  if (next >= ramp.cleanRepsToAdvance) return { cleanStreak: 0, advance: true };
  return { cleanStreak: next, advance: false };
}

/** Time of each slot, from the scheduled beat times (each beat carries the tempo it was played at). */
export function slotTimesFromBeats(
  beats: { time: number; bpm: number }[], subdivision: number,
): number[] {
  const out: number[] = [];
  for (const b of beats) {
    const dur = 60 / b.bpm / subdivision;
    for (let s = 0; s < subdivision; s++) out.push(b.time + s * dur);
  }
  return out;
}

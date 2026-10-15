import { describe, expect, it } from 'vitest';
import { applyRep, balance, drumTip, emptyLive, emptyProfile, level, streak, tip, type Live } from './progress';
import { scoreRep } from '../scoring/score';
import { exerciseById } from '../data/exercises';

const ex = exerciseById('tremolo');
const cleanRep = () => {
  const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
  return scoreRep(ex.pattern, times, times, 15);
};

describe('applyRep', () => {
  it('awards 1/hit + 10 per clean rep, and steps after N clean reps (+25)', () => {
    let live = emptyLive();
    let profile = emptyProfile();
    let bpm = 80;
    const gains: number[] = [];
    for (let i = 0; i < 3; i++) {
      const o = applyRep(live, profile, ex, bpm, cleanRep());
      live = o.live; profile = o.profile; bpm = o.bpm; gains.push(o.xpGain);
    }
    expect(gains).toEqual([26, 26, 51]); // no prior PB on the first clean rep → no +50
    expect(bpm).toBe(84);
    expect(profile.personalBests[ex.id]).toBe(80);
    expect(profile.lastBpm[ex.id]).toBe(84);
    expect(live.cleanStreak).toBe(0);
  });
  it('+50 when beating an existing personal best', () => {
    const profile = { ...emptyProfile(), personalBests: { [ex.id]: 80 } };
    const o = applyRep(emptyLive(), profile, ex, 84, cleanRep());
    expect(o.newBest).toBe(true);
    expect(o.xpGain).toBe(16 + 10 + 50);
    expect(o.profile.personalBests[ex.id]).toBe(84);
  });
  it('an unclean rep resets the clean run and the combo', () => {
    const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
    const o1 = applyRep(emptyLive(), emptyProfile(), ex, 80, cleanRep());
    const bad = scoreRep(ex.pattern, times, times.slice(0, 15), 15);
    const o2 = applyRep(o1.live, o1.profile, ex, 80, bad);
    expect(o2.live.cleanStreak).toBe(0);
    expect(o2.live.combo).toBe(0);
    expect(o2.live.bestCombo).toBe(31); // 16 + 15 on-time hits before the missed last slot
  });
});

describe('level / streak / tip', () => {
  it('1000 XP per level', () => {
    expect(level(0)).toBe(1);
    expect(level(999)).toBe(1);
    expect(level(1000)).toBe(2);
  });
  it('streak counts consecutive days with ≥ 10 min; short today does not break it', () => {
    const today = new Date(2026, 9, 14);
    const days = { '2026-10-13': 12, '2026-10-12': 10, '2026-10-11': 9 };
    expect(streak(days, today)).toBe(2);
    expect(streak({ ...days, '2026-10-14': 10 }, today)).toBe(3);
    expect(streak({ '2026-10-12': 30 }, today)).toBe(0);
  });
  it('tip threshold is 8 ms', () => {
    expect(tip(9).tone).toBe('late');
    expect(tip(8).tone).toBe('ok');
    expect(tip(-9).tone).toBe('early');
    expect(tip(null).tone).toBe('idle');
  });
});

const handsLive = (l: [number, number, number], r: [number, number, number]): Live => {
  // [meanVelocity, meanOffsetMs, count]
  const live = emptyLive();
  live.hands.L = { velSum: l[0] * l[2], velN: l[2], offSum: l[1] * l[2], offN: l[2] };
  live.hands.R = { velSum: r[0] * r[2], velN: r[2], offSum: r[1] * r[2], offN: r[2] };
  return live;
};

describe('hand stats from pad reps', () => {
  it('accumulates velocity and offset per hand from hit lanes', () => {
    const pads = exerciseById('pads-singles');
    const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
    const hits = times.map((time, i) => ({ time: time + 0.004, lane: pads.pattern[i].lane, velocity: i % 2 ? 0.5 : 1 }));
    const o = applyRep(emptyLive(), emptyProfile(), pads, 80, scoreRep(pads.pattern, times, hits, 15, { enforceLanes: true }));
    expect(o.live.hands.R.velN).toBe(8);
    expect(o.live.hands.L.velSum / o.live.hands.L.velN).toBe(0.5);
    expect(o.live.hands.R.offSum / o.live.hands.R.offN).toBe(4);
    expect(balance(o.live)).toEqual({ pct: 50, weaker: 'L' });
  });
});

describe('drumTip', () => {
  it('balance under 90% wins, naming the weaker hand', () => {
    const t = drumTip(handsLive([0.6, 0, 10], [0.8, 20, 10]), false);
    expect(t.title).toBe('Left hand is softer');
    expect(t.text).toContain('25% quieter');
    expect(t.offerLead).toBe('L');
    expect(drumTip(handsLive([0.6, 0, 10], [0.8, 0, 10]), true).text).toContain('Next rep starts on the left hand');
  });
  it('then hand offsets more than 8 ms apart', () => {
    const t = drumTip(handsLive([0.8, 2, 10], [0.8, 14, 10]), false);
    expect(t.title).toBe('Right hand drags');
    expect(t.text).toBe('Right hand drags by 12 ms.');
    expect(drumTip(handsLive([0.8, -14, 10], [0.8, 0, 10]), false).title).toBe('Left hand rushes');
  });
  it('otherwise the plain rule; needs enough hits to judge balance', () => {
    const late = { ...handsLive([0.8, 12, 10], [0.8, 10, 10]), offsetSum: 220, offsetN: 20 };
    expect(drumTip(late, false).title).toBe('Mostly late');
    expect(balance(handsLive([0.2, 0, 3], [0.9, 0, 3]))).toBeNull();
  });
});

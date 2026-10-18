import { describe, expect, it } from 'vitest';
import { applyRep, strumTip, balance, burstBpm, burstPhase, drumTip, evenness, pickTip, emptyLive, emptyProfile, level, streak, tip, type Live } from './progress';
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

const strokesLive = (d: [number, number], u: [number, number], n = 10): Live => {
  // [meanPeak, meanOffsetMs]
  const live = emptyLive();
  live.strokes.down = { velSum: d[0] * n, velN: n, offSum: d[1] * n, offN: n };
  live.strokes.up = { velSum: u[0] * n, velN: n, offSum: u[1] * n, offN: n };
  return live;
};

describe('stroke evenness and picking tips', () => {
  it('assigns strokes by the slot lane (down, up, down, up…) with peak and offset', () => {
    const t = exerciseById('tremolo');
    const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
    const hits = times.map((time, i) => ({ time: time + (i % 2 ? 0.012 : 0.002), velocity: i % 2 ? 0.4 : 0.8 }));
    const o = applyRep(emptyLive(), emptyProfile(), t, 80, scoreRep(t.pattern, times, hits, 15));
    expect(o.live.strokes.down.velN).toBe(8);
    expect(o.live.strokes.up.offSum / o.live.strokes.up.offN).toBe(12);
    expect(o.live.strokes.down.velSum / o.live.strokes.down.velN).toBeCloseTo(0.8, 9);
    expect(evenness(o.live)?.loudU).toBeCloseTo(0.4, 9);
  });
  it('offset gap over 8 ms wins: upstrokes dragging', () => {
    const t = pickTip(strokesLive([0.5, 2], [0.5, 14]));
    expect(t.title).toBe('Upstrokes are dragging');
    expect(t.text).toBe('Upstrokes are dragging — +14 ms late on average.');
  });
  it('then loudness more than 10% apart', () => {
    const t = pickTip(strokesLive([0.5, 3], [0.4, 5]));
    expect(t.title).toBe('Upstrokes are softer');
    expect(t.text).toContain('20% quieter');
    expect(pickTip(strokesLive([0.5, 3], [0.47, 5])).tone).toBe('idle'); // 6% apart, no offsets overall
  });
  it('needs enough strokes of each direction', () => {
    expect(evenness(strokesLive([0.5, 0], [0.5, 30], 3))).toBeNull();
  });
});

describe('burst and frozen tempo', () => {
  it('3 bars play, 1 bar rest, repeating', () => {
    expect([1, 2, 3, 4, 5, 6, 7, 8, 9].map(burstPhase)).toEqual(['play', 'play', 'play', 'rest', 'play', 'play', 'play', 'rest', 'play']);
  });
  it('plays at clean max + 8', () => {
    expect(burstBpm(120, 100)).toBe(128);
    expect(burstBpm(undefined, 100)).toBe(108);
    expect(burstBpm(295, 100)).toBe(300);
  });
  it('a frozen rep gives XP for hits and clean but never changes ramp, PB or last BPM', () => {
    const ex = exerciseById('tremolo');
    const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
    const rep = scoreRep(ex.pattern, times, times, 15);
    let live = emptyLive();
    const profile = { ...emptyProfile(), personalBests: { [ex.id]: 80 }, lastBpm: { [ex.id]: 80 } };
    let p = profile;
    for (let i = 0; i < 4; i++) { const o = applyRep(live, p, ex, 88, rep, true); live = o.live; p = o.profile; expect(o.stepped).toBe(false); expect(o.bpm).toBe(88); expect(o.xpGain).toBe(26); }
    expect(p.personalBests[ex.id]).toBe(80);
    expect(p.lastBpm[ex.id]).toBe(80);
    expect(live.cleanStreak).toBe(0);
  });
});

describe('strum accents', () => {
  const ex = exerciseById('strum-16th-accents');
  const times = Array.from({ length: 16 }, (_, i) => i * 0.125);
  const bar = (accentVel: number) => {
    const hits = ex.pattern.flatMap((s, i) => (s.play && !s.ghost ? [{ time: times[i], velocity: s.accent ? accentVel : 0.5 }] : []));
    return scoreRep(ex.pattern, times, hits, 15);
  };
  it('counts accents landed across bars', () => {
    let live = emptyLive();
    let p = emptyProfile();
    for (const v of [0.7, 0.52]) { const o = applyRep(live, p, ex, 80, bar(v)); live = o.live; p = o.profile; }
    expect(live.accents).toMatchObject({ landed: 2, total: 4 }); // bar 1: both land (+40%); bar 2: +4%, neither
  });
  it('tip names the beat of the weakest accent once it is below +15% for 2 bars', () => {
    let live = emptyLive();
    let p = emptyProfile();
    for (let i = 0; i < 2; i++) { const o = applyRep(live, p, ex, 80, bar(0.52)); live = o.live; p = o.profile; }
    const t = strumTip(live, ex);
    expect(t.title).toBe('Beat-2 accent is getting lost'); // slot 5 (index 4) is beat 2; ties go to the first
    expect(t.text).toContain('Only 4% louder');
    expect(strumTip(emptyLive(), ex).tone).toBe('idle');
  });
});

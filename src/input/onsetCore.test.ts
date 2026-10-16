import { describe, expect, it } from 'vitest';
import { kFromSensitivity, OnsetDetector, pickChannel } from './onsetCore';

/** Deterministic noise in [-1, 1). */
function rng(seed: number) {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 2147483648 - 1;
  };
}

/** Decaying noise bursts at the given times (s), plus optional constant background noise. */
function render(sr: number, seconds: number, times: number[], opts: { tau?: number; amp?: number; bg?: number; seed?: number } = {}) {
  const { tau = 0.03, amp = 0.4, bg = 0, seed = 1 } = opts;
  const r = rng(seed);
  const x = new Float32Array(Math.round(sr * seconds));
  for (let i = 0; i < x.length; i++) x[i] = bg * r();
  for (const t of times) {
    const s0 = Math.round(t * sr);
    for (let i = s0; i < x.length && i < s0 + sr * 0.4; i++) x[i] += amp * r() * Math.exp(-(i - s0) / sr / tau);
  }
  return x;
}

function detect(x: Float32Array, sr: number, k = kFromSensitivity(0.5)) {
  const det = new OnsetDetector({ sampleRate: sr, k });
  const out: { sample: number; peak: number }[] = [];
  for (let i = 0; i < x.length; i += 128) out.push(...det.process(x.subarray(i, i + 128), i));
  return out;
}

const sixteenths = (bpm: number, n: number, start = 1) => Array.from({ length: n }, (_, i) => start + (i * 60) / bpm / 4);

describe('onset detector', () => {
  for (const sr of [44100, 48000]) {
    for (const bpm of [90, 140, 200]) {
      for (const bg of [0, 0.004]) {
        it(`16ths at ${bpm} BPM, ${sr} Hz, background ${bg ? 'noise' : 'silence'}: within ±5 ms, no doubles`, () => {
          const times = sixteenths(bpm, 24);
          const x = render(sr, times[times.length - 1] + 1, times, { bg });
          const found = detect(x, sr);
          expect(found.length).toBe(times.length); // no misses, no doubles
          found.forEach((o, i) => {
            expect(Math.abs((o.sample / sr - times[i]) * 1000)).toBeLessThanOrEqual(5);
          });
        });
      }
    }
  }

  it('alternating loud/soft strokes (down/up unevenness) are all found', () => {
    const sr = 48000;
    const times = sixteenths(160, 16);
    const x = new Float32Array(Math.round(sr * (times[15] + 1)));
    times.forEach((t, i) => x.set(render(sr, 0.4, [0], { amp: i % 2 ? 0.2 : 0.4, seed: i + 3 }), Math.round(t * sr)));
    const found = detect(x, sr);
    expect(found.length).toBe(16);
    const mean = (xs: number[]) => xs.reduce((a, b) => a + b, 0) / xs.length;
    const down = mean(found.filter((_, i) => i % 2 === 0).map((o) => o.peak));
    const up = mean(found.filter((_, i) => i % 2 === 1).map((o) => o.peak));
    expect(up / down).toBeGreaterThan(0.35);
    expect(up / down).toBeLessThan(0.65);
  });

  it('room noise alone triggers nothing', () => {
    const sr = 48000;
    expect(detect(render(sr, 3, [], { bg: 0.004 }), sr).length).toBe(0);
  });

  it('a ringing note does not double-trigger', () => {
    const sr = 48000;
    const x = render(sr, 3, [1], { tau: 0.5 });
    expect(detect(x, sr).length).toBe(1);
  });

  it('higher sensitivity means a lower k', () => {
    expect(kFromSensitivity(1)).toBeLessThan(kFromSensitivity(0));
  });
});

describe('pickChannel', () => {
  it('chooses the loudest channel; silence falls back to 0', () => {
    expect(pickChannel([0.001, 0.2])).toBe(1);
    expect(pickChannel([0.3, 0.1])).toBe(0);
    expect(pickChannel([0, 0])).toBe(0);
  });
});

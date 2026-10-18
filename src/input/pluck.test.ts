import { describe, expect, it } from 'vitest';
import { kFromSensitivity, OnsetDetector } from './onsetCore';

function rng(seed: number) {
  let s = seed >>> 0;
  return () => { s = (Math.imul(s, 1664525) + 1013904223) >>> 0; return s / 2147483648 - 1; };
}

/** A plucked-string-like note: a 3 ms noise transient, then decaying harmonics of f0 with a slow envelope. */
function addPluck(x: Float32Array, sr: number, t0: number, f0: number, amp: number, seed: number, tau = 0.35) {
  const r = rng(seed);
  const s0 = Math.round(t0 * sr);
  const n = Math.min(x.length - s0, Math.round(sr * 1.5));
  for (let i = 0; i < n; i++) {
    const t = i / sr;
    let v = 0;
    for (let h = 1; h <= 6; h++) v += Math.sin(2 * Math.PI * f0 * h * t + h) * Math.exp(-t / (tau / (1 + 0.6 * h))) / h;
    const atk = t < 0.003 ? r() * 1.2 : 0;
    x[s0 + i] += amp * (0.55 * v + atk);
  }
}

function detect(x: Float32Array, sr: number, k = kFromSensitivity(0.6)) {
  const det = new OnsetDetector({ sampleRate: sr, k });
  const out: { sample: number; peak: number }[] = [];
  for (let i = 0; i < x.length; i += 128) out.push(...det.process(x.subarray(i, i + 128), i));
  return out;
}

describe('plucked strings with hum, noise, long sustain and damped notes', () => {
  const cases: { name: string; tau: number; bpm: number; hum: number; bg: number }[] = [
    { name: 'long sustain', tau: 0.9, bpm: 140, hum: 0, bg: 0.001 },
    { name: '60 Hz hum + noise', tau: 0.35, bpm: 120, hum: 0.02, bg: 0.003 },
    { name: 'palm-muted', tau: 0.05, bpm: 180, hum: 0.01, bg: 0.002 },
    { name: 'fast 16ths', tau: 0.3, bpm: 200, hum: 0, bg: 0.001 },
  ];
  for (const c of cases) {
    it(`${c.name}: one onset per pick`, () => {
      const sr = 44100;
      const times = Array.from({ length: 24 }, (_, i) => 1 + (i * 60) / c.bpm / 4);
      const x = new Float32Array(Math.round(sr * (times[23] + 1.5)));
      const r = rng(99);
      for (let i = 0; i < x.length; i++) x[i] = c.bg * r() + c.hum * Math.sin((2 * Math.PI * 60 * i) / sr);
      times.forEach((t, i) => addPluck(x, sr, t, i % 3 === 0 ? 82.4 : 146.8, i % 2 ? 0.22 : 0.32, i + 5, c.tau));
      const found = detect(x, sr);
      expect(found.length).toBe(times.length);
      found.forEach((o, i) => expect(Math.abs((o.sample / sr - times[i]) * 1000)).toBeLessThanOrEqual(5));
    });
  }
  it('hum and noise alone trigger nothing', () => {
    const sr = 48000;
    const x = new Float32Array(sr * 3);
    const r = rng(7);
    for (let i = 0; i < x.length; i++) x[i] = 0.003 * r() + 0.02 * Math.sin((2 * Math.PI * 60 * i) / sr);
    expect(detect(x, sr).length).toBe(0);
  });
});

describe('plucked-string signal (low fundamentals ring and flutter)', () => {
  for (const f0 of [82.4, 110, 196]) {
    for (const bpm of [80, 120, 160]) {
      it(`f0 ${f0} Hz, 16ths at ${bpm} BPM: one onset per pick, within ±5 ms`, () => {
        const sr = 48000;
        const times = Array.from({ length: 20 }, (_, i) => 1 + (i * 60) / bpm / 4);
        const x = new Float32Array(Math.round(sr * (times[19] + 1.5)));
        times.forEach((t, i) => addPluck(x, sr, t, f0, i % 2 ? 0.25 : 0.35, i + 1));
        const found = detect(x, sr);
        expect(found.length).toBe(times.length);
        found.forEach((o, i) => expect(Math.abs((o.sample / sr - times[i]) * 1000)).toBeLessThanOrEqual(5));
      });
    }
  }
});

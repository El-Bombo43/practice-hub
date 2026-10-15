import { describe, expect, it } from 'vitest';
import { laneForNote, parseMidi, perfToAudioTime } from './midi';

describe('perfToAudioTime', () => {
  const m = { contextTime: 10, performanceTime: 5000 };
  it('maps performance.now ms onto the audio clock', () => {
    expect(perfToAudioTime(5000, m)).toBe(10);
    expect(perfToAudioTime(5250, m)).toBeCloseTo(10.25, 9);
    expect(perfToAudioTime(4900, m)).toBeCloseTo(9.9, 9); // events earlier than the sample point
  });
  it('a re-synced mapping follows clock drift', () => {
    const drifted = { contextTime: 20.002, performanceTime: 15000 }; // audio clock gained 2 ms
    expect(perfToAudioTime(15100, drifted)).toBeCloseTo(20.102, 9);
  });
});

describe('parseMidi', () => {
  it('note-on with velocity, any channel', () => {
    expect(parseMidi([0x99, 36, 100])).toEqual({ type: 'noteon', note: 36, velocity: 100 });
  });
  it('note-on velocity 0 and note-off are ignored', () => {
    expect(parseMidi([0x90, 36, 0])).toBeNull();
    expect(parseMidi([0x80, 36, 64])).toBeNull();
  });
  it('control change', () => {
    expect(parseMidi([0xb0, 64, 127])).toEqual({ type: 'cc', cc: 64, value: 127 });
  });
  it('short or unrelated messages', () => {
    expect(parseMidi([0xf8])).toBeNull();
    expect(parseMidi([0xe0, 0, 64])).toBeNull();
  });
});

describe('laneForNote', () => {
  it('maps learned notes to hands', () => {
    expect(laneForNote(36, { left: 36, right: 37 })).toBe('L');
    expect(laneForNote(37, { left: 36, right: 37 })).toBe('R');
    expect(laneForNote(40, { left: 36, right: 37 })).toBeNull();
  });
});

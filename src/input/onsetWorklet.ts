/// AudioWorklet wrapper around the pure detector in onsetCore.ts. Loaded with `?worker&url`.
import { OnsetDetector, pickChannel } from './onsetCore';

declare const sampleRate: number;
declare const currentFrame: number;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
}
declare function registerProcessor(name: string, ctor: new () => AudioWorkletProcessor): void;

type ConfigMsg = { type: 'config'; k?: number; channel?: number | 'auto' } | { type: 'relock' };

const LEVEL_EVERY = 2048; // samples between level reports (~43 ms)

class OnsetProcessor extends AudioWorkletProcessor {
  private det = new OnsetDetector({ sampleRate, k: 7.5 });
  private mono = new Float32Array(128);
  private channel: number | 'auto' = 'auto';
  private locked = -1; // chosen channel once the first second has been measured
  private energy: number[] = [];
  private lockSamples = 0;
  private levelSum = 0;
  private levelPeak = 0;
  private levelN = 0;

  constructor() {
    super();
    this.port.onmessage = (e: MessageEvent<ConfigMsg>) => {
      const m = e.data;
      if (m.type === 'relock') { this.locked = -1; this.energy = []; this.lockSamples = 0; }
      else {
        if (m.k !== undefined) this.det.setSensitivityDb(m.k);
        if (m.channel !== undefined) { this.channel = m.channel; this.locked = -1; this.energy = []; this.lockSamples = 0; }
      }
    };
  }

  process(inputs: Float32Array[][]): boolean {
    const input = inputs[0];
    if (!input || !input.length) return true;
    const n = input[0].length;
    if (this.mono.length !== n) this.mono = new Float32Array(n);

    let ch = typeof this.channel === 'number' ? Math.min(this.channel, input.length - 1) : this.locked;
    if (ch < 0) {
      // first second: measure every channel, meanwhile follow the loudest so far
      if (this.energy.length !== input.length) this.energy = new Array(input.length).fill(0);
      for (let c = 0; c < input.length; c++) {
        let s = 0;
        for (let i = 0; i < n; i++) s += input[c][i] * input[c][i];
        this.energy[c] += s;
      }
      this.lockSamples += n;
      ch = pickChannel(this.energy);
      if (this.lockSamples >= sampleRate) {
        this.locked = ch;
        this.port.postMessage({ type: 'channel', channel: ch });
      }
    }
    this.mono.set(input[ch]);

    for (let i = 0; i < n; i++) {
      const a = Math.abs(this.mono[i]);
      this.levelSum += a * a;
      if (a > this.levelPeak) this.levelPeak = a;
    }
    this.levelN += n;
    if (this.levelN >= LEVEL_EVERY) {
      this.port.postMessage({ type: 'level', rms: Math.sqrt(this.levelSum / this.levelN), peak: this.levelPeak });
      this.levelSum = 0; this.levelPeak = 0; this.levelN = 0;
    }

    // currentFrame is the absolute index of this block's first sample (context time = frame / sampleRate)
    for (const o of this.det.process(this.mono, currentFrame)) {
      this.port.postMessage({ type: 'onset', time: o.sample / sampleRate, peak: o.peak });
    }
    return true;
  }
}

registerProcessor('onset-processor', OnsetProcessor);

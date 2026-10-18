import { getCtx, outputLag } from '../audio/clock';
import { inputBus } from './bus';
import { kFromSensitivity } from './onsetCore';
import workletUrl from './onsetWorklet.ts?worker&url';

export type AudioDevice = { id: string; name: string };

/** Peak frame RMS (0–1) → InputEvent velocity (0–1). Linear in amplitude, so ratios between strokes stay meaningful. */
export const peakToVelocity = (peak: number): number => Math.min(1, peak * 2);

/** Guitar DI input: getUserMedia → AudioWorklet onset detector → InputEvents on the bus. */
export class AudioInput {
  private stream: MediaStream | null = null;
  private source: MediaStreamAudioSourceNode | null = null;
  private node: AudioWorkletNode | null = null;
  private sink: GainNode | null = null;
  private loaded = false;
  private listeners = new Set<() => void>();
  error: string | null = null;
  channel: number | null = null; // locked channel after the first second
  rms = 0;
  peak = 0;
  lastOnsetAt = 0; // performance.now of the last detection, for the meter flash
  picks = 0; // detections since the input started
  lastPeak = 0; // loudness of the last pick (0–1 RMS)
  active = false;

  subscribe = (l: () => void): (() => void) => {
    this.listeners.add(l);
    return () => { this.listeners.delete(l); };
  };
  private emit(): void { this.listeners.forEach((l) => l()); }

  /** Device list; labels only appear after microphone permission has been granted once. */
  async devices(): Promise<AudioDevice[]> {
    if (!navigator.mediaDevices?.enumerateDevices) return [];
    const all = await navigator.mediaDevices.enumerateDevices();
    return all.filter((d) => d.kind === 'audioinput').map((d, i) => ({ id: d.deviceId, name: d.label || `Input ${i + 1}` }));
  }

  async start(deviceId: string | null, sensitivity: number): Promise<boolean> {
    this.stop();
    if (!navigator.mediaDevices?.getUserMedia) {
      this.error = 'Audio input is not available (needs HTTPS and a supported browser).';
      this.emit();
      return false;
    }
    try {
      this.stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          deviceId: deviceId ? { exact: deviceId } : undefined,
          echoCancellation: false, noiseSuppression: false, autoGainControl: false,
          channelCount: { ideal: 2 },
        },
      });
    } catch (e) {
      const name = e instanceof DOMException ? e.name : '';
      this.error = name === 'NotAllowedError'
        ? 'Microphone access was blocked. Allow it in the site permissions and retry.'
        : 'Could not open that input. Close Ableton (or any app holding the interface), then retry.';
      this.emit();
      return false;
    }
    try {
      const ctx = getCtx();
      void ctx.resume(); // may wait for a user gesture; the worklet loads either way
      if (!this.loaded) { await ctx.audioWorklet.addModule(workletUrl); this.loaded = true; }
      this.source = ctx.createMediaStreamSource(this.stream);
      this.node = new AudioWorkletNode(ctx, 'onset-processor', {
        numberOfInputs: 1, numberOfOutputs: 1, outputChannelCount: [1],
        channelCount: 2, channelCountMode: 'explicit', channelInterpretation: 'discrete',
      });
      this.node.port.onmessage = (e: MessageEvent) => this.onMessage(e.data);
      this.node.port.postMessage({ type: 'config', k: kFromSensitivity(sensitivity) });
      // a silent sink keeps the node pulled by the audio graph
      this.sink = ctx.createGain();
      this.sink.gain.value = 0;
      this.source.connect(this.node).connect(this.sink).connect(ctx.destination);
    } catch {
      this.error = 'Audio input could not start (AudioWorklet failed to load).';
      this.stop();
      this.emit();
      return false;
    }
    this.error = null;
    this.active = true;
    this.channel = null;
    this.emit();
    return true;
  }

  stop(): void {
    this.node?.disconnect();
    this.source?.disconnect();
    this.sink?.disconnect();
    this.stream?.getTracks().forEach((t) => t.stop());
    this.node = this.source = this.sink = null;
    this.stream = null;
    this.active = false;
    this.rms = this.peak = 0;
  }

  /** Dead time after a pick: about half a slot of the current exercise, so fast playing still gets through. */
  setRefractoryMs(ms: number): void { this.node?.port.postMessage({ type: 'config', refractoryMs: ms }); }
  setSensitivity(s: number): void { this.node?.port.postMessage({ type: 'config', k: kFromSensitivity(s) }); }
  /** Re-measure which channel carries the guitar: play for one second afterwards. */
  relock(): void { this.channel = null; this.node?.port.postMessage({ type: 'relock' }); this.emit(); }

  private onMessage(m: { type: string; time?: number; peak?: number; rms?: number; channel?: number }): void {
    if (m.type === 'level') { this.rms = m.rms ?? 0; this.peak = m.peak ?? 0; return; }
    if (m.type === 'channel') { this.channel = m.channel ?? 0; this.emit(); return; }
    if (m.type === 'onset' && m.time !== undefined) {
      this.lastOnsetAt = performance.now();
      this.picks++;
      this.lastPeak = m.peak ?? 0;
      // the onset sample's render time → "heard" domain (the same one keyboard and MIDI hits are in)
      inputBus.emit({ time: m.time - outputLag(), source: 'audio', velocity: peakToVelocity(m.peak ?? 0) });
    }
  }
}

import { resumeCtx, heardNow } from './clock';
import { Metronome } from './metronome';
import { inputBus } from '../input/bus';
import type { InputEvent } from '../input/types';
import {
  calibrationFromHits, CALIBRATION_BPM, CALIBRATION_CLICKS, type CalibrationResult,
} from '../scoring/calibration';

/** Plays 16 clicks at 90 BPM and collects the raw (uncalibrated) hits from one input source. */
export class Calibrator {
  private metro: Metronome | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private unsub: (() => void) | null = null;

  async run(
    source: InputEvent['source'], volume: number, onClick: (n: number) => void,
  ): Promise<CalibrationResult> {
    this.cancel();
    await resumeCtx();
    const clicks: number[] = [];
    const hits: number[] = [];
    this.unsub = inputBus.subscribe((e) => { if (e.source === source) hits.push(e.time); });
    return new Promise((resolve) => {
      const metro = new Metronome(
        { bpm: CALIBRATION_BPM, beatsPerBar: 4, subdivision: 1, volume, gapClick: false },
        (e) => {
          if (clicks.length >= CALIBRATION_CLICKS) return;
          clicks.push(e.time);
          onClick(clicks.length);
          if (clicks.length === CALIBRATION_CLICKS) {
            metro.stop();
            const wait = Math.max(0, e.time - heardNow()) * 1000 + 450;
            this.timer = setTimeout(() => { this.cancel(); resolve(calibrationFromHits(clicks, hits)); }, wait);
          }
        },
      );
      this.metro = metro;
      metro.start();
    });
  }

  cancel(): void {
    this.metro?.stop();
    this.metro = null;
    if (this.timer) clearTimeout(this.timer);
    this.timer = null;
    this.unsub?.();
    this.unsub = null;
  }
}

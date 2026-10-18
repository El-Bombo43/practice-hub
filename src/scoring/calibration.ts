export const CALIBRATION_CLICKS = 16;
export const CALIBRATION_BPM = 90;
export const CALIBRATION_MAX_OFFSET_MS = 150;

export type CalibrationResult = {
  medianMs: number | null; used: number; ignored: number;
  spreadMs: number | null; // median absolute deviation of the usable hits: how steady the timing was
};

export function median(xs: number[]): number | null {
  if (!xs.length) return null;
  const s = [...xs].sort((a, b) => a - b);
  const m = s.length >> 1;
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}

/**
 * Median offset (ms, + = late) of hits against their nearest click. Hits more than 150 ms from
 * every click are ignored. The median becomes the stored calibration, which scoring subtracts.
 */
export function calibrationFromHits(clicks: number[], hits: number[]): CalibrationResult {
  const offsets: number[] = [];
  let ignored = 0;
  for (const h of hits) {
    let best = Infinity;
    for (const c of clicks) {
      const d = (h - c) * 1000;
      if (Math.abs(d) < Math.abs(best)) best = d;
    }
    if (Math.abs(best) > CALIBRATION_MAX_OFFSET_MS) ignored++;
    else offsets.push(best);
  }
  const m = median(offsets);
  const spread = m === null ? null : median(offsets.map((o) => Math.abs(o - m)));
  return {
    medianMs: m === null ? null : Math.round(m * 10) / 10, used: offsets.length, ignored,
    spreadMs: spread === null ? null : Math.round((spread as number) * 10) / 10,
  };
}

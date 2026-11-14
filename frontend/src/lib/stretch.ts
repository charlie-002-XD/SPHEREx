// Display stretch functions (pure, NaN/Inf-tolerant) plus display intervals.
// Linear, Log and Asinh share a 1-99 percentile interval; ZScale computes its
// own IRAF-style interval (linear fit on sorted values with sigma rejection).

export const STRETCH_KINDS = ["linear", "log", "asinh", "zscale"] as const;
export type StretchKind = (typeof STRETCH_KINDS)[number];

export interface Interval {
  lo: number;
  hi: number;
}

export function isStretchKind(v: unknown): v is StretchKind {
  return typeof v === "string" && (STRETCH_KINDS as readonly string[]).includes(v);
}

function finiteSorted(data: Float32Array | readonly number[]): number[] {
  const out: number[] = [];
  for (const v of data) {
    if (Number.isFinite(v)) out.push(v);
  }
  out.sort((a, b) => a - b);
  return out;
}

export function percentile(sorted: readonly number[], p: number): number {
  if (sorted.length === 0) return NaN;
  const idx = (p / 100) * (sorted.length - 1);
  const lo = Math.floor(idx);
  const hi = Math.ceil(idx);
  const a = sorted[lo] ?? NaN;
  const b = sorted[hi] ?? NaN;
  return a + (b - a) * (idx - lo);
}

export function minMaxInterval(data: Float32Array | readonly number[]): Interval {
  const s = finiteSorted(data);
  if (s.length === 0) return { lo: 0, hi: 1 };
  const lo = s[0] ?? 0;
  const hi = s[s.length - 1] ?? 1;
  return lo === hi ? { lo: lo - 0.5, hi: hi + 0.5 } : { lo, hi };
}

export function percentileInterval(
  data: Float32Array | readonly number[],
  loPct = 1,
  hiPct = 99,
): Interval {
  const s = finiteSorted(data);
  if (s.length === 0) return { lo: 0, hi: 1 };
  let lo = percentile(s, loPct);
  let hi = percentile(s, hiPct);
  if (!(hi > lo)) {
    const mm = minMaxInterval(s);
    lo = mm.lo;
    hi = mm.hi;
  }
  return { lo, hi };
}

// IRAF/STScI numdisplay-style zscale: least-squares line through the sorted
// sample with iterative 2.5-sigma rejection; display range is the fitted
// slope rescaled by `contrast` around the median.
export function zscaleInterval(
  data: Float32Array | readonly number[],
  contrast = 0.25,
  maxSamples = 600,
): Interval {
  const all = finiteSorted(data);
  const n = all.length;
  if (n < 10) return minMaxInterval(all);
  const step = Math.max(1, Math.floor(n / maxSamples));
  const z: number[] = [];
  for (let i = 0; i < n; i += step) {
    const v = all[i];
    if (v !== undefined) z.push(v);
  }
  const krej = 2.5;
  const maxIter = 5;
  const mask = z.map(() => true);
  let slope = 0;
  for (let iter = 0; iter < maxIter; iter++) {
    let sx = 0;
    let sy = 0;
    let sxx = 0;
    let sxy = 0;
    let m = 0;
    for (let i = 0; i < z.length; i++) {
      if (mask[i] !== true) continue;
      const y = z[i];
      if (y === undefined) continue;
      sx += i;
      sy += y;
      sxx += i * i;
      sxy += i * y;
      m++;
    }
    if (m < 2) break;
    const denom = m * sxx - sx * sx;
    if (denom === 0) break;
    slope = (m * sxy - sx * sy) / denom;
    const intercept = (sy - slope * sx) / m;
    let sumRes2 = 0;
    for (let i = 0; i < z.length; i++) {
      if (mask[i] !== true) continue;
      const y = z[i];
      if (y === undefined) continue;
      const r = y - (intercept + slope * i);
      sumRes2 += r * r;
    }
    const sigma = Math.sqrt(sumRes2 / m);
    let changed = false;
    for (let i = 0; i < z.length; i++) {
      if (mask[i] !== true) continue;
      const y = z[i];
      if (y === undefined) continue;
      if (Math.abs(y - (intercept + slope * i)) > krej * sigma) {
        mask[i] = false;
        changed = true;
      }
    }
    if (!changed) break;
  }
  const median = percentile(z, 50);
  const zmin = all[0] ?? median;
  const zmax = all[n - 1] ?? median;
  if (!(slope > 0)) return { lo: zmin, hi: zmax === zmin ? zmin + 1 : zmax };
  const half = (n / 2) * slope * contrast;
  const lo = Math.max(zmin, median - half);
  const hi = Math.min(zmax, median + half);
  return hi > lo ? { lo, hi } : { lo: zmin, hi: zmax === zmin ? zmin + 1 : zmax };
}

export function computeInterval(
  data: Float32Array | readonly number[],
  kind: StretchKind,
): Interval {
  return kind === "zscale" ? zscaleInterval(data) : percentileInterval(data);
}

// Symmetric interval around zero for diverging colormaps (difference frames).
export function symmetricInterval(
  data: Float32Array | readonly number[],
  pct = 98,
): Interval {
  const s = finiteSorted(data);
  if (s.length === 0) return { lo: -1, hi: 1 };
  const abs = s.map((v) => Math.abs(v)).sort((a, b) => a - b);
  const a = percentile(abs, pct);
  const amp = Number.isFinite(a) && a > 0 ? a : (abs[abs.length - 1] ?? 1);
  return { lo: -amp, hi: amp };
}

export const ASINH_SOFTENING = 10;

// Map a normalized position t in [0,1] through the stretch curve.
export function stretchUnit(t: number, kind: StretchKind): number {
  const c = Math.min(Math.max(t, 0), 1);
  switch (kind) {
    case "log":
      return Math.log(1 + c * 999) / Math.log(1000);
    case "asinh":
      return Math.asinh(c * ASINH_SOFTENING) / Math.asinh(ASINH_SOFTENING);
    case "linear":
    case "zscale":
      return c;
  }
}

// Normalize a pixel to [0,1] after stretch. Returns NaN for non-finite input
// so callers can mask it (shader: isnan; 2D fallback: transparent pixel).
export function normalizeValue(
  v: number,
  lo: number,
  hi: number,
  kind: StretchKind,
): number {
  if (!Number.isFinite(v)) return NaN;
  const range = hi - lo;
  if (!(range > 0)) return 0;
  return stretchUnit((v - lo) / range, kind);
}

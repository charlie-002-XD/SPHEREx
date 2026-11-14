// Deterministic synthetic SpectrumResponse for development and tests:
// a smooth continuum plus one absorption dip inside the H2O band, sampled at
// data-dependent wavelengths (never a fixed 102 points).
import type { SpectrumPoint, SpectrumResponse } from "./types";

function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export interface MockSpectrumOptions {
  seed?: number;
  count?: number;
}

export function makeMockSpectrum(
  raDeg: number,
  decDeg: number,
  opts: MockSpectrumOptions = {},
): SpectrumResponse {
  const rand = mulberry32(opts.seed ?? 7);
  const count = opts.count ?? 96;
  const points: SpectrumPoint[] = [];
  for (let i = 0; i < count; i++) {
    const lam = 0.75 + ((5.0 - 0.75) * i) / Math.max(count - 1, 1);
    const continuum = 12 + 6 * Math.exp(-((lam - 1.6) ** 2) / 2) + 0.8 * lam;
    const dip = 3.5 * Math.exp(-((lam - 3.05) ** 2) / (2 * 0.08 ** 2)); // H2O-ish
    const sigma = 0.4 + 0.15 * rand();
    const value = continuum - dip + (rand() - 0.5) * sigma;
    const detector = Math.min(6, 1 + Math.floor((lam - 0.75) / 0.75));
    points.push({
      wavelength_um: lam,
      bandwidth_um: 0.11,
      value: i === count - 1 ? NaN : value, // exercise the NaN path
      sigma: i === 0 ? NaN : sigma, // exercise missing error bars
      obs_mjd: 60200 + i * 2.3,
      detector,
      flags: i % 17 === 0 ? ["PERSIST"] : [],
    });
  }
  return { ra_deg: raDeg, dec_deg: decDeg, unit: "MJy/sr", points };
}

// Near-infrared band groups for the epoch comparator. These are UI
// configuration, not instrument truth: each band is a wavelength window that
// groups whatever frames the archive returns, so the number of bands here is
// a display choice and the channel/detector count of the data is never
// assumed (see CLAUDE.md rule 7).
import type { DecodedFrame } from "./types";

export interface BandDef {
  id: string;
  label: string;
  centerUm: number;
  /** Inclusive lower edge of the window (um). */
  minUm: number;
  /** Exclusive upper edge of the window (um). */
  maxUm: number;
  /** CSS custom property defined in styles/tokens.css. */
  color: string;
  note: string;
}

export const NIR_BANDS: readonly BandDef[] = [
  {
    id: "b1",
    label: "Band 1",
    centerUm: 1.0,
    minUm: 0.7,
    maxUm: 1.3,
    color: "var(--band-1)",
    note: "Short near-IR · hot stars",
  },
  {
    id: "b2",
    label: "Band 2",
    centerUm: 1.6,
    minUm: 1.3,
    maxUm: 2.0,
    color: "var(--band-2)",
    note: "Stellar peak · faint dust",
  },
  {
    id: "b3",
    label: "Band 3",
    centerUm: 2.4,
    minUm: 2.0,
    maxUm: 3.0,
    color: "var(--band-3)",
    note: "Warm dust · ices",
  },
  {
    id: "b4",
    label: "Band 4",
    centerUm: 3.8,
    minUm: 3.0,
    maxUm: 5.1,
    color: "var(--band-4)",
    note: "Long near-IR · nebulae and PAH",
  },
];

export const DEFAULT_BAND_ID = "b2";

/** Half a Julian year: the nominal SPHEREx all-sky revisit interval. */
export const SIX_MONTHS_DAYS = 365.25 / 2;

export function isBandId(v: unknown): v is string {
  return typeof v === "string" && NIR_BANDS.some((b) => b.id === v);
}

export function bandById(id: string): BandDef | null {
  return NIR_BANDS.find((b) => b.id === id) ?? null;
}

export function bandOfWavelength(um: number): BandDef | null {
  if (!Number.isFinite(um)) return null;
  return NIR_BANDS.find((b) => um >= b.minUm && um < b.maxUm) ?? null;
}

export function framesInBand(
  frames: readonly DecodedFrame[],
  band: BandDef,
): DecodedFrame[] {
  return frames
    .filter(
      (f) =>
        f.meta.wavelength_um >= band.minUm && f.meta.wavelength_um < band.maxUm,
    )
    .sort((a, b) => a.meta.obs_mjd_start - b.meta.obs_mjd_start);
}

export function bandFrameCounts(
  frames: readonly DecodedFrame[],
): Record<string, number> {
  const out: Record<string, number> = {};
  for (const b of NIR_BANDS) out[b.id] = 0;
  for (const f of frames) {
    const b = bandOfWavelength(f.meta.wavelength_um);
    if (b !== null) out[b.id] = (out[b.id] ?? 0) + 1;
  }
  return out;
}

export interface EpochPair {
  a: DecodedFrame;
  /** null when the band holds a single epoch. */
  b: DecodedFrame | null;
  /** Observed separation B - A in days; null without an epoch B. */
  dtDays: number | null;
}

// Epoch A is the earliest frame in the band; epoch B is the later frame whose
// separation from A is closest to `targetDays` (six months by default). The
// real separation is returned so the UI never claims a cadence it lacks.
export function selectEpochPair(
  frames: readonly DecodedFrame[],
  band: BandDef,
  targetDays = SIX_MONTHS_DAYS,
): EpochPair | null {
  const inBand = framesInBand(frames, band);
  const a = inBand[0];
  if (a === undefined) return null;
  let best: DecodedFrame | null = null;
  let bestErr = Infinity;
  for (const f of inBand.slice(1)) {
    const dt = f.meta.obs_mjd_start - a.meta.obs_mjd_start;
    if (dt <= 0) continue;
    const err = Math.abs(dt - targetDays);
    if (err < bestErr) {
      best = f;
      bestErr = err;
    }
  }
  return {
    a,
    b: best,
    dtDays:
      best === null ? null : best.meta.obs_mjd_start - a.meta.obs_mjd_start,
  };
}

export function formatDeltaT(days: number): string {
  if (!Number.isFinite(days)) return "—";
  const months = days / (365.25 / 12);
  if (months >= 1) return `${months.toFixed(1)} months`;
  return `${days.toFixed(1)} d`;
}

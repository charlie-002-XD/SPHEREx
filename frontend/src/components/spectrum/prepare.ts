// Pure data preparation for the spectrum plot. Kept DOM/uPlot-free so every
// branch is unit-testable with synthetic payloads (empty, NaN, unsorted).
import type { SpectrumPoint } from "./types";

// Approximate molecular absorption band centers/widths in the SPHEREx range
// (0.75-5.0 um). Sources: Boogert, Gerakines & Whittet 2015, ARA&A 53, 541
// "Observations of the Icy Universe" (H2O ~3.05, CO2 ~4.27, CO ~4.67 um);
// CH4 nu3 band ~3.32 um (Ehrenfreund & Charnley 2000, ARA&A 38, 427).
// Widths are illustrative FWHM-style spans, not instrument line profiles.
export interface MolecularBand {
  key: string;
  label: string;
  rangeUm: readonly [number, number];
}

export const MOLECULAR_BANDS: readonly MolecularBand[] = [
  { key: "h2o", label: "H₂O", rangeUm: [2.95, 3.2] },
  { key: "ch4", label: "CH₄", rangeUm: [3.25, 3.4] },
  { key: "co2", label: "CO₂", rangeUm: [4.2, 4.35] },
  { key: "co", label: "CO", rangeUm: [4.6, 4.75] },
];

export interface PreparedSpectrum {
  // Sorted by wavelength_um. `flux` uses null for non-finite values so the
  // chart library draws a gap; `sigma` is null unless finite and > 0.
  wavelengthUm: number[];
  flux: Array<number | null>;
  sigma: Array<number | null>;
  validCount: number;
}

export function prepareSpectrum(points: readonly SpectrumPoint[]): PreparedSpectrum {
  const sorted = [...points].sort((a, b) => a.wavelength_um - b.wavelength_um);
  const wavelengthUm: number[] = [];
  const flux: Array<number | null> = [];
  const sigma: Array<number | null> = [];
  let validCount = 0;
  for (const p of sorted) {
    wavelengthUm.push(p.wavelength_um);
    const v = Number.isFinite(p.value) ? p.value : null;
    flux.push(v);
    sigma.push(Number.isFinite(p.sigma) && p.sigma > 0 ? p.sigma : null);
    if (v !== null) validCount += 1;
  }
  return { wavelengthUm, flux, sigma, validCount };
}

// Sorted, de-duplicated detector ids present in the payload.
export function detectorsPresent(points: readonly SpectrumPoint[]): number[] {
  const set = new Set<number>();
  for (const p of points) {
    if (Number.isInteger(p.detector)) set.add(p.detector);
  }
  return [...set].sort((a, b) => a - b);
}

export interface FlagCount {
  name: string;
  count: number;
}

// Flag names are API-provided strings (FITS header legend); they are never
// interpreted here, just counted for display.
export function flagCounts(points: readonly SpectrumPoint[]): FlagCount[] {
  const m = new Map<string, number>();
  for (const p of points) {
    for (const f of p.flags ?? []) m.set(f, (m.get(f) ?? 0) + 1);
  }
  return [...m.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
}

export interface BandRect {
  key: string;
  label: string;
  x0: number;
  x1: number;
}

// Pixel-space rectangles for the band shading layer. `umToPx` maps a
// wavelength to canvas x pixels; `xMinUm`/`xMaxUm` is the visible range so
// off-screen bands are skipped. Returns rectangles left-to-right even if the
// axis is reversed.
export function bandRects(
  bands: readonly MolecularBand[],
  umToPx: (um: number) => number,
  xMinUm: number,
  xMaxUm: number,
): BandRect[] {
  const lo = Math.min(xMinUm, xMaxUm);
  const hi = Math.max(xMinUm, xMaxUm);
  const out: BandRect[] = [];
  for (const b of bands) {
    const [b0, b1] = b.rangeUm;
    if (b1 < lo || b0 > hi) continue;
    const a = umToPx(b0);
    const c = umToPx(b1);
    if (!Number.isFinite(a) || !Number.isFinite(c)) continue;
    out.push({ key: b.key, label: b.label, x0: Math.min(a, c), x1: Math.max(a, c) });
  }
  return out;
}

export interface BandLabel {
  key: string;
  label: string;
  x: number;
  // Stagger row; the caller turns it into a y offset. 0 is the top row.
  row: number;
}

// Assigns band labels to slots on staggered rows so no two labels overlap,
// whatever the chart width. Greedy left-to-right: each label is anchored
// at its band's left edge and takes the first row whose previous label
// ends at least `gap` px earlier (widths come from the caller's measurer,
// e.g. ctx.measureText). Labels that fit on no row are dropped — the band
// shading and the Bands toggle/legend still describe them. Pure, so the
// collision rules are unit-testable without a canvas.
export function layoutBandLabels(
  rects: readonly BandRect[],
  measure: (label: string) => number,
  maxRows = 2,
  gap = 6,
): BandLabel[] {
  const sorted = [...rects].sort((a, b) => a.x0 - b.x0);
  const rowEnd: number[] = [];
  const out: BandLabel[] = [];
  for (const r of sorted) {
    const x = r.x0 + 4;
    const w = measure(r.label);
    if (!Number.isFinite(w) || w <= 0) continue;
    for (let row = 0; row < maxRows; row++) {
      if (x >= (rowEnd[row] ?? -Infinity) + gap) {
        rowEnd[row] = x + w;
        out.push({ key: r.key, label: r.label, x, row });
        break;
      }
    }
  }
  return out;
}

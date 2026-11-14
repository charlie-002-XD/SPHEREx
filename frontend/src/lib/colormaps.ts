// Colormap lookup tables (256 RGB entries). Data generated from the
// matplotlib v3.9.4 colormap tables; see colormapData.ts for provenance.
import { COOLWARM_RGB, INFERNO_RGB, VIRIDIS_RGB } from "./colormapData";

export const COLORMAP_NAMES = [
  "grayscale",
  "inferno",
  "viridis",
  "coolwarm",
] as const;
export type ColormapName = (typeof COLORMAP_NAMES)[number];

export function isColormapName(v: unknown): v is ColormapName {
  return (
    typeof v === "string" &&
    (COLORMAP_NAMES as readonly string[]).includes(v)
  );
}

export function isDiverging(name: ColormapName): boolean {
  return name === "coolwarm";
}

function rgbFlat(name: ColormapName): readonly number[] {
  switch (name) {
    case "viridis":
      return VIRIDIS_RGB;
    case "inferno":
      return INFERNO_RGB;
    case "coolwarm":
      return COOLWARM_RGB;
    case "grayscale": {
      const g: number[] = [];
      for (let i = 0; i < 256; i++) g.push(i, i, i);
      return g;
    }
  }
}

const lutCache = new Map<ColormapName, Uint8ClampedArray>();

// 256x4 RGBA LUT suitable for a WebGL texture or canvas ImageData.
export function colormapLut(name: ColormapName): Uint8ClampedArray {
  const hit = lutCache.get(name);
  if (hit !== undefined) return hit;
  const src = rgbFlat(name);
  const lut = new Uint8ClampedArray(256 * 4);
  for (let i = 0; i < 256; i++) {
    lut[i * 4] = src[i * 3] ?? 0;
    lut[i * 4 + 1] = src[i * 3 + 1] ?? 0;
    lut[i * 4 + 2] = src[i * 3 + 2] ?? 0;
    lut[i * 4 + 3] = 255;
  }
  lutCache.set(name, lut);
  return lut;
}

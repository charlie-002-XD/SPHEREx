// Synthetic FramesResponse for development and unit tests: a noisy gradient
// background plus a Gaussian point source that drifts linearly across the
// field. Mirrors shared/openapi-contract.json exactly.
import { float32ToBase64 } from "./base64";
import type { Frame, FramesResponse } from "./types";

export interface MockOptions {
  width?: number;
  height?: number;
  count?: number;
  seed?: number;
}

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

// SPHEREx detector band edges (um), approximate.
const DET_WAVELENGTH = [0.75, 1.11, 1.66, 2.43, 3.32, 4.42];

export function makeMockFramesResponse(opts: MockOptions = {}): FramesResponse {
  const width = opts.width ?? 48;
  const height = opts.height ?? 48;
  const count = opts.count ?? 12;
  const rand = mulberry32(opts.seed ?? 42);
  const px = width * height;
  const scale = 0.05 / width; // deg per pixel, roughly matches fov 0.05

  const frames: Frame[] = [];
  for (let i = 0; i < count; i++) {
    const det = (i % 6) + 1;
    const pixels = new Float32Array(px);
    const cx = width * 0.25 + (width * 0.5 * i) / Math.max(count - 1, 1);
    const cy = height * 0.5 + Math.sin(i * 0.9) * height * 0.1;
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const bg = 5 + 0.02 * x + 0.03 * y;
        const d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
        const src = 40 * Math.exp(-d2 / 4);
        pixels[y * width + x] = bg + src + (rand() - 0.5) * 1.5;
      }
    }
    // a few NaN/Inf pixels to exercise masking
    if (px > 4) {
      pixels[0] = NaN;
      pixels[px - 1] = Infinity;
    }
    const mjd = 60200 + i * 91;
    frames.push({
      id: `mock-${i.toString()}`,
      obs_mjd_start: mjd,
      obs_mjd_end: mjd + 0.01,
      detector: det,
      wavelength_um: DET_WAVELENGTH[det - 1] ?? 1.0,
      bandwidth_um: 0.11,
      collection: "spherex_mock",
      zodi_subtracted: false,
      flags: i % 4 === 0 ? ["PERSIST"] : [],
      wcs: {
        crpix: [width / 2, height / 2],
        crval: [133.795158, -7.245147],
        cd: [[-scale, 0], [0, scale]],
      },
      data_b64: float32ToBase64(pixels),
    });
  }

  return {
    query: { ra_deg: 133.795158, dec_deg: -7.245147, fov_deg: 0.05, mode: "raw" },
    width,
    height,
    unit: "MJy/sr",
    frames,
    flag_legend: { "0": "PERSIST" },
    truncated: false,
    total_observations: count,
  };
}

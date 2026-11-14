// Canvas click -> sky position. The contract's Wcs is a linear approximation
// of the celestial WCS (TAN): crpix is 1-based, cd is deg/pixel. Pure
// functions so they can be unit-tested without a DOM.
import type { Wcs } from "../../lib/types";

export interface SkyCoord {
  ra_deg: number;
  dec_deg: number;
}

// Maps a 0-based pixel coordinate (x right, y down, as in the row-major pixel
// buffer and canvas layout) to ICRS degrees. Returns null for non-finite
// input or a degenerate CD matrix product.
export function pixelToSky(wcs: Wcs, x: number, y: number): SkyCoord | null {
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
  const xi = x + 1 - wcs.crpix[0]; // FITS pixels are 1-based
  const eta = y + 1 - wcs.crpix[1];
  const row0 = wcs.cd[0];
  const row1 = wcs.cd[1];
  if (row0 === undefined || row1 === undefined) return null;
  const ra = wcs.crval[0] + row0[0] * xi + row0[1] * eta;
  const dec = wcs.crval[1] + row1[0] * xi + row1[1] * eta;
  if (!Number.isFinite(ra) || !Number.isFinite(dec)) return null;
  return { ra_deg: ((ra % 360) + 360) % 360, dec_deg: dec };
}

export interface RectLike {
  left: number;
  top: number;
  width: number;
  height: number;
}

// Converts a client-space point (e.g. MouseEvent.clientX/Y) to a 0-based
// pixel coordinate in an image of `width`x`height` shown inside `rect`.
// Returns null when the rect is degenerate or inputs are non-finite.
export function clientPointToPixel(
  rect: RectLike,
  clientX: number,
  clientY: number,
  width: number,
  height: number,
): { x: number; y: number } | null {
  if (
    !Number.isFinite(clientX) ||
    !Number.isFinite(clientY) ||
    !Number.isFinite(rect.width) ||
    !Number.isFinite(rect.height) ||
    rect.width <= 0 ||
    rect.height <= 0 ||
    width <= 0 ||
    height <= 0
  ) {
    return null;
  }
  return {
    x: ((clientX - rect.left) / rect.width) * width,
    y: ((clientY - rect.top) / rect.height) * height,
  };
}

// Full pipeline for a click on the rendered cutout canvas. `canvas` is the
// element that was clicked (must be the cutout canvas, not an overlay).
export function canvasPickToSky(
  canvas: HTMLCanvasElement,
  clientX: number,
  clientY: number,
  wcs: Wcs,
  width: number,
  height: number,
): SkyCoord | null {
  const px = clientPointToPixel(
    canvas.getBoundingClientRect(),
    clientX,
    clientY,
    width,
    height,
  );
  if (px === null) return null;
  return pixelToSky(wcs, px.x, px.y);
}

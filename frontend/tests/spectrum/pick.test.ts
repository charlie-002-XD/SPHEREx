import { describe, expect, it } from "vitest";
import {
  canvasPickToSky,
  clientPointToPixel,
  pixelToSky,
} from "../../src/components/spectrum/pick";
import type { Wcs } from "../../src/lib/types";

// North-up cutout: 100x100 px over a 0.1 deg field centered on the crval.
const wcs: Wcs = {
  crpix: [50, 50],
  crval: [133.795129, -7.245147],
  cd: [[-0.001, 0], [0, 0.001]],
};

describe("pixelToSky", () => {
  it("maps the reference pixel to crval (1-based crpix)", () => {
    // FITS pixel 50 -> 0-based index 49
    const sky = pixelToSky(wcs, 49, 49);
    expect(sky).not.toBeNull();
    expect(sky!.ra_deg).toBeCloseTo(133.795129, 6);
    expect(sky!.dec_deg).toBeCloseTo(-7.245147, 6);
  });

  it("applies the CD matrix (negative RA slope)", () => {
    const sky = pixelToSky(wcs, 59, 49); // +10 px in x
    expect(sky!.ra_deg).toBeCloseTo(133.795129 - 0.01, 6);
    expect(sky!.dec_deg).toBeCloseTo(-7.245147, 6);
  });

  it("supports rotated CD terms", () => {
    const rot: Wcs = {
      crpix: [10, 10],
      crval: [10, 20],
      cd: [[0, -0.001], [0.001, 0]],
    };
    const sky = pixelToSky(rot, 19, 9); // xi=10, eta=0
    expect(sky!.ra_deg).toBeCloseTo(10, 6);
    expect(sky!.dec_deg).toBeCloseTo(20.01, 6);
  });

  it("wraps RA into [0, 360)", () => {
    const s: Wcs = {
      crpix: [1, 1],
      crval: [359.9995, 0],
      cd: [[0.001, 0], [0, 0.001]],
    };
    const sky = pixelToSky(s, 2, 0); // xi=2 -> 359.9995+0.002 = 360.0015
    expect(sky!.ra_deg).toBeCloseTo(0.0015, 6);
  });

  it("returns null for non-finite input", () => {
    expect(pixelToSky(wcs, NaN, 0)).toBeNull();
    expect(pixelToSky(wcs, 0, Infinity)).toBeNull();
    const bad: Wcs = {
      crpix: [NaN, 0],
      crval: [0, 0],
      cd: [[1, 0], [0, 1]],
    };
    expect(pixelToSky(bad, 0, 0)).toBeNull();
  });
});

describe("clientPointToPixel", () => {
  const rect = { left: 10, top: 20, width: 200, height: 100 };

  it("scales client coords into pixel coords", () => {
    const p = clientPointToPixel(rect, 110, 70, 400, 200);
    expect(p).toEqual({ x: 200, y: 100 }); // rect center
  });

  it("maps rect corners to pixel corners", () => {
    expect(clientPointToPixel(rect, 10, 20, 400, 200)).toEqual({ x: 0, y: 0 });
    expect(clientPointToPixel(rect, 210, 120, 400, 200)).toEqual({
      x: 400,
      y: 200,
    });
  });

  it("returns null on degenerate rects and sizes", () => {
    expect(
      clientPointToPixel({ ...rect, width: 0 }, 0, 0, 400, 200),
    ).toBeNull();
    expect(clientPointToPixel(rect, 0, 0, 0, 200)).toBeNull();
    expect(clientPointToPixel(rect, NaN, 0, 400, 200)).toBeNull();
  });
});

describe("canvasPickToSky", () => {
  it("combines rect scaling and WCS projection", () => {
    const canvas = {
      getBoundingClientRect: () => ({
        left: 0,
        top: 0,
        width: 200,
        height: 200,
        right: 200,
        bottom: 200,
        x: 0,
        y: 0,
        toJSON: () => ({}),
      }),
    } as unknown as HTMLCanvasElement;
    // Client (100,100) -> pixel (50,50) in a 100x100 image = FITS pixel 51,
    // one pixel past crpix=50 -> 1 * 0.001 deg offset.
    const sky = canvasPickToSky(canvas, 100, 100, wcs, 100, 100);
    expect(sky).not.toBeNull();
    expect(sky!.ra_deg).toBeCloseTo(133.795129 - 0.001, 6);
    expect(sky!.dec_deg).toBeCloseTo(-7.245147 + 0.001, 6);
  });
});

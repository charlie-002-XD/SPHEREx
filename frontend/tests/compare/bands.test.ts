import { describe, expect, it } from "vitest";
import {
  NIR_BANDS,
  SIX_MONTHS_DAYS,
  bandById,
  bandFrameCounts,
  bandOfWavelength,
  formatDeltaT,
  selectEpochPair,
} from "../../src/lib/bands";
import type { DecodedFrame } from "../../src/lib/types";

function frame(id: string, um: number, mjd: number): DecodedFrame {
  return {
    meta: { id, wavelength_um: um, obs_mjd_start: mjd },
    pixels: new Float32Array(1),
  } as unknown as DecodedFrame;
}

const b1 = bandById("b1");
if (b1 === null) throw new Error("band b1 missing");

describe("band config", () => {
  it("has contiguous, increasing windows", () => {
    for (let i = 1; i < NIR_BANDS.length; i++) {
      const prev = NIR_BANDS[i - 1];
      const cur = NIR_BANDS[i];
      expect(prev?.maxUm).toBe(cur?.minUm);
      expect((cur?.centerUm ?? 0) > (prev?.centerUm ?? 0)).toBe(true);
    }
  });
  it("maps wavelengths to bands, half-open", () => {
    expect(bandOfWavelength(1.0)?.id).toBe("b1");
    expect(bandOfWavelength(1.3)?.id).toBe("b2");
    expect(bandOfWavelength(3.8)?.id).toBe("b4");
    expect(bandOfWavelength(0.5)).toBeNull();
    expect(bandOfWavelength(Number.NaN)).toBeNull();
  });
  it("counts frames per band", () => {
    const counts = bandFrameCounts([
      frame("a", 1.0, 0),
      frame("b", 1.1, 1),
      frame("c", 4.0, 2),
    ]);
    expect(counts).toEqual({ b1: 2, b2: 0, b3: 0, b4: 1 });
  });
});

describe("selectEpochPair", () => {
  it("picks the earliest frame as A and the one closest to +6 months as B", () => {
    const pair = selectEpochPair(
      [
        frame("late", 1.0, 60200 + 400),
        frame("b", 1.0, 60200 + 180),
        frame("a", 1.0, 60200),
        frame("near", 1.0, 60200 + 30),
        frame("other-band", 2.5, 60200 + SIX_MONTHS_DAYS),
      ],
      b1,
    );
    expect(pair?.a.meta.id).toBe("a");
    expect(pair?.b?.meta.id).toBe("b");
    expect(pair?.dtDays).toBe(180);
  });
  it("returns the real separation when no frame is near six months", () => {
    const pair = selectEpochPair(
      [frame("a", 1.0, 100), frame("b", 1.0, 130)],
      b1,
    );
    expect(pair?.b?.meta.id).toBe("b");
    expect(pair?.dtDays).toBe(30);
  });
  it("handles a single epoch and empty bands", () => {
    expect(selectEpochPair([frame("a", 1.0, 1)], b1)).toEqual({
      a: expect.anything() as unknown,
      b: null,
      dtDays: null,
    });
    expect(selectEpochPair([frame("x", 2.5, 1)], b1)).toBeNull();
  });
});

describe("formatDeltaT", () => {
  it("formats months and days", () => {
    expect(formatDeltaT(SIX_MONTHS_DAYS)).toBe("6.0 months");
    expect(formatDeltaT(12)).toBe("12.0 d");
    expect(formatDeltaT(Number.POSITIVE_INFINITY)).toBe("—");
  });
});

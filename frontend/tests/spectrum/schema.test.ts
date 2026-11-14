import { describe, expect, it } from "vitest";
import { spectrumResponseSchema } from "../../src/components/spectrum/types";
import { makeMockSpectrum } from "../../src/components/spectrum/mockSpectrum";
import { flagBitFor } from "../../src/components/metadata/FlagBadges";

describe("spectrumResponseSchema", () => {
  const valid = {
    ra_deg: 133.795,
    dec_deg: -7.245,
    unit: "MJy/sr",
    points: [
      {
        wavelength_um: 1.234,
        bandwidth_um: 0.11,
        value: 12.5,
        sigma: 0.4,
        obs_mjd: 60123.4,
        detector: 2,
        flags: ["SOME_FLAG"],
      },
    ],
  };

  it("accepts a contract-shaped payload", () => {
    const r = spectrumResponseSchema.safeParse(valid);
    expect(r.success).toBe(true);
  });

  it("accepts an empty point list", () => {
    expect(
      spectrumResponseSchema.safeParse({ ...valid, points: [] }).success,
    ).toBe(true);
  });

  it("rejects missing fields, bad detector ids and NaN literals", () => {
    expect(spectrumResponseSchema.safeParse({ ...valid, unit: 3 }).success).toBe(
      false,
    );
    expect(
      spectrumResponseSchema.safeParse({
        ...valid,
        points: [{ ...valid.points[0], detector: 7 }],
      }).success,
    ).toBe(false);
    // JSON on the wire cannot carry NaN; a NaN in a mock object is rejected
    // here and tolerated later at the display layer (prepareSpectrum).
    expect(
      spectrumResponseSchema.safeParse({
        ...valid,
        points: [{ ...valid.points[0], value: NaN }],
      }).success,
    ).toBe(false);
  });

  it("mock spectrum has the contract shape except planted NaN points", () => {
    const res = makeMockSpectrum(133.8, -7.24, { count: 8 });
    expect(res.unit).toBe("MJy/sr");
    expect(res.points).toHaveLength(8);
    const finite = {
      ...res,
      points: res.points.map((p) => ({
        ...p,
        value: Number.isFinite(p.value) ? p.value : 0,
        sigma: Number.isFinite(p.sigma) ? p.sigma : 0.1,
      })),
    };
    expect(spectrumResponseSchema.safeParse(finite).success).toBe(true);
  });
});

describe("flagBitFor", () => {
  const legend = { "0": "PERSIST", "5": "SATURATED", "12": "NONLINEAR" };

  it("maps a flag name to its bit via the API legend", () => {
    expect(flagBitFor(legend, "SATURATED")).toBe("5");
    expect(flagBitFor(legend, "PERSIST")).toBe("0");
  });

  it("returns null for unknown names or missing legend", () => {
    expect(flagBitFor(legend, "BOGUS")).toBeNull();
    expect(flagBitFor(undefined, "PERSIST")).toBeNull();
    expect(flagBitFor({}, "PERSIST")).toBeNull();
  });
});

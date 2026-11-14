import { describe, expect, it } from "vitest";
import {
  computeInterval,
  minMaxInterval,
  normalizeValue,
  percentileInterval,
  stretchUnit,
  symmetricInterval,
  zscaleInterval,
} from "../../src/lib/stretch";

const ramp = (n: number): Float32Array => {
  const a = new Float32Array(n);
  for (let i = 0; i < n; i++) a[i] = i;
  return a;
};

describe("minMaxInterval", () => {
  it("returns finite lo < hi for normal data", () => {
    const iv = minMaxInterval(ramp(100));
    expect(iv.lo).toBe(0);
    expect(iv.hi).toBe(99);
  });
  it("widens a degenerate constant field", () => {
    const iv = minMaxInterval(new Float32Array([5, 5, 5]));
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
  it("ignores NaN and Inf", () => {
    const iv = minMaxInterval(new Float32Array([NaN, 1, 2, Infinity, -Infinity, 3]));
    expect(iv.lo).toBe(1);
    expect(iv.hi).toBe(3);
  });
  it("returns a default for all-NaN input", () => {
    const iv = minMaxInterval(new Float32Array([NaN, NaN]));
    expect(Number.isFinite(iv.lo)).toBe(true);
    expect(Number.isFinite(iv.hi)).toBe(true);
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
});

describe("percentileInterval", () => {
  it("clips outliers", () => {
    const data = ramp(1000);
    data[0] = -1e6;
    data[999] = 1e6;
    const iv = percentileInterval(data, 1, 99);
    expect(iv.lo).toBeGreaterThan(-1e6);
    expect(iv.hi).toBeLessThan(1e6);
  });
  it("falls back to min/max when the range collapses", () => {
    const data = new Float32Array(100).fill(7);
    data[0] = 1;
    const iv = percentileInterval(data);
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
});

describe("zscaleInterval", () => {
  it("returns lo < hi on a ramp", () => {
    const iv = zscaleInterval(ramp(500));
    expect(iv.lo).toBeGreaterThanOrEqual(0);
    expect(iv.hi).toBeLessThanOrEqual(499);
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
  it("is robust to NaN/Inf and tiny inputs", () => {
    const iv = zscaleInterval(new Float32Array([NaN, Infinity, 1, 2, 3]));
    expect(Number.isFinite(iv.lo)).toBe(true);
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
  it("handles all-NaN input", () => {
    const iv = zscaleInterval(new Float32Array([NaN, NaN, NaN]));
    expect(iv.hi).toBeGreaterThan(iv.lo);
  });
});

describe("symmetricInterval", () => {
  it("is centered on zero", () => {
    const iv = symmetricInterval(new Float32Array([-5, -1, 0, 2, 10]));
    expect(iv.lo).toBeCloseTo(-iv.hi, 6);
  });
});

describe("stretchUnit", () => {
  it("clamps input to [0,1]", () => {
    expect(stretchUnit(-1, "linear")).toBe(0);
    expect(stretchUnit(2, "asinh")).toBe(1);
  });
  it("keeps endpoints fixed for every kind", () => {
    for (const k of ["linear", "log", "asinh", "zscale"] as const) {
      expect(stretchUnit(0, k)).toBe(0);
      expect(stretchUnit(1, k)).toBeCloseTo(1, 6);
    }
  });
  it("log and asinh boost mid-tones relative to linear", () => {
    expect(stretchUnit(0.5, "log")).toBeGreaterThan(0.5);
    expect(stretchUnit(0.5, "asinh")).toBeGreaterThan(0.5);
  });
});

describe("normalizeValue", () => {
  it("returns NaN for NaN and Inf", () => {
    expect(Number.isNaN(normalizeValue(NaN, 0, 1, "linear"))).toBe(true);
    expect(Number.isNaN(normalizeValue(Infinity, 0, 1, "linear"))).toBe(true);
  });
  it("maps lo->0 and hi->1", () => {
    expect(normalizeValue(0, 0, 10, "linear")).toBe(0);
    expect(normalizeValue(10, 0, 10, "linear")).toBe(1);
  });
  it("returns 0 on a degenerate interval", () => {
    expect(normalizeValue(5, 3, 3, "linear")).toBe(0);
  });
});

describe("computeInterval", () => {
  it("dispatches zscale vs percentile", () => {
    const data = ramp(200);
    for (const k of ["linear", "log", "asinh", "zscale"] as const) {
      const iv = computeInterval(data, k);
      expect(iv.hi).toBeGreaterThan(iv.lo);
    }
  });
});

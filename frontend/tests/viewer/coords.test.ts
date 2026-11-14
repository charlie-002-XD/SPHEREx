import { describe, expect, it } from "vitest";
import {
  formatDecDeg,
  formatRaDeg,
  isValidFov,
  mjdToIso,
  parseDecToDeg,
  parseRaToDeg,
} from "../../src/lib/coords";

describe("parseRaToDeg", () => {
  it("parses decimal degrees", () => {
    expect(parseRaToDeg("133.795")).toBeCloseTo(133.795, 6);
    expect(parseRaToDeg("0")).toBe(0);
    expect(parseRaToDeg("359.999")).toBeCloseTo(359.999, 6);
    expect(parseRaToDeg("269.45deg")).toBeCloseTo(269.45, 6);
  });
  it("parses sexagesimal hms", () => {
    expect(parseRaToDeg("08h55m10.831s")).toBeCloseTo(133.795129, 4);
    expect(parseRaToDeg("17h57m48.5s")).toBeCloseTo(269.45208, 4);
    expect(parseRaToDeg("08:55:10.831")).toBeCloseTo(133.795129, 4);
  });
  it("rejects out-of-range values", () => {
    expect(parseRaToDeg("360")).toBeNull();
    expect(parseRaToDeg("360.5")).toBeNull();
    expect(parseRaToDeg("-1")).toBeNull();
    expect(parseRaToDeg("24h00m00s")).toBeNull();
  });
  it("rejects malformed input", () => {
    for (const s of ["", "abc", "12h", "1h99m2s", "08h55m61s", "h5m3s", "--5", "1.2.3"]) {
      expect(parseRaToDeg(s)).toBeNull();
    }
  });
});

describe("parseDecToDeg", () => {
  it("parses decimal degrees", () => {
    expect(parseDecToDeg("-7.2451")).toBeCloseTo(-7.2451, 6);
    expect(parseDecToDeg("+30.401")).toBeCloseTo(30.401, 6);
    expect(parseDecToDeg("-90")).toBe(-90);
    expect(parseDecToDeg("90")).toBe(90);
  });
  it("parses sexagesimal dms", () => {
    expect(parseDecToDeg("-07d14m42.53s")).toBeCloseTo(-7.245147, 4);
    expect(parseDecToDeg("+04d41m36s")).toBeCloseTo(4.693333, 4);
    expect(parseDecToDeg("-07:14:42.53")).toBeCloseTo(-7.245147, 4);
  });
  it("rejects out-of-range values", () => {
    expect(parseDecToDeg("90.1")).toBeNull();
    expect(parseDecToDeg("-91")).toBeNull();
    expect(parseDecToDeg("-90d01m00s")).toBeNull();
  });
  it("rejects malformed input", () => {
    for (const s of ["", "xyz", "+-5", "10d99m", "7d30m60s", "d5m3s"]) {
      expect(parseDecToDeg(s)).toBeNull();
    }
  });
});

describe("isValidFov", () => {
  it("enforces 0.01..0.5", () => {
    expect(isValidFov(0.01)).toBe(true);
    expect(isValidFov(0.5)).toBe(true);
    expect(isValidFov(0.009)).toBe(false);
    expect(isValidFov(0.51)).toBe(false);
    expect(isValidFov(NaN)).toBe(false);
    expect(isValidFov(Infinity)).toBe(false);
  });
});

describe("formatting", () => {
  it("round-trips RA through parse", () => {
    const deg = 133.795129;
    const parsed = parseRaToDeg(formatRaDeg(deg));
    expect(parsed).not.toBeNull();
    expect(parsed ?? 0).toBeCloseTo(deg, 4);
  });
  it("round-trips Dec through parse", () => {
    const deg = -7.245147;
    const parsed = parseDecToDeg(formatDecDeg(deg));
    expect(parsed).not.toBeNull();
    expect(parsed ?? 0).toBeCloseTo(deg, 3);
  });
  it("formats negative dec with sign", () => {
    expect(formatDecDeg(-7.245).startsWith("-")).toBe(true);
    expect(formatDecDeg(4.69).startsWith("+")).toBe(true);
  });
  it("handles non-finite input", () => {
    expect(formatRaDeg(NaN)).toBe("—");
    expect(formatDecDeg(Infinity)).toBe("—");
  });
});

describe("mjdToIso", () => {
  it("converts the unix epoch", () => {
    expect(mjdToIso(40587)).toBe("1970-01-01");
    expect(mjdToIso(60200)).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
  it("handles NaN", () => {
    expect(mjdToIso(NaN)).toBe("—");
  });
});

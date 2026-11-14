import { describe, expect, it } from "vitest";
import { base64ToFloat32, float32ToBase64 } from "../../src/lib/base64";
import { makeMockFramesResponse } from "../../src/lib/mock";
import { framesResponseSchema } from "../../src/lib/types";

describe("float32 base64 codec", () => {
  it("round-trips values bit-exactly", () => {
    const src = new Float32Array([0, -1.5, 3.14159, NaN, Infinity, -Infinity, 1e-30]);
    const dec = base64ToFloat32(float32ToBase64(src));
    expect(dec.length).toBe(src.length);
    for (let i = 0; i < src.length; i++) {
      const a = src[i];
      const b = dec[i];
      if (a !== undefined && Number.isNaN(a)) {
        expect(Number.isNaN(b)).toBe(true);
      } else {
        expect(b).toBe(a);
      }
    }
  });
  it("rejects malformed base64", () => {
    expect(() => base64ToFloat32("!!!")).toThrow();
    expect(() => base64ToFloat32("AAAAA")).toThrow();
    expect(() => base64ToFloat32("AQI=")).toThrow(); // 1 byte: not multiple of 4
  });
  it("accepts payloads with padding and whitespace", () => {
    const src = new Float32Array([42]);
    const b64 = float32ToBase64(src);
    expect(base64ToFloat32(`\n ${b64} `)[0]).toBe(42);
  });
});

describe("makeMockFramesResponse", () => {
  it("produces a contract-valid payload", () => {
    const res = makeMockFramesResponse({ width: 8, height: 8, count: 5 });
    expect(() => framesResponseSchema.parse(res)).not.toThrow();
    expect(res.frames).toHaveLength(5);
    for (const f of res.frames) {
      expect(base64ToFloat32(f.data_b64)).toHaveLength(64);
      expect(f.detector).toBeGreaterThanOrEqual(1);
      expect(f.detector).toBeLessThanOrEqual(6);
    }
  });
});

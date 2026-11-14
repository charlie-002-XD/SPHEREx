import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createSpectrumClient,
  spectrumQuery,
} from "../../src/components/spectrum/spectrumApi";
import { ApiError } from "../../src/lib/api";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("spectrumQuery", () => {
  it("encodes ra, dec and aperture_px", () => {
    const q = new URLSearchParams(spectrumQuery(133.795129, -7.245147, 3));
    expect(q.get("ra")).toBe("133.795129");
    expect(q.get("dec")).toBe("-7.245147");
    expect(q.get("aperture_px")).toBe("3");
  });
});

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

const payload = {
  ra_deg: 133.8,
  dec_deg: -7.24,
  unit: "MJy/sr",
  points: [],
};

describe("createSpectrumClient", () => {
  it("fetches and validates the response", async () => {
    const fetchMock = vi.fn(async (input: unknown) => {
      void input;
      return jsonResponse(200, payload);
    });
    vi.stubGlobal("fetch", fetchMock);
    const client = createSpectrumClient("http://api.test/");
    const res = await client.spectrum(133.8, -7.24, 5);
    expect(res.unit).toBe("MJy/sr");
    const url = String(fetchMock.mock.calls[0]?.[0]);
    expect(url).toContain("http://api.test/api/v1/spectrum?");
    expect(url).toContain("aperture_px=5");
  });

  it("throws ApiError with the contract code on error bodies", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        jsonResponse(404, {
          error: { code: "not_found", message: "No observations" },
        }),
      ),
    );
    const client = createSpectrumClient("http://api.test");
    await expect(client.spectrum(1, 2, 3)).rejects.toMatchObject({
      name: "ApiError",
      status: 404,
      code: "not_found",
      message: "No observations",
    });
  });

  it("maps malformed error bodies to a generic ApiError", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => jsonResponse(502, { unexpected: true })),
    );
    const client = createSpectrumClient("http://api.test");
    await expect(client.spectrum(1, 2, 1)).rejects.toBeInstanceOf(ApiError);
    await expect(client.spectrum(1, 2, 1)).rejects.toMatchObject({
      status: 502,
      code: "internal",
    });
  });

  it("rejects invalid payloads", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => jsonResponse(200, { ra_deg: "x" })),
    );
    const client = createSpectrumClient("http://api.test");
    await expect(client.spectrum(1, 2, 3)).rejects.toThrow();
  });
});

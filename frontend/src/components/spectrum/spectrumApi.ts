// API client for /api/v1/spectrum. The base URL is injected by the caller;
// callers may also inject any object satisfying SpectrumClient (tests, mocks).
import { ApiError } from "../../lib/api";
import {
  spectrumResponseSchema,
  type AperturePx,
  type SpectrumResponse,
} from "./types";

export interface SpectrumClient {
  spectrum(
    raDeg: number,
    decDeg: number,
    aperturePx: AperturePx,
    signal?: AbortSignal,
  ): Promise<SpectrumResponse>;
}

export function spectrumQuery(raDeg: number, decDeg: number, aperturePx: AperturePx): string {
  const p = new URLSearchParams();
  p.set("ra", raDeg.toString());
  p.set("dec", decDeg.toString());
  p.set("aperture_px", aperturePx.toString());
  return p.toString();
}

async function parseError(res: Response): Promise<ApiError> {
  try {
    const body: unknown = await res.json();
    if (
      typeof body === "object" &&
      body !== null &&
      "error" in body &&
      typeof (body as { error: unknown }).error === "object"
    ) {
      const err = (body as { error: { code?: unknown; message?: unknown } }).error;
      if (typeof err?.code === "string" && typeof err?.message === "string") {
        return new ApiError(res.status, err.code, err.message);
      }
    }
  } catch {
    // fall through to generic error
  }
  return new ApiError(res.status, "internal", `HTTP ${res.status}`);
}

export function createSpectrumClient(apiBase: string): SpectrumClient {
  const base = apiBase.replace(/\/$/, "");
  return {
    async spectrum(raDeg, decDeg, aperturePx, signal) {
      const res = await fetch(
        `${base}/api/v1/spectrum?${spectrumQuery(raDeg, decDeg, aperturePx)}`,
        { signal: signal ?? null },
      );
      if (!res.ok) throw await parseError(res);
      return spectrumResponseSchema.parse(await res.json());
    },
  };
}

// Frame fetching with cancellation. Pass mockData to bypass the network
// entirely (development, tests, embeds without a backend).
import { useEffect, useRef, useState } from "react";
import { base64ToFloat32 } from "../lib/base64";
import { fetchFrames, type FramesParams } from "../lib/api";
import { parseDecToDeg, parseRaToDeg } from "../lib/coords";
import type { DecodedFrame, FramesResponse } from "../lib/types";

export type FramesStatus = "idle" | "loading" | "success" | "error";

export interface FramesResult {
  status: FramesStatus;
  response: FramesResponse | null;
  frames: DecodedFrame[];
  error: string | null;
}

function decodeAll(res: FramesResponse): DecodedFrame[] {
  const px = res.width * res.height;
  const out: DecodedFrame[] = [];
  for (const meta of res.frames) {
    try {
      const pixels = base64ToFloat32(meta.data_b64);
      if (pixels.length === px) out.push({ meta, pixels });
    } catch {
      // skip undecodable frames rather than failing the whole series
    }
  }
  return out;
}

export function useFrames(
  apiBase: string | null,
  params: Omit<FramesParams, "ra" | "dec"> & { ra: string; dec: string },
  mockData?: FramesResponse,
): FramesResult {
  const [result, setResult] = useState<FramesResult>({
    status: "idle",
    response: null,
    frames: [],
    error: null,
  });
  const seq = useRef(0);

  const raValid = parseRaToDeg(params.ra) !== null;
  const decValid = parseDecToDeg(params.dec) !== null;

  useEffect(() => {
    const id = ++seq.current;
    if (mockData !== undefined) {
      setResult({
        status: "success",
        response: mockData,
        frames: decodeAll(mockData),
        error: null,
      });
      return;
    }
    if (apiBase === null || !raValid || !decValid) {
      setResult({ status: "idle", response: null, frames: [], error: null });
      return;
    }
    const ctrl = new AbortController();
    setResult((prev) => ({ ...prev, status: "loading", error: null }));
    fetchFrames(apiBase, params, ctrl.signal)
      .then((res) => {
        if (seq.current !== id) return;
        setResult({
          status: "success",
          response: res,
          frames: decodeAll(res),
          error: null,
        });
      })
      .catch((err: unknown) => {
        if (seq.current !== id || ctrl.signal.aborted) return;
        setResult({
          status: "error",
          response: null,
          frames: [],
          error: err instanceof Error ? err.message : "request failed",
        });
      });
    return () => ctrl.abort();
  }, [
    apiBase,
    params.ra,
    params.dec,
    params.fov,
    params.mode,
    params.detector,
    params.maxFrames,
    params.subtractZodi,
    mockData,
    raValid,
    decValid,
  ]);

  return result;
}

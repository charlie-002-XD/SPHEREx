// Thin API client. The base URL is always injected by the caller
// (prop/context) — never hardcoded — so the same build works behind any
// deployment path the Rust backend serves.
import {
  errorBodySchema,
  framesResponseSchema,
  targetSchema,
  type ApiMode,
  type FramesResponse,
  type Target,
} from "./types";
import { z } from "zod";

export interface FramesParams {
  ra: string;
  dec: string;
  fov: number;
  mode: ApiMode;
  detector?: number | null;
  maxFrames?: number;
  subtractZodi?: boolean;
}

export function framesQuery(params: FramesParams): string {
  const p = new URLSearchParams();
  p.set("ra", params.ra);
  p.set("dec", params.dec);
  p.set("fov", params.fov.toString());
  p.set("mode", params.mode);
  if (params.detector != null) p.set("detector", params.detector.toString());
  if (params.maxFrames != null) p.set("max_frames", params.maxFrames.toString());
  if (params.subtractZodi === true) p.set("subtract_zodi", "true");
  return p.toString();
}

export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

async function parseError(res: Response): Promise<ApiError> {
  try {
    const body = errorBodySchema.safeParse(await res.json());
    if (body.success) {
      return new ApiError(res.status, body.data.error.code, body.data.error.message);
    }
  } catch {
    // fall through to generic error
  }
  return new ApiError(res.status, "internal", `HTTP ${res.status}`);
}

export async function fetchFrames(
  apiBase: string,
  params: FramesParams,
  signal?: AbortSignal,
): Promise<FramesResponse> {
  const base = apiBase.replace(/\/$/, "");
  const res = await fetch(`${base}/api/v1/frames?${framesQuery(params)}`, {
    signal: signal ?? null,
  });
  if (!res.ok) throw await parseError(res);
  return framesResponseSchema.parse(await res.json());
}

export async function fetchTargets(
  apiBase: string,
  signal?: AbortSignal,
): Promise<Target[]> {
  const base = apiBase.replace(/\/$/, "");
  const res = await fetch(`${base}/api/v1/targets`, { signal: signal ?? null });
  if (!res.ok) throw await parseError(res);
  return z.array(targetSchema).parse(await res.json());
}

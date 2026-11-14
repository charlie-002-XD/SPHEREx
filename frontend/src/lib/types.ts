// Zod schemas mirroring shared/openapi-contract.json (single source of truth).
import { z } from "zod";

export const apiModeSchema = z.enum(["raw", "diff", "coadd", "stack"]);
export type ApiMode = z.infer<typeof apiModeSchema>;
export const API_MODES: readonly ApiMode[] = ["raw", "diff", "coadd", "stack"];

export const wcsSchema = z.object({
  crpix: z.tuple([z.number(), z.number()]),
  crval: z.tuple([z.number(), z.number()]),
  cd: z.tuple([
    z.tuple([z.number(), z.number()]),
    z.tuple([z.number(), z.number()]),
  ]),
});
export type Wcs = z.infer<typeof wcsSchema>;

export const frameSchema = z.object({
  id: z.string(),
  obs_mjd_start: z.number(),
  obs_mjd_end: z.number(),
  detector: z.number().int().min(1).max(6),
  wavelength_um: z.number(),
  bandwidth_um: z.number().optional(),
  collection: z.string(),
  zodi_subtracted: z.boolean().optional(),
  flags: z.array(z.string()).optional(),
  wcs: wcsSchema,
  data_b64: z.string(),
});
export type Frame = z.infer<typeof frameSchema>;

export const framesResponseSchema = z.object({
  query: z.object({
    ra_deg: z.number(),
    dec_deg: z.number(),
    fov_deg: z.number(),
    mode: apiModeSchema,
  }),
  width: z.number().int().positive(),
  height: z.number().int().positive(),
  unit: z.string(),
  frames: z.array(frameSchema),
  flag_legend: z.record(z.string()),
  truncated: z.boolean().optional(),
  total_observations: z.number().int().optional(),
});
export type FramesResponse = z.infer<typeof framesResponseSchema>;

export const targetSchema = z.object({
  id: z.string(),
  name: z.string(),
  ra_deg: z.number(),
  dec_deg: z.number(),
  fov_deg: z.number(),
  note: z.string().optional(),
});
export type Target = z.infer<typeof targetSchema>;

export const errorBodySchema = z.object({
  error: z.object({
    code: z.enum([
      "invalid_parameter",
      "not_found",
      "rate_limited",
      "upstream_unavailable",
      "internal",
    ]),
    message: z.string(),
  }),
});
export type ErrorBody = z.infer<typeof errorBodySchema>;

export const DETECTOR_IDS = [1, 2, 3, 4, 5, 6] as const;

export interface DecodedFrame {
  meta: Frame;
  pixels: Float32Array;
}

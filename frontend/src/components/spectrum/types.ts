// Zod schemas for /api/v1/spectrum, mirroring shared/openapi-contract.json.
// Duplicated here (instead of src/lib/types.ts) because this task's scope is
// limited to components/spectrum; the orchestrator can merge them later.
import { z } from "zod";

export const spectrumPointSchema = z.object({
  wavelength_um: z.number(),
  bandwidth_um: z.number().optional(),
  value: z.number(),
  sigma: z.number(),
  obs_mjd: z.number(),
  detector: z.number().int().min(1).max(6),
  flags: z.array(z.string()).optional(),
});
export type SpectrumPoint = z.infer<typeof spectrumPointSchema>;

export const spectrumResponseSchema = z.object({
  ra_deg: z.number(),
  dec_deg: z.number(),
  unit: z.string(),
  points: z.array(spectrumPointSchema),
});
export type SpectrumResponse = z.infer<typeof spectrumResponseSchema>;

export const APERTURE_PX = [1, 3, 5] as const;
export type AperturePx = (typeof APERTURE_PX)[number];

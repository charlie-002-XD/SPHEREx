// Pure two-way mapping between app state and the URL query string.
// Every key is zod/coords-validated; anything malformed falls back to the
// default so a hand-edited or stale URL can never crash the viewer.
import { z } from "zod";
import { apiModeSchema, type ApiMode } from "./types";
import { isValidFov, parseDecToDeg, parseRaToDeg } from "./coords";
import { isStretchKind, type StretchKind } from "./stretch";
import { isColormapName, type ColormapName } from "./colormaps";
import { DEFAULT_BAND_ID, isBandId } from "./bands";

export const VIEWS = ["timelapse", "compare"] as const;
export type ViewKind = (typeof VIEWS)[number];

export function isViewKind(v: unknown): v is ViewKind {
  return typeof v === "string" && (VIEWS as readonly string[]).includes(v);
}

export interface UrlState {
  ra: string;
  dec: string;
  fov: number;
  mode: ApiMode;
  stretch: StretchKind;
  cmap: ColormapName;
  fps: number;
  frame: number;
  embed: boolean;
  view: ViewKind;
  band: string;
}

export const DEFAULT_URL_STATE: UrlState = {
  ra: "",
  dec: "",
  fov: 0.05,
  mode: "raw",
  stretch: "linear",
  cmap: "grayscale",
  fps: 5,
  frame: 0,
  embed: false,
  view: "compare",
  band: DEFAULT_BAND_ID,
};

export const FPS_MIN = 1;
export const FPS_MAX = 30;

const fovSchema = z.number().refine(isValidFov);
const fpsSchema = z.number().int().min(FPS_MIN).max(FPS_MAX);
const frameSchemaUrl = z.number().int().min(0);

function num(v: string | null): number | null {
  if (v === null) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

export function parseUrlState(search: string): UrlState {
  const p = new URLSearchParams(search.startsWith("?") ? search : `?${search}`);
  const st = { ...DEFAULT_URL_STATE };

  const ra = p.get("ra");
  if (ra !== null && parseRaToDeg(ra) !== null) st.ra = ra.trim();
  const dec = p.get("dec");
  if (dec !== null && parseDecToDeg(dec) !== null) st.dec = dec.trim();

  const fov = num(p.get("fov"));
  if (fov !== null && fovSchema.safeParse(fov).success) st.fov = fov;

  const mode = p.get("mode");
  const modeParsed = apiModeSchema.safeParse(mode);
  if (modeParsed.success) st.mode = modeParsed.data;

  const stretch = p.get("stretch");
  if (isStretchKind(stretch)) st.stretch = stretch;
  const cmap = p.get("cmap");
  if (isColormapName(cmap)) st.cmap = cmap;

  const fps = num(p.get("fps"));
  if (fps !== null && fpsSchema.safeParse(fps).success) st.fps = fps;
  const frame = num(p.get("frame"));
  if (frame !== null && frameSchemaUrl.safeParse(frame).success) {
    st.frame = frame;
  }

  const embed = p.get("embed");
  st.embed = embed === "1" || embed === "true";

  const view = p.get("view");
  if (isViewKind(view)) st.view = view;
  const band = p.get("band");
  if (isBandId(band)) st.band = band;

  return st;
}

export function serializeUrlState(st: UrlState): string {
  const p = new URLSearchParams();
  if (st.ra !== "") p.set("ra", st.ra);
  if (st.dec !== "") p.set("dec", st.dec);
  if (st.fov !== DEFAULT_URL_STATE.fov) p.set("fov", st.fov.toString());
  if (st.mode !== DEFAULT_URL_STATE.mode) p.set("mode", st.mode);
  if (st.stretch !== DEFAULT_URL_STATE.stretch) p.set("stretch", st.stretch);
  if (st.cmap !== DEFAULT_URL_STATE.cmap) p.set("cmap", st.cmap);
  if (st.fps !== DEFAULT_URL_STATE.fps) p.set("fps", st.fps.toString());
  if (st.frame !== 0) p.set("frame", st.frame.toString());
  if (st.embed) p.set("embed", "1");
  if (st.view !== DEFAULT_URL_STATE.view) p.set("view", st.view);
  if (st.band !== DEFAULT_URL_STATE.band) p.set("band", st.band);
  const s = p.toString();
  return s === "" ? "" : `?${s}`;
}

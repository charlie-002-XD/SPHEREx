// Sky coordinate parsing/formatting with the same valid ranges as the
// backend: 0 <= ra < 360 deg, -90 <= dec <= 90 deg, 0.01 <= fov <= 0.5 deg.
// RA accepts decimal degrees or sexagesimal hours (08h55m10.831s / 08:55:10.831).
// Dec accepts decimal degrees or sexagesimal degrees (-07d14m42.53s / -07:14:42.53).

export const RA_MIN_DEG = 0;
export const RA_MAX_DEG = 360;
export const DEC_MIN_DEG = -90;
export const DEC_MAX_DEG = 90;
export const FOV_MIN_DEG = 0.01;
export const FOV_MAX_DEG = 0.5;

const RA_SEXA_RE =
  /^(\d{1,3})\s*(?:h|:)\s*(\d{1,2})\s*(?:m|:)\s*(\d{1,2}(?:\.\d+)?)\s*s?$/i;
const DEC_SEXA_RE =
  /^([+-]?)(\d{1,3})\s*(?:d|°|:)\s*(\d{1,2})\s*(?:m|'|:)\s*(\d{1,2}(?:\.\d+)?)\s*(?:s|")?$/i;
const DECIMAL_RE = /^[+-]?\d+(?:\.\d+)?(?:\s*(?:d|deg|°))?$/i;

function parts60(min: string, sec: string): number | null {
  const m = Number(min);
  const s = Number(sec);
  if (!Number.isFinite(m) || !Number.isFinite(s) || m >= 60 || s >= 60) {
    return null;
  }
  return m / 60 + s / 3600;
}

export function parseRaToDeg(input: string): number | null {
  const s = input.trim();
  if (s === "") return null;
  const sexa = RA_SEXA_RE.exec(s);
  if (sexa !== null) {
    const h = Number(sexa[1]);
    const rest = parts60(sexa[2] ?? "", sexa[3] ?? "");
    if (rest === null || !Number.isFinite(h)) return null;
    const deg = (h + rest) * 15;
    return deg >= RA_MIN_DEG && deg < RA_MAX_DEG ? deg : null;
  }
  if (DECIMAL_RE.test(s)) {
    const deg = Number.parseFloat(s);
    return Number.isFinite(deg) && deg >= RA_MIN_DEG && deg < RA_MAX_DEG
      ? deg
      : null;
  }
  return null;
}

export function parseDecToDeg(input: string): number | null {
  const s = input.trim();
  if (s === "") return null;
  const sexa = DEC_SEXA_RE.exec(s);
  if (sexa !== null) {
    const d = Number(sexa[2]);
    const rest = parts60(sexa[3] ?? "", sexa[4] ?? "");
    if (rest === null || !Number.isFinite(d)) return null;
    const deg = (sexa[1] === "-" ? -1 : 1) * (d + rest);
    return deg >= DEC_MIN_DEG && deg <= DEC_MAX_DEG ? deg : null;
  }
  if (DECIMAL_RE.test(s)) {
    const deg = Number.parseFloat(s);
    return Number.isFinite(deg) && deg >= DEC_MIN_DEG && deg <= DEC_MAX_DEG
      ? deg
      : null;
  }
  return null;
}

export function isValidFov(fov: number): boolean {
  return Number.isFinite(fov) && fov >= FOV_MIN_DEG && fov <= FOV_MAX_DEG;
}

function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

export function formatRaDeg(deg: number, secDecimals = 2): string {
  if (!Number.isFinite(deg)) return "—";
  const totalHours = (((deg % 360) + 360) % 360) / 15;
  const h = Math.floor(totalHours);
  const mFloat = (totalHours - h) * 60;
  const m = Math.floor(mFloat);
  const s = (mFloat - m) * 60;
  return `${pad2(h)}h${pad2(m)}m${s.toFixed(secDecimals).padStart(3 + secDecimals, "0")}s`;
}

export function formatDecDeg(deg: number, secDecimals = 1): string {
  if (!Number.isFinite(deg)) return "—";
  const sign = deg < 0 ? "-" : "+";
  const a = Math.abs(deg);
  const d = Math.floor(a);
  const mFloat = (a - d) * 60;
  const m = Math.floor(mFloat);
  const s = (mFloat - m) * 60;
  return `${sign}${pad2(d)}d${pad2(m)}m${s.toFixed(secDecimals).padStart(3 + secDecimals, "0")}s`;
}

const MJD_UNIX_EPOCH = 40587; // MJD at 1970-01-01T00:00:00Z

export function mjdToIso(mjd: number): string {
  if (!Number.isFinite(mjd)) return "—";
  const ms = (mjd - MJD_UNIX_EPOCH) * 86400_000;
  const d = new Date(ms);
  return Number.isNaN(d.getTime()) ? "—" : d.toISOString().slice(0, 10);
}

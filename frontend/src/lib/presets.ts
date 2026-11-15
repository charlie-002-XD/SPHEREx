// Preset pills for the five official challenge targets. WISE 0855 uses the
// coordinates published on the challenge page; the other four come from the
// team plan table (to be verified against SIMBAD before final submission).
import { parseDecToDeg, parseRaToDeg } from "./coords";

export interface PresetTarget {
  id: string;
  label: string;
  ra: string;
  dec: string;
  fov: number;
  note?: string;
}

export const PRESET_TARGETS: readonly PresetTarget[] = [
  {
    id: "barnards-star",
    label: "Barnard's Star",
    ra: "17h57m48.5s",
    dec: "+04d41m36s",
    fov: 0.1,
    note: "Highest proper-motion star known",
  },
  {
    id: "epsilon-indi-b",
    label: "Epsilon Indi B",
    ra: "22h04m10.5s",
    dec: "-56d46m58s",
    fov: 0.05,
  },
  {
    id: "wise-0855",
    label: "WISE 0855",
    ra: "08h55m10.831s",
    dec: "-07d14m42.53s",
    fov: 0.05,
    note: "Official challenge coordinates",
  },
  {
    id: "rw-aurigae",
    label: "RW Aurigae",
    ra: "05h07m49.5s",
    dec: "+30d24m05s",
    fov: 0.05,
  },
  {
    id: "r-draconis",
    label: "R Draconis",
    ra: "16h32m40.2s",
    dec: "+66d45m17s",
    fov: 0.05,
  },
];

// Loaded when the page opens with no position so the viewer never starts
// empty. The bundled mock series (lib/mock.ts) covers the same position
// and stands in when the backend does not answer.
export const DEMO_TARGET: PresetTarget = {
  id: "wise-0855",
  label: "WISE 0855",
  ra: "08h55m10.831s",
  dec: "-07d14m42.53s",
  fov: 0.05,
};

const DEMO_TOLERANCE_DEG = 0.01;

export function isDemoTarget(ra: string, dec: string): boolean {
  const r = parseRaToDeg(ra);
  const d = parseDecToDeg(dec);
  const r0 = parseRaToDeg(DEMO_TARGET.ra);
  const d0 = parseDecToDeg(DEMO_TARGET.dec);
  if (r === null || d === null || r0 === null || d0 === null) return false;
  return (
    Math.abs(r - r0) < DEMO_TOLERANCE_DEG &&
    Math.abs(d - d0) < DEMO_TOLERANCE_DEG
  );
}

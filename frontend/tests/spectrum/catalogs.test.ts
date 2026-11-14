import { describe, expect, it } from "vitest";
import {
  ALADIN_ATTRIBUTION,
  ALADIN_URL,
  ALADIN_VERSION,
  CATALOGS,
} from "../../src/components/overlays/aladin";
import { UPLOT_VERSION } from "../../src/components/spectrum/uplot";

describe("pinned third-party builds", () => {
  it("Aladin Lite is a fixed version, not a floating channel", () => {
    expect(ALADIN_URL).toBe(
      `https://aladin.cds.unistra.fr/AladinLite/api/v3/${ALADIN_VERSION}/aladin.js`,
    );
    expect(ALADIN_URL).not.toContain("latest");
    expect(ALADIN_URL).not.toContain("beta");
    expect(ALADIN_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });

  it("uPlot version is a semver pin", () => {
    expect(UPLOT_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

describe("CATALOGS", () => {
  it("declares Gaia DR3 and AllWISE with VizieR ids", () => {
    const ids = new Map(CATALOGS.map((c) => [c.key, c.vizierId]));
    expect(ids.get("gaia")).toBe("I/355/gaiadr3");
    expect(ids.get("allwise")).toBe("II/328/allwise");
    expect(CATALOGS).toHaveLength(2);
  });

  it("has unique keys and non-empty attributions", () => {
    const keys = CATALOGS.map((c) => c.key);
    expect(new Set(keys).size).toBe(keys.length);
    for (const c of CATALOGS) {
      expect(c.attribution.length).toBeGreaterThan(0);
      expect(c.label.length).toBeGreaterThan(0);
    }
  });

  it("attribution covers CDS, Gaia and AllWISE", () => {
    const all = `${ALADIN_ATTRIBUTION} ${CATALOGS.map((c) => c.attribution).join(" ")}`;
    expect(all).toContain("CDS");
    expect(all).toContain("Gaia DR3");
    expect(all).toContain("AllWISE");
  });
});

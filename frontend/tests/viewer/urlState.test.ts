import { describe, expect, it } from "vitest";
import {
  DEFAULT_URL_STATE,
  parseUrlState,
  serializeUrlState,
} from "../../src/lib/urlState";

describe("parseUrlState", () => {
  it("returns defaults for an empty query", () => {
    expect(parseUrlState("")).toEqual(DEFAULT_URL_STATE);
    expect(parseUrlState("?")).toEqual(DEFAULT_URL_STATE);
  });
  it("parses a full valid query", () => {
    const st = parseUrlState(
      "?ra=133.79&dec=-7.24&fov=0.1&mode=diff&stretch=asinh&cmap=viridis&fps=12&frame=7&embed=1",
    );
    expect(st).toEqual({
      ra: "133.79",
      dec: "-7.24",
      fov: 0.1,
      mode: "diff",
      stretch: "asinh",
      cmap: "viridis",
      fps: 12,
      frame: 7,
      embed: true,
    });
  });
  it("accepts sexagesimal coordinates", () => {
    const st = parseUrlState("?ra=08h55m10.831s&dec=-07d14m42.53s");
    expect(st.ra).toBe("08h55m10.831s");
    expect(st.dec).toBe("-07d14m42.53s");
  });
  it("falls back per-field on malformed values", () => {
    const st = parseUrlState(
      "?ra=999&dec=abc&fov=99&mode=bogus&stretch=nope&cmap=nope&fps=0&frame=-3&embed=yes",
    );
    expect(st).toEqual(DEFAULT_URL_STATE);
  });
  it("clamps nothing but validates ranges", () => {
    expect(parseUrlState("?fps=30").fps).toBe(30);
    expect(parseUrlState("?fps=31").fps).toBe(DEFAULT_URL_STATE.fps);
    expect(parseUrlState("?fov=0.5").fov).toBe(0.5);
    expect(parseUrlState("?fov=0.001").fov).toBe(DEFAULT_URL_STATE.fov);
  });
  it("round-trips through serialize", () => {
    const st = parseUrlState(
      "?ra=133.79&dec=-7.24&fov=0.2&mode=stack&stretch=log&cmap=coolwarm&fps=8&frame=3&embed=1",
    );
    const st2 = parseUrlState(serializeUrlState(st));
    expect(st2).toEqual(st);
  });
  it("omits defaults on serialize", () => {
    expect(serializeUrlState(DEFAULT_URL_STATE)).toBe("");
  });
});

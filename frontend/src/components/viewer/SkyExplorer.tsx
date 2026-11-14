// Composed viewer: URL state + search bar + mode selector + frame fetch +
// time-lapse canvas. Pass apiBase to talk to the backend, or mockData for a
// fully offline view. embed=1 (URL or prop) hides all chrome except the
// canvas and the minimal transport.
import { useState } from "react";
import { TimeLapseCanvas, type TransportState } from "./TimeLapseCanvas";
import { CoordinateSearchBar } from "../controls/CoordinateSearchBar";
import { ModeSelector } from "../controls/ModeSelector";
import { useUrlState } from "../../hooks/useUrlState";
import { useFrames } from "../../hooks/useFrames";
import { STRETCH_KINDS, isStretchKind } from "../../lib/stretch";
import { COLORMAP_NAMES, isColormapName } from "../../lib/colormaps";
import type { FramesResponse } from "../../lib/types";
import "./viewer.css";

export interface SkyExplorerProps {
  apiBase?: string | null;
  mockData?: FramesResponse;
  embed?: boolean;
}

export function SkyExplorer(props: SkyExplorerProps) {
  const { apiBase = null, mockData, embed: embedProp } = props;
  const [url, setUrl] = useUrlState();
  const embed = embedProp ?? url.embed;
  const [detector, setDetector] = useState<number | null>(null);
  const [local, setLocal] = useState({ playing: false, loop: true, blink: false });

  const framesResult = useFrames(
    apiBase,
    {
      ra: url.ra,
      dec: url.dec,
      fov: url.fov,
      mode: url.mode,
      detector,
      maxFrames: 60,
    },
    mockData,
  );

  const transport: TransportState = {
    playing: local.playing,
    frameIndex: url.frame,
    fps: url.fps,
    loop: local.loop,
    blink: local.blink,
  };

  const onTransport = (patch: Partial<TransportState>) => {
    const u: Partial<typeof url> = {};
    if (patch.frameIndex !== undefined) u.frame = patch.frameIndex;
    if (patch.fps !== undefined) u.fps = patch.fps;
    if (patch.playing !== undefined || patch.loop !== undefined || patch.blink !== undefined) {
      setLocal((prev) => ({
        playing: patch.playing ?? prev.playing,
        loop: patch.loop ?? prev.loop,
        blink: patch.blink ?? prev.blink,
      }));
    }
    setUrl(u);
  };

  const width = framesResult.response?.width ?? 64;
  const height = framesResult.response?.height ?? 64;

  return (
    <section className={embed ? "sx sx-embed" : "sx"}>
      {!embed ? (
        <>
          <CoordinateSearchBar
            value={{ ra: url.ra, dec: url.dec, fov: url.fov }}
            loading={framesResult.status === "loading"}
            onSubmit={(v) => setUrl({ ...v, frame: 0 })}
          />
          <ModeSelector
            mode={url.mode}
            detector={detector}
            onChange={(p) => {
              if (p.mode !== undefined) setUrl({ mode: p.mode, frame: 0 });
              if (p.detector !== undefined) setDetector(p.detector);
            }}
          />
          <div className="sx-display">
            <label>
              <span>Stretch </span>
              <select
                aria-label="Display stretch"
                value={url.stretch}
                onChange={(e) => {
                  const v = e.currentTarget.value;
                  if (isStretchKind(v)) setUrl({ stretch: v });
                }}
              >
                {STRETCH_KINDS.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>Colormap </span>
              <select
                aria-label="Colormap"
                value={url.cmap}
                onChange={(e) => {
                  const v = e.currentTarget.value;
                  if (isColormapName(v)) setUrl({ cmap: v });
                }}
              >
                {COLORMAP_NAMES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </>
      ) : null}
      {framesResult.status === "error" ? (
        <p className="sx-error" role="alert">
          {framesResult.error}
        </p>
      ) : null}
      {framesResult.status === "loading" ? (
        <p className="sx-status" role="status">
          Loading frames…
        </p>
      ) : null}
      <TimeLapseCanvas
        frames={framesResult.frames}
        width={width}
        height={height}
        unit={framesResult.response?.unit}
        stretch={url.stretch}
        cmap={url.cmap}
        transport={transport}
        onTransport={onTransport}
        embed={embed}
      />
      {!embed && framesResult.response?.truncated === true ? (
        <p className="sx-status">
          Series truncated at {framesResult.frames.length} of{" "}
          {framesResult.response.total_observations ?? "?"} observations.
        </p>
      ) : null}
    </section>
  );
}

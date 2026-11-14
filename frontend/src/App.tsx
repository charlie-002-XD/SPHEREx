// Composition root: wires URL state, the f1 viewer/controls and the API
// client into the app shell. apiBase is injected and defaults to "" so
// requests hit the backend's /api/v1 routes on the same origin (the client
// in lib/api.ts appends /api/v1 itself).
import { useCallback, useEffect, useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import type { MenuTarget } from "./components/layout/TargetMenu";
import {
  EmptyState,
  ErrorState,
  LoadingSkeleton,
} from "./components/layout/States";
import {
  CoordinateSearchBar,
  type CoordinateValue,
} from "./components/controls/CoordinateSearchBar";
import { ModeSelector } from "./components/controls/ModeSelector";
import {
  TimeLapseCanvas,
  type TransportState,
} from "./components/viewer/TimeLapseCanvas";
import { useUrlState } from "./hooks/useUrlState";
import { useFrames, type FramesStatus } from "./hooks/useFrames";
import { fetchTargets } from "./lib/api";
import { formatDecDeg, formatRaDeg } from "./lib/coords";
import { PRESET_TARGETS } from "./lib/presets";
import { isStretchKind, STRETCH_KINDS } from "./lib/stretch";
import { COLORMAP_NAMES, isColormapName } from "./lib/colormaps";
import type { FramesResponse } from "./lib/types";
import type { UrlState } from "./lib/urlState";

const DEFAULT_API_BASE = (import.meta.env.VITE_API_BASE ?? "") as string;

export interface AppProps {
  apiBase?: string;
  mockData?: FramesResponse;
}

interface ExplorerProps {
  apiBase: string | null;
  url: UrlState;
  setUrl: (patch: Partial<UrlState>) => void;
  detector: number | null;
  embed: boolean;
  mockData?: FramesResponse;
  onStatusChange: (s: FramesStatus) => void;
  onRetry: () => void;
}

// Owns frame fetching and transport state for the main viewer area. App
// remounts it (key change) on Retry, which re-runs the fetch effect.
function Explorer(props: ExplorerProps) {
  const { url, setUrl, detector, embed, apiBase, mockData } = props;
  const [local, setLocal] = useState({
    playing: false,
    loop: true,
    blink: false,
  });
  const res = useFrames(
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

  const { onStatusChange } = props;
  useEffect(() => onStatusChange(res.status), [res.status, onStatusChange]);

  const transport: TransportState = {
    playing: local.playing,
    frameIndex: url.frame,
    fps: url.fps,
    loop: local.loop,
    blink: local.blink,
  };

  const onTransport = (patch: Partial<TransportState>) => {
    const u: Partial<UrlState> = {};
    if (patch.frameIndex !== undefined) u.frame = patch.frameIndex;
    if (patch.fps !== undefined) u.fps = patch.fps;
    if (
      patch.playing !== undefined ||
      patch.loop !== undefined ||
      patch.blink !== undefined
    ) {
      setLocal((prev) => ({
        playing: patch.playing ?? prev.playing,
        loop: patch.loop ?? prev.loop,
        blink: patch.blink ?? prev.blink,
      }));
    }
    setUrl(u);
  };

  if (res.status === "error") {
    return (
      <ErrorState
        message={res.error ?? "Request failed."}
        onRetry={props.onRetry}
      />
    );
  }
  if (res.status === "idle") {
    return <EmptyState />;
  }
  if (res.status === "loading" && res.frames.length === 0) {
    return <LoadingSkeleton />;
  }
  if (res.frames.length === 0) {
    return (
      <EmptyState
        title="No observations"
        hint="SPHEREx has not covered this position yet, or no frames could be decoded."
      />
    );
  }

  const width = res.response?.width ?? 64;
  const height = res.response?.height ?? 64;

  return (
    <>
      {res.status === "loading" ? (
        <p className="app-status" role="status">
          Updating…
        </p>
      ) : null}
      <TimeLapseCanvas
        frames={res.frames}
        width={width}
        height={height}
        unit={res.response?.unit}
        stretch={url.stretch}
        cmap={url.cmap}
        transport={transport}
        onTransport={onTransport}
        embed={embed}
      />
      {!embed && res.response?.truncated === true ? (
        <p className="app-status">
          Series truncated at {res.frames.length} of{" "}
          {res.response.total_observations ?? "?"} observations.
        </p>
      ) : null}
    </>
  );
}

export function App(props: AppProps) {
  const { apiBase = DEFAULT_API_BASE, mockData } = props;
  const [url, setUrl] = useUrlState();
  const [detector, setDetector] = useState<number | null>(null);
  const [status, setStatus] = useState<FramesStatus>("idle");
  const [retryNonce, setRetryNonce] = useState(0);
  const [targets, setTargets] =
    useState<readonly MenuTarget[]>(PRESET_TARGETS);

  // The preset menu prefers server-provided targets (/api/v1/targets) and
  // silently keeps the built-in presets when the backend is unreachable.
  useEffect(() => {
    if (mockData !== undefined) return;
    const ctrl = new AbortController();
    fetchTargets(apiBase, ctrl.signal)
      .then((ts) => {
        if (ts.length === 0) return;
        setTargets(
          ts.map((t) => ({
            id: t.id,
            label: t.name,
            ra: formatRaDeg(t.ra_deg),
            dec: formatDecDeg(t.dec_deg),
            fov: t.fov_deg,
            note: t.note,
          })),
        );
      })
      .catch(() => {
        /* offline or backend down: keep PRESET_TARGETS */
      });
    return () => ctrl.abort();
  }, [apiBase, mockData]);

  const submitCoords = useCallback(
    (v: CoordinateValue) => setUrl({ ...v, frame: 0 }),
    [setUrl],
  );

  const panel = (
    <div className="app-controls">
      <section className="app-group" aria-labelledby="app-grp-mode">
        <h2 className="app-group-title" id="app-grp-mode">
          Mode &amp; detector
        </h2>
        <ModeSelector
          mode={url.mode}
          detector={detector}
          onChange={(p) => {
            if (p.mode !== undefined) setUrl({ mode: p.mode, frame: 0 });
            if (p.detector !== undefined) setDetector(p.detector);
          }}
        />
      </section>
      <section className="app-group" aria-labelledby="app-grp-display">
        <h2 className="app-group-title" id="app-grp-display">
          Display
        </h2>
        <div className="app-fields">
          <label className="app-field">
            <span>Stretch</span>
            <select
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
          <label className="app-field">
            <span>Colormap</span>
            <select
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
      </section>
    </div>
  );

  return (
    <AppShell
      embed={url.embed}
      targets={targets}
      onSelectTarget={submitCoords}
      search={
        <CoordinateSearchBar
          value={{ ra: url.ra, dec: url.dec, fov: url.fov }}
          loading={status === "loading"}
          onSubmit={submitCoords}
        />
      }
      panel={panel}
    >
      <Explorer
        key={retryNonce}
        apiBase={apiBase}
        url={url}
        setUrl={setUrl}
        detector={detector}
        embed={url.embed}
        mockData={mockData}
        onStatusChange={setStatus}
        onRetry={() => setRetryNonce((n) => n + 1)}
      />
    </AppShell>
  );
}

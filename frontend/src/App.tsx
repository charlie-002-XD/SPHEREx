// Composition root: wires URL state, the f1 viewer/controls, the f2
// spectrum/metadata/overlay components and the API client into the f4 app
// shell. apiBase is injected and defaults to "" so requests hit the
// backend's /api/v1 routes on the same origin (the client in lib/api.ts
// appends /api/v1 itself). ?mock=1 swaps every wire payload for the
// synthetic mocks so the full UI runs with no backend (demos, e2e tests).
import {
  Suspense,
  lazy,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { AppShell } from "./components/layout/AppShell";
import { AboutDialog } from "./components/about/AboutDialog";
import { FirstRunHint } from "./components/about/FirstRunHint";
import { EmbedCredit } from "./components/about/EmbedCredit";
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
import { SpectrumPanel } from "./components/spectrum/SpectrumPanel";
import {
  canvasClickToSpectrum,
  useSpectrum,
} from "./components/spectrum/useSpectrum";
import {
  createSpectrumClient,
  type SpectrumClient,
} from "./components/spectrum/spectrumApi";
import { makeMockSpectrum } from "./components/spectrum/mockSpectrum";
import { FrameMetadata } from "./components/metadata/FrameMetadata";
import { CompareView } from "./components/compare/CompareView";
import { BandSelector } from "./components/compare/BandSelector";
import { FindingsList } from "./components/compare/FindingsList";
import type { PendingMark } from "./components/compare/EpochComparator";
import { bandById, bandFrameCounts, selectEpochPair } from "./lib/bands";
import {
  createLocalFindingsClient,
  type Finding,
  type FindingKind,
  type FindingsClient,
} from "./lib/findings";
import { useUrlState } from "./hooks/useUrlState";
import { useFrames, type FramesResult } from "./hooks/useFrames";
import { fetchTargets } from "./lib/api";
import {
  formatDecDeg,
  formatRaDeg,
  parseDecToDeg,
  parseRaToDeg,
} from "./lib/coords";
import { makeMockFramesResponse } from "./lib/mock";
import { PRESET_TARGETS } from "./lib/presets";
import { isStretchKind, STRETCH_KINDS } from "./lib/stretch";
import { COLORMAP_NAMES, isColormapName } from "./lib/colormaps";
import type { FramesResponse } from "./lib/types";
import type { UrlState, ViewKind } from "./lib/urlState";

const DEFAULT_API_BASE = (import.meta.env.VITE_API_BASE ?? "") as string;

// The Aladin chunk (and the CDN script it loads) only downloads when the
// user enables the overlay.
const AladinOverlay = lazy(() =>
  import("./components/overlays/AladinOverlay").then((m) => ({
    default: m.AladinOverlay,
  })),
);

// ?mock=1 renders the full UI from the synthetic payloads (frames +
// spectrum) with no backend — used for demos and Playwright.
function mockModeEnabled(): boolean {
  if (typeof window === "undefined") return false;
  return new URLSearchParams(window.location.search).get("mock") === "1";
}

// Aperture is a spectrum-request parameter; the mock ignores it because the
// synthetic response does not depend on the aperture size.
function createMockSpectrumClient(): SpectrumClient {
  return {
    spectrum: (raDeg, decDeg) =>
      Promise.resolve(makeMockSpectrum(raDeg, decDeg)),
  };
}

const VIEW_OPTIONS: ReadonlyArray<{ id: ViewKind; label: string }> = [
  { id: "timelapse", label: "Time-lapse" },
  { id: "compare", label: "Compare A/B" },
];

function ViewSwitch(props: {
  value: ViewKind;
  onChange: (v: ViewKind) => void;
}) {
  return (
    <div className="vsw">
      <div className="ms-group" role="radiogroup" aria-label="Viewer">
        {VIEW_OPTIONS.map((o) => (
          <button
            key={o.id}
            type="button"
            role="radio"
            aria-checked={props.value === o.id}
            className={props.value === o.id ? "ms-opt ms-on" : "ms-opt"}
            onClick={() => props.onChange(o.id)}
          >
            {o.label}
          </button>
        ))}
      </div>
    </div>
  );
}

// Disclosure chevron for the collapsible panel sections.
function GroupChevron() {
  return (
    <svg
      className="app-group-chev"
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      <path
        d="M4 6l4 4 4-4"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

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
  onPick: (raDeg: number, decDeg: number) => void;
  findings: readonly Finding[];
  pending: PendingMark | null;
  onMark: (m: PendingMark | null) => void;
  onSaveFinding: (kind: FindingKind, note: string) => Promise<Finding>;
  onResult: (r: FramesResult) => void;
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

  const { onResult } = props;
  useEffect(() => onResult(res), [res, onResult]);

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
  const current =
    res.frames[Math.min(Math.max(url.frame, 0), res.frames.length - 1)] ?? null;

  const compare = url.view === "compare" && !embed;

  return (
    <>
      {!embed ? (
        <ViewSwitch
          value={url.view}
          onChange={(v) => {
            props.onMark(null);
            setLocal((prev) => ({ ...prev, playing: false }));
            setUrl({ view: v });
          }}
        />
      ) : null}
      {res.status === "loading" ? (
        <p className="app-status" role="status">
          Updating…
        </p>
      ) : null}
      {compare ? (
        <CompareView
          frames={res.frames}
          width={width}
          height={height}
          stretch={url.stretch}
          cmap={url.cmap}
          bandId={url.band}
          findings={props.findings}
          pending={props.pending}
          onMark={props.onMark}
          onSave={props.onSaveFinding}
          onSpectrum={props.onPick}
        />
      ) : (
        <div
          className="app-stage"
          onClick={
            embed
              ? undefined
              : canvasClickToSpectrum(
                  { frame: current, width, height },
                  props.onPick,
                )
          }
        >
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
        </div>
      )}
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
  const { apiBase = DEFAULT_API_BASE } = props;
  const [url, setUrl] = useUrlState();
  const [detector, setDetector] = useState<number | null>(null);
  const [retryNonce, setRetryNonce] = useState(0);
  const [targets, setTargets] = useState<readonly MenuTarget[]>(PRESET_TARGETS);
  const [overlayOn, setOverlayOn] = useState(false);
  const [framesResult, setFramesResult] = useState<FramesResult>({
    status: "idle",
    response: null,
    frames: [],
    error: null,
  });

  // Mock mode: the prop wins, then ?mock=1. Memoized so useFrames sees a
  // stable payload and does not refetch on every render.
  const mockData = useMemo(
    () =>
      props.mockData ??
      (mockModeEnabled() ? makeMockFramesResponse() : undefined),
    [props.mockData],
  );

  const spectrumClient = useMemo<SpectrumClient>(
    () =>
      mockData !== undefined
        ? createMockSpectrumClient()
        : createSpectrumClient(apiBase),
    [apiBase, mockData],
  );
  const spec = useSpectrum(spectrumClient);

  // The API contract has no findings endpoint yet, so findings persist in
  // localStorage behind the FindingsClient interface.
  const findingsClient = useMemo<FindingsClient>(
    () => createLocalFindingsClient(),
    [],
  );
  const [findings, setFindings] = useState<readonly Finding[]>([]);
  const [pending, setPending] = useState<PendingMark | null>(null);

  useEffect(() => {
    let alive = true;
    findingsClient
      .list()
      .then((fs) => {
        if (alive) setFindings(fs);
      })
      .catch(() => {
        /* unreadable storage: start empty */
      });
    return () => {
      alive = false;
    };
  }, [findingsClient]);

  const saveFinding = useCallback(
    async (kind: FindingKind, note: string): Promise<Finding> => {
      if (pending === null) throw new Error("no pending mark");
      const f = await findingsClient.add({
        x: pending.x,
        y: pending.y,
        ra_deg: pending.raDeg,
        dec_deg: pending.decDeg,
        band: pending.band,
        epoch: pending.epoch,
        kind,
        note,
        frame_id: pending.frameId,
      });
      setFindings((prev) => [...prev, f]);
      setPending(null);
      return f;
    },
    [findingsClient, pending],
  );

  const removeFinding = useCallback(
    (id: string) => {
      findingsClient
        .remove(id)
        .then(() => setFindings((prev) => prev.filter((f) => f.id !== id)))
        .catch(() => {
          /* keep the list unchanged */
        });
    },
    [findingsClient],
  );

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

  const frames = framesResult.frames;
  const bandCounts = useMemo(() => bandFrameCounts(frames), [frames]);
  const currentFrame =
    frames[Math.min(Math.max(url.frame, 0), Math.max(frames.length - 1, 0))] ??
    null;

  // In Compare the metadata card follows the marked epoch, else epoch A.
  const compareFrame = useMemo(() => {
    if (url.view !== "compare" || url.embed) return null;
    const marked = frames.find((f) => f.meta.id === pending?.frameId);
    if (marked !== undefined) return marked;
    const band = bandById(url.band);
    return band !== null ? (selectEpochPair(frames, band)?.a ?? null) : null;
  }, [url.view, url.embed, url.band, frames, pending]);
  const shownFrame =
    url.view === "compare" && !url.embed ? compareFrame : currentFrame;

  // Overlay view position: the queried position when a response exists,
  // else the coordinates currently entered in the search bar.
  const q = framesResult.response?.query;
  const overlayPos = useMemo(() => {
    const raDeg = q?.ra_deg ?? parseRaToDeg(url.ra);
    const decDeg = q?.dec_deg ?? parseDecToDeg(url.dec);
    if (raDeg === null || raDeg === undefined) return null;
    if (decDeg === null || decDeg === undefined) return null;
    return { raDeg, decDeg, fovDeg: q?.fov_deg ?? url.fov };
  }, [q, url.ra, url.dec, url.fov]);

  const panel = (
    <div className="app-controls">
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-mode">View</h2>
          <GroupChevron />
        </summary>
        <ModeSelector
          mode={url.mode}
          detector={detector}
          onChange={(p) => {
            if (p.mode !== undefined) setUrl({ mode: p.mode, frame: 0 });
            if (p.detector !== undefined) setDetector(p.detector);
          }}
        />
      </details>
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-bands">Spectral band</h2>
          <GroupChevron />
        </summary>
        <BandSelector
          value={url.band}
          counts={bandCounts}
          onChange={(b) => {
            setPending(null);
            setUrl({ band: b, view: "compare" });
          }}
        />
      </details>
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-display">Display</h2>
          <GroupChevron />
        </summary>
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
      </details>
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-frame">Current frame</h2>
          <GroupChevron />
        </summary>
        <FrameMetadata
          frame={shownFrame?.meta ?? null}
          unit={framesResult.response?.unit}
          flagLegend={framesResult.response?.flag_legend}
        />
      </details>
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-findings">Findings ({findings.length})</h2>
          <GroupChevron />
        </summary>
        <FindingsList
          findings={findings}
          storage={findingsClient.kind}
          onRemove={removeFinding}
        />
      </details>
      <details className="app-group" open>
        <summary className="app-group-title">
          <h2 id="app-grp-overlay">Overlay</h2>
          <GroupChevron />
        </summary>
        <label className="app-check">
          <input
            type="checkbox"
            checked={overlayOn}
            onChange={(e) => setOverlayOn(e.currentTarget.checked)}
          />
          <span>Catalog overlay (Gaia DR3, AllWISE)</span>
        </label>
        {overlayOn ? (
          overlayPos !== null ? (
            <Suspense fallback={<p className="app-status">Loading overlay…</p>}>
              <AladinOverlay
                raDeg={overlayPos.raDeg}
                decDeg={overlayPos.decDeg}
                fovDeg={overlayPos.fovDeg}
                defaultOpen
              />
            </Suspense>
          ) : (
            <p className="app-status">
              Search for a position to enable the sky overlay.
            </p>
          )
        ) : null}
      </details>
    </div>
  );

  return (
    <AppShell
      embed={url.embed}
      targets={targets}
      onSelectTarget={submitCoords}
      headerActions={<AboutDialog />}
      notice={<FirstRunHint />}
      credit={<EmbedCredit />}
      search={
        <CoordinateSearchBar
          value={{ ra: url.ra, dec: url.dec, fov: url.fov }}
          loading={framesResult.status === "loading"}
          onSubmit={submitCoords}
        />
      }
      panel={panel}
      drawer={
        spec.open ? (
          <SpectrumPanel
            open={spec.open}
            status={spec.status}
            spectrum={spec.data}
            error={spec.error}
            raDeg={spec.raDeg}
            decDeg={spec.decDeg}
            aperturePx={spec.aperturePx}
            onApertureChange={spec.setAperture}
            onClose={spec.close}
          />
        ) : null
      }
    >
      <Explorer
        key={retryNonce}
        apiBase={apiBase}
        url={url}
        setUrl={setUrl}
        detector={detector}
        embed={url.embed}
        mockData={mockData}
        onPick={spec.requestAt}
        findings={findings}
        pending={pending}
        onMark={setPending}
        onSaveFinding={saveFinding}
        onResult={setFramesResult}
        onRetry={() => setRetryNonce((n) => n + 1)}
      />
    </AppShell>
  );
}

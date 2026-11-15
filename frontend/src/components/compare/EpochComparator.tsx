// Two-epoch comparator: epoch B is drawn over epoch A and either clipped by
// a draggable vertical divider (slider mode) or toggled on a timer (blink
// comparator). A click marks a pixel (X, Y) with a radar pulse and resolves
// its sky position from the frame WCS. All animation is CSS (compare.css)
// and is reduced to a static state under prefers-reduced-motion.
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { useFrameCanvas } from "./useFrameCanvas";
import { pixelToSky } from "../spectrum/pick";
import { mjdToIso } from "../../lib/coords";
import { formatDeltaT, type BandDef, type EpochPair } from "../../lib/bands";
import type { StretchKind } from "../../lib/stretch";
import type { ColormapName } from "../../lib/colormaps";
import type { Finding } from "../../lib/findings";
import "./compare.css";

export type CompareMode = "slider" | "blink";
export type Epoch = "A" | "B";

export interface PendingMark {
  x: number;
  y: number;
  raDeg: number | null;
  decDeg: number | null;
  epoch: Epoch;
  band: string;
  frameId: string;
}

export interface EpochComparatorProps {
  pair: EpochPair;
  band: BandDef;
  width: number;
  height: number;
  stretch: StretchKind;
  cmap: ColormapName;
  mode: CompareMode;
  intervalMs: number;
  playing: boolean;
  findings: readonly Finding[];
  pending: PendingMark | null;
  onMark: (m: PendingMark) => void;
}

interface Pulse {
  id: number;
  xPct: number;
  yPct: number;
}

const PULSE_MS = 1600;
const GRID_TICKS = [0.25, 0.5, 0.75] as const;

const clamp = (v: number, lo: number, hi: number) =>
  Math.min(hi, Math.max(lo, v));

export function EpochComparator(props: EpochComparatorProps) {
  const { pair, band, width, height, mode, intervalMs, playing } = props;
  const frameRef = useRef<HTMLDivElement | null>(null);
  const canvasA = useRef<HTMLCanvasElement | null>(null);
  const canvasB = useRef<HTMLCanvasElement | null>(null);
  const hudRef = useRef<HTMLSpanElement | null>(null);
  const dragging = useRef(false);
  const pulseSeq = useRef(0);
  const timers = useRef<number[]>([]);
  const [pos, setPos] = useState(50);
  const [visible, setVisible] = useState<Epoch>("A");
  const [pulses, setPulses] = useState<Pulse[]>([]);

  useFrameCanvas(canvasA, pair.a, width, height, props.stretch, props.cmap);
  useFrameCanvas(canvasB, pair.b, width, height, props.stretch, props.cmap);

  const hasB = pair.b !== null;

  useEffect(() => {
    if (mode !== "blink" || !playing || !hasB) return;
    const id = window.setInterval(
      () => setVisible((e) => (e === "A" ? "B" : "A")),
      intervalMs,
    );
    return () => window.clearInterval(id);
  }, [mode, playing, intervalMs, hasB]);

  useEffect(() => {
    if (mode === "slider") setVisible("A");
  }, [mode]);

  useEffect(
    () => () => {
      for (const t of timers.current) window.clearTimeout(t);
    },
    [],
  );

  const pctFromEvent = (clientX: number, clientY: number) => {
    const el = frameRef.current;
    if (el === null) return null;
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return null;
    return {
      xPct: ((clientX - r.left) / r.width) * 100,
      yPct: ((clientY - r.top) / r.height) * 100,
    };
  };

  const onFramePointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const p = pctFromEvent(e.clientX, e.clientY);
    if (p === null) return;
    if (dragging.current) {
      setPos(clamp(p.xPct, 0, 100));
      return;
    }
    if (hudRef.current !== null) {
      const x = Math.floor(clamp(p.xPct, 0, 99.999) * (width / 100));
      const y = Math.floor(clamp(p.yPct, 0, 99.999) * (height / 100));
      hudRef.current.textContent = `X ${String(x).padStart(3, "0")}  Y ${String(y).padStart(3, "0")}`;
    }
  };

  const onFrameClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const p = pctFromEvent(e.clientX, e.clientY);
    if (p === null) return;
    if (p.xPct < 0 || p.xPct > 100 || p.yPct < 0 || p.yPct > 100) return;
    const px = clamp((p.xPct / 100) * width, 0, width - 1e-6);
    const py = clamp((p.yPct / 100) * height, 0, height - 1e-6);
    const epoch: Epoch = !hasB
      ? "A"
      : mode === "blink"
        ? visible
        : p.xPct < pos
          ? "A"
          : "B";
    const frame = epoch === "B" && pair.b !== null ? pair.b : pair.a;
    const sky = pixelToSky(frame.meta.wcs, px, py);
    const id = ++pulseSeq.current;
    setPulses((list) => [...list, { id, xPct: p.xPct, yPct: p.yPct }]);
    timers.current.push(
      window.setTimeout(
        () => setPulses((list) => list.filter((q) => q.id !== id)),
        PULSE_MS,
      ),
    );
    props.onMark({
      x: Math.floor(px),
      y: Math.floor(py),
      raDeg: sky?.ra_deg ?? null,
      decDeg: sky?.dec_deg ?? null,
      epoch,
      band: band.id,
      frameId: frame.meta.id,
    });
  };

  const onDividerDown = (e: PointerEvent<HTMLDivElement>) => {
    e.stopPropagation();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    dragging.current = true;
  };
  const onDividerUp = (e: PointerEvent<HTMLDivElement>) => {
    dragging.current = false;
    e.currentTarget.releasePointerCapture?.(e.pointerId);
  };
  const onDividerKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const step = e.shiftKey ? 10 : 2;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      setPos((v) => clamp(v - step, 0, 100));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      setPos((v) => clamp(v + step, 0, 100));
    } else if (e.key === "Home") {
      e.preventDefault();
      setPos(0);
    } else if (e.key === "End") {
      e.preventDefault();
      setPos(100);
    }
  };

  // Grid tick labels from epoch A's WCS (pixel centre of each grid line).
  const ticks = useMemo(() => {
    const wcs = pair.a.meta.wcs;
    return GRID_TICKS.map((t) => ({
      t,
      ra: pixelToSky(wcs, t * width, height / 2)?.ra_deg ?? null,
      dec: pixelToSky(wcs, width / 2, t * height)?.dec_deg ?? null,
    }));
  }, [pair.a, width, height]);

  const showB = mode === "slider" || visible === "B";
  const frameStyle = {
    "--pos": pos.toFixed(2),
    "--ar": `${width} / ${height}`,
    "--ar-w": width,
    "--ar-h": height,
    "--band": band.color,
  } as CSSProperties;
  const dt = pair.dtDays !== null ? formatDeltaT(pair.dtDays) : null;
  const dateA = mjdToIso(pair.a.meta.obs_mjd_start);
  const dateB = pair.b !== null ? mjdToIso(pair.b.meta.obs_mjd_start) : null;

  return (
    <div className="cmp-stage">
      <div
        ref={frameRef}
        className={mode === "blink" ? "cmp-frame cmp-blink" : "cmp-frame"}
        style={frameStyle}
        onClick={onFrameClick}
        onPointerMove={onFramePointerMove}
        onPointerUp={() => {
          dragging.current = false;
        }}
      >
        <canvas
          ref={canvasA}
          className="cmp-canvas"
          role="img"
          aria-label={`Epoch A, ${band.label} ${band.centerUm.toFixed(1)} micrometers, observed ${dateA}`}
        />
        {/* Always mounted so its renderer survives band switches that
            temporarily have no epoch B. */}
        <canvas
          ref={canvasB}
          className={
            hasB && showB ? "cmp-canvas cmp-b" : "cmp-canvas cmp-b cmp-hidden"
          }
          role={hasB ? "img" : undefined}
          aria-hidden={hasB ? undefined : true}
          aria-label={
            hasB
              ? `Epoch B, ${band.label}, observed ${dateB ?? "—"}`
              : undefined
          }
        />

        <div className="cmp-grid" aria-hidden="true">
          {ticks.map((k) => (
            <span
              key={`ra-${k.t}`}
              className="cmp-tick cmp-tick-x num"
              style={{ left: `${k.t * 100}%` }}
            >
              {k.ra !== null ? `${k.ra.toFixed(3)}°` : ""}
            </span>
          ))}
          {ticks.map((k) => (
            <span
              key={`dec-${k.t}`}
              className="cmp-tick cmp-tick-y num"
              style={{ top: `${k.t * 100}%` }}
            >
              {k.dec !== null ? `${k.dec.toFixed(3)}°` : ""}
            </span>
          ))}
        </div>

        {props.findings
          .filter(
            (f) =>
              f.frame_id === pair.a.meta.id || f.frame_id === pair.b?.meta.id,
          )
          .map((f) => (
            <span
              key={f.id}
              className={
                f.band === band.id ? "cmp-marker" : "cmp-marker cmp-marker-dim"
              }
              style={{
                left: `${((f.x + 0.5) / width) * 100}%`,
                top: `${((f.y + 0.5) / height) * 100}%`,
              }}
              title={`(${f.x}, ${f.y}) · ${f.kind} · epoch ${f.epoch}`}
              aria-hidden="true"
            />
          ))}

        {props.pending !== null ? (
          <span
            key={`${props.pending.x}-${props.pending.y}`}
            className="cmp-reticle"
            style={{
              left: `${((props.pending.x + 0.5) / width) * 100}%`,
              top: `${((props.pending.y + 0.5) / height) * 100}%`,
            }}
            aria-hidden="true"
          />
        ) : null}

        {pulses.map((p) => (
          <span
            key={p.id}
            className="cmp-pulse"
            style={{ left: `${p.xPct}%`, top: `${p.yPct}%` }}
            aria-hidden="true"
          >
            <i />
            <i />
            <i />
          </span>
        ))}

        {mode === "slider" && hasB ? (
          <div
            className="cmp-divider"
            role="slider"
            tabIndex={0}
            aria-label="Epoch A / epoch B divider"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(pos)}
            aria-valuetext={`${Math.round(pos)}% epoch A`}
            onPointerDown={onDividerDown}
            onPointerUp={onDividerUp}
            onPointerCancel={onDividerUp}
            onClick={(e) => e.stopPropagation()}
            onKeyDown={onDividerKey}
          >
            <span className="cmp-divider-line" />
            <span className="cmp-divider-knob">
              <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path
                  d="M9 6l-6 6 6 6M15 6l6 6-6 6"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </span>
          </div>
        ) : null}

        <div className="cmp-labels" aria-hidden="true">
          {mode === "slider" || !hasB ? (
            <>
              <span className="tl-chip num">Epoch A · {dateA}</span>
              {hasB ? (
                <span className="tl-chip num">Epoch B · +{dt ?? "—"}</span>
              ) : null}
            </>
          ) : (
            <span className="tl-chip num cmp-epoch-chip" key={visible}>
              <span
                className={visible === "A" ? "cmp-dot" : "cmp-dot cmp-dot-b"}
              />
              Epoch {visible}
              {visible === "B" ? ` · +${dt ?? "—"}` : ` · ${dateA}`}
            </span>
          )}
        </div>

        <div className="cmp-hud" aria-hidden="true">
          <span ref={hudRef} className="tl-chip num">
            X 000 Y 000
          </span>
          <span className="tl-chip num cmp-lambda">
            λ {band.centerUm.toFixed(1)} µm
          </span>
        </div>
      </div>
      <span className="tl-sr" role="status" aria-live="polite">
        {mode === "blink" && hasB ? `Showing epoch ${visible}` : ""}
      </span>
    </div>
  );
}

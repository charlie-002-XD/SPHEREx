// Spectrum panel: flux vs wavelength with 1-sigma error bars and optional
// shaded molecular absorption bands. Renders as a bottom sheet on small
// screens (<=640px) and as a side panel on desktop. uPlot is loaded lazily
// from a pinned CDN build (see uplot.ts); if it fails, the panel degrades to
// the textual summary (point count, detectors, flags) so data is never lost.
import { useEffect, useMemo, useRef, useState } from "react";
import { formatDecDeg, formatRaDeg } from "../../lib/coords";
import { APERTURE_PX, type AperturePx, type SpectrumResponse } from "./types";
import {
  MOLECULAR_BANDS,
  bandRects,
  detectorsPresent,
  flagCounts,
  prepareSpectrum,
} from "./prepare";
import {
  UPLOT_VERSION,
  loadUPlot,
  type UPlotConstructor,
  type UPlotData,
  type UPlotInstance,
  type UPlotOptions,
} from "./uplot";
import type { SpectrumStatus } from "./useSpectrum";
import { FlagBadges } from "../metadata/FlagBadges";
import "./spectrum.css";

export interface SpectrumPanelProps {
  open: boolean;
  status: SpectrumStatus;
  spectrum: SpectrumResponse | null;
  error: string | null;
  raDeg: number | null;
  decDeg: number | null;
  aperturePx: AperturePx;
  onApertureChange: (px: AperturePx) => void;
  onClose: () => void;
}

function cssVar(el: HTMLElement, name: string, fallback: string): string {
  const v = getComputedStyle(el).getPropertyValue(name).trim();
  return v === "" ? fallback : v;
}

function buildChart(
  UPlot: UPlotConstructor,
  el: HTMLElement,
  spectrum: SpectrumResponse,
  showBands: boolean,
): UPlotInstance {
  const prepared = prepareSpectrum(spectrum.points);
  const fg = cssVar(el, "--sx-fg-dim", "#a0a0b8");
  const grid = cssVar(el, "--sx-border-soft", "#2a2a3a");
  const accent = cssVar(el, "--sx-accent", "#6ea8fe");
  const bandFill = cssVar(el, "--sx-band-fill", "rgba(110, 168, 254, 0.10)");
  const bandStroke = cssVar(el, "--sx-band-stroke", "rgba(110, 168, 254, 0.35)");

  const xs = prepared.wavelengthUm;
  const ys = prepared.flux;
  const sigmas = prepared.sigma;
  const data: UPlotData = [xs, ys];

  const drawBands = (u: UPlotInstance) => {
    if (!showBands) return;
    const sx = u.scales.x;
    if (sx?.min === undefined || sx.max === undefined) return;
    const { ctx, bbox } = u;
    const rects = bandRects(
      MOLECULAR_BANDS,
      (um) => u.valToPos(um, "x", true),
      sx.min,
      sx.max,
    );
    if (rects.length === 0) return;
    ctx.save();
    ctx.fillStyle = bandFill;
    ctx.strokeStyle = bandStroke;
    ctx.lineWidth = 1;
    ctx.font = "10px system-ui, sans-serif";
    for (const r of rects) {
      ctx.fillRect(r.x0, bbox.top, r.x1 - r.x0, bbox.height);
      ctx.strokeRect(r.x0 + 0.5, bbox.top, r.x1 - r.x0 - 1, bbox.height);
      ctx.save();
      ctx.fillStyle = fg;
      ctx.fillText(r.label, r.x0 + 4, bbox.top + 12);
      ctx.restore();
    }
    ctx.restore();
  };

  const drawErrBars = (u: UPlotInstance) => {
    const { ctx } = u;
    ctx.save();
    ctx.strokeStyle = accent;
    ctx.lineWidth = 1;
    ctx.globalAlpha = 0.8;
    ctx.beginPath();
    for (let i = 0; i < xs.length; i++) {
      const v = ys[i];
      const s = sigmas[i];
      const x = xs[i];
      if (v === null || v === undefined || s === null || s === undefined || x === undefined) {
        continue;
      }
      const xp = u.valToPos(x, "x", true);
      const yTop = u.valToPos(v + s, "y", true);
      const yBot = u.valToPos(v - s, "y", true);
      ctx.moveTo(xp - 3, yTop);
      ctx.lineTo(xp + 3, yTop);
      ctx.moveTo(xp, yTop);
      ctx.lineTo(xp, yBot);
      ctx.moveTo(xp - 3, yBot);
      ctx.lineTo(xp + 3, yBot);
    }
    ctx.stroke();
    ctx.restore();
  };

  const font = "11px system-ui, sans-serif";
  const opts: UPlotOptions = {
    width: Math.max(el.clientWidth, 120),
    height: Math.max(el.clientHeight, 140),
    scales: { x: { time: false } },
    axes: [
      { stroke: fg, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, label: "λ (µm)", font, labelFont: font },
      { stroke: fg, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, label: spectrum.unit, font, labelFont: font },
    ],
    series: [
      {},
      { label: spectrum.unit, stroke: accent, width: 1.5, points: { show: true, size: 4, fill: accent } },
    ],
    hooks: { drawClear: [drawBands], draw: [drawErrBars] },
    legend: { show: false },
    cursor: { show: true, points: { size: 8 } },
    padding: [16, 8, 0, 0],
  };
  return new UPlot(opts, data, el);
}

export function SpectrumPanel(props: SpectrumPanelProps) {
  const { open, status, spectrum, error, raDeg, decDeg } = props;
  const [showBands, setShowBands] = useState(true);
  const plotRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<UPlotInstance | null>(null);
  const [plotError, setPlotError] = useState<string | null>(null);

  const prepared = useMemo(
    () => (spectrum === null ? null : prepareSpectrum(spectrum.points)),
    [spectrum],
  );
  const dets = useMemo(
    () => (spectrum === null ? [] : detectorsPresent(spectrum.points)),
    [spectrum],
  );
  const flags = useMemo(
    () => (spectrum === null ? [] : flagCounts(spectrum.points).map((f) => f.name)),
    [spectrum],
  );

  // Build/rebuild the chart when data or band visibility changes.
  useEffect(() => {
    const el = plotRef.current;
    if (el === null || !open || spectrum === null || prepared === null) return;
    if (prepared.validCount === 0) return;
    let disposed = false;
    let ro: ResizeObserver | null = null;
    loadUPlot()
      .then((UPlot) => {
        if (disposed || plotRef.current === null) return;
        chartRef.current?.destroy();
        const u = buildChart(UPlot, plotRef.current, spectrum, showBands);
        chartRef.current = u;
        setPlotError(null);
        ro = new ResizeObserver(() => {
          const host = plotRef.current;
          if (host !== null) {
            u.setSize({
              width: Math.max(host.clientWidth, 120),
              height: Math.max(host.clientHeight, 140),
            });
          }
        });
        ro.observe(plotRef.current);
      })
      .catch((e: unknown) => {
        if (!disposed) {
          setPlotError(e instanceof Error ? e.message : "chart failed to load");
        }
      });
    return () => {
      disposed = true;
      ro?.disconnect();
      chartRef.current?.destroy();
      chartRef.current = null;
    };
  }, [open, spectrum, prepared, showBands]);

  // Escape closes the sheet.
  const { onClose } = props;
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const posLabel =
    raDeg !== null && decDeg !== null
      ? `${formatRaDeg(raDeg)} ${formatDecDeg(decDeg)}`
      : "";

  return (
    <aside className="sp-panel" aria-label="Spectrum panel">
      <header className="sp-head">
        <div className="sp-titles">
          <h2 className="sp-title">Spectrum</h2>
          {posLabel !== "" ? <span className="sp-pos">{posLabel}</span> : null}
        </div>
        <button
          type="button"
          className="sp-close"
          aria-label="Close spectrum panel"
          onClick={props.onClose}
        >
          ×
        </button>
      </header>

      <div className="sp-controls">
        <label className="sp-toggle">
          <input
            type="checkbox"
            checked={showBands}
            onChange={(e) => setShowBands(e.currentTarget.checked)}
          />
          <span>Molecular bands</span>
        </label>
        <label className="sp-aperture">
          <span>Aperture</span>
          <select
            aria-label="Aperture radius in pixels"
            value={props.aperturePx}
            onChange={(e) => {
              const v = Number(e.currentTarget.value);
              if ((APERTURE_PX as readonly number[]).includes(v)) {
                props.onApertureChange(v as AperturePx);
              }
            }}
          >
            {APERTURE_PX.map((a) => (
              <option key={a} value={a}>
                {a} px
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="sp-body">
        {status === "loading" ? (
          <p className="sp-status" role="status">
            Loading spectrum…
          </p>
        ) : null}
        {status === "error" ? (
          <p className="sp-error" role="alert">
            {error ?? "Spectrum request failed"}
          </p>
        ) : null}
        {spectrum !== null && status !== "error" ? (
          prepared !== null && prepared.validCount > 0 ? (
            <>
              <div
                ref={plotRef}
                className="sp-plot"
                role="img"
                aria-label={`Spectrum chart, ${prepared.validCount} points`}
              />
              {plotError !== null ? (
                <p className="sp-status" role="status">
                  Chart unavailable ({plotError}); data summary below.
                </p>
              ) : null}
              <p className="sp-meta">
                {prepared.validCount} points
                {dets.length > 0 ? ` · D${dets.join(" D")}` : ""} · 1σ error bars
              </p>
              {flags.length > 0 ? (
                <div className="sp-flags">
                  <span className="sp-flags-label">Flags</span>
                  <FlagBadges flags={flags} />
                </div>
              ) : null}
            </>
          ) : (
            <p className="sp-status" role="status">
              No usable spectral points at this position.
            </p>
          )
        ) : null}
        {status === "idle" ? (
          <p className="sp-status">Click the image to sample a spectrum.</p>
        ) : null}
      </div>

      <footer className="sp-foot">
        <span>Bands: Boogert+2015 · Chart: uPlot {UPLOT_VERSION} (MIT)</span>
      </footer>
    </aside>
  );
}

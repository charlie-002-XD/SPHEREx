// Lazy loader + minimal typings for uPlot, loaded once from a pinned CDN URL.
// package.json is outside this task's scope, so uPlot is not an npm dep:
// https://cdn.jsdelivr.net/npm/uplot@1.6.32 (MIT, Leon Sorokin). Subresource
// integrity hashes pin the exact bytes. To vendor it later: `npm i uplot`
// and replace loadUPlot() with a static import.

export const UPLOT_VERSION = "1.6.32";
const UPLOT_BASE = `https://cdn.jsdelivr.net/npm/uplot@${UPLOT_VERSION}/dist`;
const UPLOT_JS_URL = `${UPLOT_BASE}/uPlot.iife.min.js`;
const UPLOT_CSS_URL = `${UPLOT_BASE}/uPlot.min.css`;
const UPLOT_JS_SRI = "sha384-Gx3t0zdBAuQOuvvmaLZj7HKEiSgWTAs+VdtNY7wt19QDPTDQjFIwAuXDj0zeN00c";
const UPLOT_CSS_SRI = "sha384-IfV0B7MIOYuO95kO9G5ySKPz/85zqFNOAs8iy4tkK5zd9izhJAB8b7lHrwYqqmYE";

export type UPlotData = Array<Array<number | null>>;
export type UPlotHook = (u: UPlotInstance) => void;

export interface UPlotHooks {
  drawClear?: UPlotHook[];
  draw?: UPlotHook[];
}

export interface UPlotInstance {
  setData(data: UPlotData, resetScales?: boolean): void;
  setSize(size: { width: number; height: number }): void;
  destroy(): void;
  valToPos(val: number, scaleKey: string, canvasPx?: boolean): number;
  bbox: { left: number; top: number; width: number; height: number };
  ctx: CanvasRenderingContext2D;
  scales: Record<string, { min?: number; max?: number }>;
}

export interface UPlotOptions {
  width: number;
  height: number;
  scales?: Record<string, { time?: boolean; range?: unknown }>;
  axes?: Array<{
    stroke?: string;
    label?: string;
    labelFont?: string;
    font?: string;
    grid?: { stroke?: string; width?: number };
    ticks?: { stroke?: string; width?: number };
  }>;
  series?: Array<{
    label?: string;
    stroke?: string;
    width?: number;
    points?: { show?: boolean; size?: number; stroke?: string; fill?: string };
  }>;
  hooks?: UPlotHooks;
  legend?: { show?: boolean };
  cursor?: { show?: boolean; points?: { size?: number } };
  padding?: [number, number, number, number] | null;
}

export type UPlotConstructor = new (
  opts: UPlotOptions,
  data: UPlotData,
  el: HTMLElement,
) => UPlotInstance;

function injectCss(): void {
  if (document.querySelector(`link[href="${UPLOT_CSS_URL}"]`) !== null) return;
  const link = document.createElement("link");
  link.rel = "stylesheet";
  link.href = UPLOT_CSS_URL;
  link.integrity = UPLOT_CSS_SRI;
  link.crossOrigin = "anonymous";
  document.head.appendChild(link);
}

let pending: Promise<UPlotConstructor> | null = null;

export function loadUPlot(): Promise<UPlotConstructor> {
  const globalCtor = (window as unknown as { uPlot?: UPlotConstructor }).uPlot;
  if (globalCtor !== undefined) return Promise.resolve(globalCtor);
  if (pending !== null) return pending;
  pending = new Promise<UPlotConstructor>((resolve, reject) => {
    injectCss();
    const script = document.createElement("script");
    script.src = UPLOT_JS_URL;
    script.integrity = UPLOT_JS_SRI;
    script.crossOrigin = "anonymous";
    script.async = true;
    script.onload = () => {
      const ctor = (window as unknown as { uPlot?: UPlotConstructor }).uPlot;
      if (ctor !== undefined) resolve(ctor);
      else reject(new Error("uPlot loaded but window.uPlot is missing"));
    };
    script.onerror = () => reject(new Error("failed to load uPlot"));
    document.head.appendChild(script);
  });
  pending.catch(() => {
    pending = null; // allow retry after a failed load
  });
  return pending;
}

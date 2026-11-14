// Lazy loader + minimal typings for uPlot, bundled as an npm dependency
// (uplot@1.6.32, MIT, Leon Sorokin). The library and its stylesheet are
// loaded on first use via dynamic import() so they stay out of the main
// bundle; no CDN or SRI is involved at runtime.

export const UPLOT_VERSION = "1.6.32";

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
    /** Space reserved for the axis label, px (uPlot axis option). */
    labelSize?: number;
    /** Space reserved for ticks + values, px (uPlot axis option). */
    size?: number;
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

let pending: Promise<UPlotConstructor> | null = null;

// Resolves to the uPlot constructor. The package's own typings are much
// wider than the minimal surface the panel uses, so the module is cast to
// the local UPlotConstructor — the call sites keep the same typed API.
export function loadUPlot(): Promise<UPlotConstructor> {
  if (pending !== null) return pending;
  pending = Promise.all([
    import("uplot"),
    import("uplot/dist/uPlot.min.css"),
  ]).then(
    ([mod]) => mod.default as unknown as UPlotConstructor,
    (err: unknown) => {
      pending = null; // allow retry after a failed load
      throw err instanceof Error ? err : new Error("failed to load uPlot");
    },
  );
  return pending;
}

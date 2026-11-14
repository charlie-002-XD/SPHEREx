// Spectrum fetch state + the click-to-spectrum glue. `useSpectrum` owns the
// request lifecycle (abort on change, race guard); `canvasClickToSpectrum`
// turns a click on the time-lapse canvas into a request at that RA/Dec.
import { useCallback, useEffect, useRef, useState } from "react";
import type { DecodedFrame } from "../../lib/types";
import { canvasPickToSky } from "./pick";
import type { SpectrumClient } from "./spectrumApi";
import type { AperturePx, SpectrumResponse } from "./types";

export type SpectrumStatus = "idle" | "loading" | "success" | "error";

export interface SpectrumState {
  status: SpectrumStatus;
  open: boolean;
  data: SpectrumResponse | null;
  error: string | null;
  raDeg: number | null;
  decDeg: number | null;
  aperturePx: AperturePx;
}

export interface SpectrumController extends SpectrumState {
  requestAt(raDeg: number, decDeg: number): void;
  setAperture(px: AperturePx): void;
  close(): void;
}

export function useSpectrum(client: SpectrumClient | null): SpectrumController {
  const [state, setState] = useState<SpectrumState>({
    status: "idle",
    open: false,
    data: null,
    error: null,
    raDeg: null,
    decDeg: null,
    aperturePx: 3,
  });
  const seq = useRef(0);
  const abort = useRef<AbortController | null>(null);
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(
    () => () => {
      abort.current?.abort();
    },
    [],
  );

  const requestAt = useCallback(
    (raDeg: number, decDeg: number, aperturePx?: AperturePx) => {
      const ap = aperturePx ?? stateRef.current.aperturePx;
      const id = ++seq.current;
      abort.current?.abort();
      if (client === null) {
        setState((s) => ({
          ...s,
          open: true,
          aperturePx: ap,
          raDeg,
          decDeg,
          data: null,
          status: "error",
          error: "Spectrum service unavailable",
        }));
        return;
      }
      setState((s) => ({
        ...s,
        open: true,
        aperturePx: ap,
        raDeg,
        decDeg,
        error: null,
        status: "loading",
      }));
      const ctrl = new AbortController();
      abort.current = ctrl;
      client
        .spectrum(raDeg, decDeg, ap, ctrl.signal)
        .then((data) => {
          if (seq.current !== id) return;
          setState((s) => ({ ...s, status: "success", data, error: null }));
        })
        .catch((err: unknown) => {
          if (seq.current !== id || ctrl.signal.aborted) return;
          setState((s) => ({
            ...s,
            status: "error",
            data: null,
            error: err instanceof Error ? err.message : "Spectrum request failed",
          }));
        });
    },
    [client],
  );

  const setAperture = useCallback(
    (px: AperturePx) => {
      const s = stateRef.current;
      if (s.raDeg !== null && s.decDeg !== null && s.status !== "idle") {
        requestAt(s.raDeg, s.decDeg, px); // refetch at the same position
      } else {
        setState((prev) => ({ ...prev, aperturePx: px }));
      }
    },
    [requestAt],
  );

  const close = useCallback(() => {
    seq.current += 1; // drop in-flight results
    abort.current?.abort();
    setState((s) => ({ ...s, open: false }));
  }, []);

  return { ...state, requestAt, setAperture, close };
}

export interface CanvasPickContext {
  frame: DecodedFrame | null;
  width: number;
  height: number;
}

// Returns a click handler for the element that wraps the time-lapse canvas.
// Only clicks whose target is the cutout <canvas> are converted (overlay divs
// and controls are ignored). The resulting RA/Dec is passed to `onPick`.
export function canvasClickToSpectrum(
  ctx: CanvasPickContext,
  onPick: (raDeg: number, decDeg: number) => void,
): (e: React.MouseEvent<HTMLElement>) => void {
  return (e) => {
    const target = e.target;
    if (!(target instanceof HTMLCanvasElement) || ctx.frame === null) return;
    const sky = canvasPickToSky(
      target,
      e.clientX,
      e.clientY,
      ctx.frame.meta.wcs,
      ctx.width,
      ctx.height,
    );
    if (sky !== null) onPick(sky.ra_deg, sky.dec_deg);
  };
}

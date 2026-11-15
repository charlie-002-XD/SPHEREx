// Draws one decoded frame onto a canvas with the shared WebGL2/2D renderer,
// using the same per-frame stretch interval as the time-lapse viewer.
import { useEffect, useRef, type RefObject } from "react";
import { createFrameRenderer, type FrameRenderer } from "../../lib/gl";
import {
  computeInterval,
  symmetricInterval,
  type StretchKind,
} from "../../lib/stretch";
import { isDiverging, type ColormapName } from "../../lib/colormaps";
import type { DecodedFrame } from "../../lib/types";

export function useFrameCanvas(
  canvasRef: RefObject<HTMLCanvasElement | null>,
  frame: DecodedFrame | null,
  width: number,
  height: number,
  stretch: StretchKind,
  cmap: ColormapName,
): void {
  const rendererRef = useRef<FrameRenderer | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const r = createFrameRenderer(canvas);
    rendererRef.current = r;
    return () => {
      r.dispose();
      rendererRef.current = null;
    };
  }, [canvasRef]);

  useEffect(() => {
    const canvas = canvasRef.current;
    const r = rendererRef.current;
    if (canvas === null || r === null || frame === null) return;
    canvas.width = width;
    canvas.height = height;
    r.upload(frame.pixels, width, height);
    const iv = isDiverging(cmap)
      ? symmetricInterval(frame.pixels)
      : computeInterval(frame.pixels, stretch);
    r.draw({ lo: iv.lo, hi: iv.hi, stretch, cmap });
  }, [canvasRef, frame, width, height, stretch, cmap]);
}

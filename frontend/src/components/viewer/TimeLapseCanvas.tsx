// Time-lapse viewer: decodes float32 frames onto a WebGL2 R32F texture
// (2D canvas fallback), requestAnimationFrame playback, scrubber, blink
// comparator and a per-frame metadata overlay.
import { useCallback, useEffect, useRef, useState } from "react";
import type { DecodedFrame } from "../../lib/types";
import {
  computeInterval,
  symmetricInterval,
  type Interval,
  type StretchKind,
} from "../../lib/stretch";
import {
  isDiverging as isDivergingCmap,
  type ColormapName,
} from "../../lib/colormaps";
import { createFrameRenderer, type FrameRenderer } from "../../lib/gl";
import { mjdToIso } from "../../lib/coords";
import { FPS_MAX, FPS_MIN } from "../../lib/urlState";
import "./viewer.css";

export interface TransportState {
  playing: boolean;
  frameIndex: number;
  fps: number;
  loop: boolean;
  blink: boolean;
}

export interface TimeLapseCanvasProps {
  frames: DecodedFrame[];
  width: number;
  height: number;
  unit?: string;
  stretch: StretchKind;
  cmap: ColormapName;
  transport: TransportState;
  onTransport: (patch: Partial<TransportState>) => void;
  embed?: boolean;
}

export function TimeLapseCanvas(props: TimeLapseCanvasProps) {
  const { frames, transport, onTransport, embed = false } = props;
  const n = frames.length;
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<FrameRenderer | null>(null);
  const lastPixelsRef = useRef<Float32Array | null>(null);
  const [blinkA, setBlinkA] = useState(0);
  const [blinkPhase, setBlinkPhase] = useState<0 | 1>(0);

  const clampIndex = useCallback(
    (i: number) => Math.min(Math.max(i, 0), Math.max(n - 1, 0)),
    [n],
  );
  const frameIndex = clampIndex(transport.frameIndex);
  const anchor = clampIndex(blinkA);
  const displayIndex =
    transport.blink && blinkPhase === 0 ? anchor : frameIndex;
  const current = frames[displayIndex] ?? null;

  const intervalCache = useRef(new Map<string, Interval>());

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null) return;
    const r = createFrameRenderer(canvas);
    rendererRef.current = r;
    return () => {
      r.dispose();
      rendererRef.current = null;
    };
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas !== null) {
      canvas.width = props.width;
      canvas.height = props.height;
    }
  }, [props.width, props.height]);

  const intervalFor = useCallback(
    (f: DecodedFrame): Interval => {
      const key = `${f.meta.id}:${props.stretch}:${props.cmap}`;
      const hit = intervalCache.current.get(key);
      if (hit !== undefined) return hit;
      const iv = isDivergingCmap(props.cmap)
        ? symmetricInterval(f.pixels)
        : computeInterval(f.pixels, props.stretch);
      intervalCache.current.set(key, iv);
      return iv;
    },
    [props.stretch, props.cmap],
  );

  useEffect(() => {
    const r = rendererRef.current;
    if (r === null || current === null) return;
    if (lastPixelsRef.current !== current.pixels) {
      r.upload(current.pixels, props.width, props.height);
      lastPixelsRef.current = current.pixels;
    }
    const iv = intervalFor(current);
    r.draw({ lo: iv.lo, hi: iv.hi, stretch: props.stretch, cmap: props.cmap });
  }, [current, intervalFor, props.stretch, props.cmap, props.width, props.height]);

  // requestAnimationFrame playback clock
  const blinkRef = useRef(transport.blink);
  blinkRef.current = transport.blink;
  useEffect(() => {
    if (!transport.playing || n === 0) return;
    let raf = 0;
    let last = performance.now();
    let acc = 0;
    const tick = (now: number) => {
      acc += now - last;
      last = now;
      if (acc >= 1000 / transport.fps) {
        acc = 0;
        if (blinkRef.current) {
          setBlinkPhase((p) => (p === 0 ? 1 : 0));
        } else if (frameIndex >= n - 1) {
          if (transport.loop) {
            onTransport({ frameIndex: 0 });
          } else {
            onTransport({ playing: false });
          }
        } else {
          onTransport({ frameIndex: frameIndex + 1 });
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [transport.playing, transport.fps, transport.loop, frameIndex, n, onTransport]);

  useEffect(() => {
    if (!transport.playing) setBlinkPhase(0);
  }, [transport.playing]);

  const step = useCallback(
    (delta: number) => onTransport({ frameIndex: clampIndex(frameIndex + delta) }),
    [clampIndex, frameIndex, onTransport],
  );

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.target instanceof HTMLInputElement) return;
    switch (e.key) {
      case " ":
      case "k":
        e.preventDefault();
        onTransport({ playing: !transport.playing });
        break;
      case "ArrowLeft":
        e.preventDefault();
        step(-1);
        break;
      case "ArrowRight":
        e.preventDefault();
        step(1);
        break;
      case "Home":
        e.preventDefault();
        onTransport({ frameIndex: 0 });
        break;
      case "End":
        e.preventDefault();
        onTransport({ frameIndex: n - 1 });
        break;
      case "l":
        onTransport({ loop: !transport.loop });
        break;
      case "b":
        if (!embed) onTransport({ blink: !transport.blink });
        break;
    }
  };

  const meta = current?.meta;
  const ariaFrame = meta === undefined
    ? "No frames"
    : `Frame ${displayIndex + 1} of ${n}, detector ${meta.detector}, ${meta.wavelength_um.toFixed(2)} micrometers, observed ${mjdToIso(meta.obs_mjd_start)}`;

  return (
    <div
      className="tl-viewer"
      role="application"
      aria-label="Time-lapse viewer. Arrow keys step frames, space toggles playback."
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      <div className="tl-stage">
        {current === null ? (
          <div className="tl-empty" role="status">
            No frames loaded
          </div>
        ) : (
          <>
            <canvas
              ref={canvasRef}
              className="tl-canvas"
              role="img"
              aria-label={ariaFrame}
            />
            <div className="tl-overlay" aria-hidden="true">
              <span>{mjdToIso(meta?.obs_mjd_start ?? NaN)}</span>
              <span>D{meta?.detector ?? "?"}</span>
              <span>
                {meta !== undefined ? `${meta.wavelength_um.toFixed(2)} µm` : ""}
              </span>
              <span>
                {displayIndex + 1}/{n}
                {transport.blink ? (blinkPhase === 0 ? " ·A" : " ·B") : ""}
              </span>
              {meta !== undefined && (meta.flags?.length ?? 0) > 0 ? (
                <span className="tl-flag">{meta.flags?.join(", ")}</span>
              ) : null}
            </div>
          </>
        )}
      </div>
      <div className="tl-transport">
        <button
          type="button"
          aria-label="Previous frame"
          onClick={() => step(-1)}
          disabled={n === 0}
        >
          ⟨
        </button>
        <button
          type="button"
          aria-label={transport.playing ? "Pause" : "Play"}
          aria-pressed={transport.playing}
          onClick={() => onTransport({ playing: !transport.playing })}
          disabled={n === 0}
        >
          {transport.playing ? "❚❚" : "▶"}
        </button>
        <button
          type="button"
          aria-label="Next frame"
          onClick={() => step(1)}
          disabled={n === 0}
        >
          ⟩
        </button>
        <input
          type="range"
          className="tl-scrubber"
          aria-label="Frame scrubber"
          min={0}
          max={Math.max(n - 1, 0)}
          step={1}
          value={frameIndex}
          onChange={(e) =>
            onTransport({ frameIndex: Number(e.currentTarget.value) })
          }
          disabled={n === 0}
        />
        {!embed ? (
          <>
            <button
              type="button"
              aria-label="Toggle loop"
              aria-pressed={transport.loop}
              title="Loop (L)"
              onClick={() => onTransport({ loop: !transport.loop })}
            >
              ∞
            </button>
            <button
              type="button"
              aria-label="Toggle blink comparator"
              aria-pressed={transport.blink}
              title="Blink A/B (B)"
              onClick={() => onTransport({ blink: !transport.blink })}
            >
              A/B
            </button>
            {transport.blink ? (
              <button
                type="button"
                aria-label="Set blink reference to current frame"
                title="Anchor frame A to the current frame"
                onClick={() => setBlinkA(frameIndex)}
              >
                A={anchor + 1}
              </button>
            ) : null}
            <label className="tl-fps">
              <span className="tl-fps-label">{transport.fps} fps</span>
              <input
                type="range"
                aria-label="Playback speed"
                min={FPS_MIN}
                max={FPS_MAX}
                step={1}
                value={transport.fps}
                onChange={(e) =>
                  onTransport({ fps: Number(e.currentTarget.value) })
                }
              />
            </label>
          </>
        ) : null}
      </div>
      <span className="tl-sr" role="status" aria-live="polite">
        {ariaFrame}
      </span>
    </div>
  );
}

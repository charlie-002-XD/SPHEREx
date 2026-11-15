// Comparator transport: slider / blink segmented control, blink play-pause
// and the blink interval (seconds per epoch).
import type { CompareMode } from "./EpochComparator";

export const BLINK_MIN_MS = 100;
export const BLINK_MAX_MS = 2000;
export const BLINK_DEFAULT_MS = 500;

export interface CompareControlsProps {
  mode: CompareMode;
  onMode: (m: CompareMode) => void;
  playing: boolean;
  onPlaying: (p: boolean) => void;
  intervalMs: number;
  onInterval: (ms: number) => void;
  disabled?: boolean;
}

const MODES: ReadonlyArray<{ id: CompareMode; label: string; hint: string }> = [
  {
    id: "slider",
    label: "Slider",
    hint: "Drag the divider between epoch A and epoch B",
  },
  {
    id: "blink",
    label: "Blink",
    hint: "Blink comparator: alternate epoch A and epoch B",
  },
];

export function CompareControls(props: CompareControlsProps) {
  const blink = props.mode === "blink";
  return (
    <div className="cmp-controls">
      <div
        className="ms-group cmp-modes"
        role="radiogroup"
        aria-label="Comparison mode"
      >
        {MODES.map((m) => (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={props.mode === m.id}
            className={props.mode === m.id ? "ms-opt ms-on" : "ms-opt"}
            title={m.hint}
            onClick={() => props.onMode(m.id)}
          >
            {m.label}
          </button>
        ))}
      </div>
      <div className={blink ? "cmp-blinkctl" : "cmp-blinkctl cmp-off"}>
        <button
          type="button"
          className="tl-btn tl-btn-play"
          aria-label={props.playing ? "Pause blink" : "Play blink"}
          aria-pressed={props.playing}
          disabled={!blink || props.disabled === true}
          onClick={() => props.onPlaying(!props.playing)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
            {props.playing ? (
              <path
                d="M6 4v8M10 4v8"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            ) : (
              <path
                d="M5.5 3.5v9l7-4.5z"
                fill="currentColor"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinejoin="round"
              />
            )}
          </svg>
        </button>
        <label className="cmp-speed">
          <span>Speed</span>
          <input
            type="range"
            min={BLINK_MIN_MS}
            max={BLINK_MAX_MS}
            step={50}
            value={props.intervalMs}
            disabled={!blink || props.disabled === true}
            aria-label="Blink interval"
            aria-valuetext={`${(props.intervalMs / 1000).toFixed(2)} seconds`}
            onChange={(e) => props.onInterval(Number(e.currentTarget.value))}
          />
          <output className="num">
            {(props.intervalMs / 1000).toFixed(2)} s
          </output>
        </label>
      </div>
    </div>
  );
}

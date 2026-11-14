// Display-mode segmented control (raw / difference / co-add / detector
// stack) plus a detector filter (all arrays or a single D1-D6 group).
import { API_MODES, DETECTOR_IDS, type ApiMode } from "../../lib/types";
import "./controls.css";

const MODE_LABELS: Record<ApiMode, string> = {
  raw: "Raw",
  diff: "Diff t₂−t₁",
  coadd: "Co-add t₂+t₁",
  stack: "Det stack",
};

const MODE_HINTS: Record<ApiMode, string> = {
  raw: "Observed frames as delivered",
  diff: "Each frame minus the first frame of its detector group",
  coadd: "Running co-addition against the first frame",
  stack: "Per-detector-array co-added stacks",
};

export interface ModeSelectorProps {
  mode: ApiMode;
  detector: number | null;
  onChange: (patch: { mode?: ApiMode; detector?: number | null }) => void;
}

export function ModeSelector(props: ModeSelectorProps) {
  const { mode, detector, onChange } = props;
  return (
    <div className="ms">
      <div
        className="ms-group"
        role="radiogroup"
        aria-label="Display mode"
      >
        {API_MODES.map((m) => (
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={mode === m}
            className={mode === m ? "ms-opt ms-on" : "ms-opt"}
            title={MODE_HINTS[m]}
            onClick={() => onChange({ mode: m })}
          >
            {MODE_LABELS[m]}
          </button>
        ))}
      </div>
      <div
        className="ms-group"
        role="radiogroup"
        aria-label="Detector filter"
      >
        <button
          type="button"
          role="radio"
          aria-checked={detector === null}
          className={detector === null ? "ms-opt ms-on" : "ms-opt"}
          onClick={() => onChange({ detector: null })}
        >
          All
        </button>
        {DETECTOR_IDS.map((d) => (
          <button
            key={d}
            type="button"
            role="radio"
            aria-checked={detector === d}
            className={detector === d ? "ms-opt ms-on" : "ms-opt"}
            onClick={() => onChange({ detector: d })}
          >
            D{d}
          </button>
        ))}
      </div>
    </div>
  );
}

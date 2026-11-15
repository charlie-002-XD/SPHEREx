// Near-IR band picker with frame counts and a wavelength scale. Bands with
// no frames in the current series are disabled rather than hidden.
import { NIR_BANDS } from "../../lib/bands";

export interface BandSelectorProps {
  value: string;
  counts: Record<string, number>;
  onChange: (id: string) => void;
}

export function BandSelector(props: BandSelectorProps) {
  const first = NIR_BANDS[0];
  const last = NIR_BANDS[NIR_BANDS.length - 1];
  const lo = first?.minUm ?? 0;
  const hi = last?.maxUm ?? 1;
  return (
    <div className="bsel">
      <div
        className="bsel-list"
        role="radiogroup"
        aria-label="Near-infrared band"
      >
        {NIR_BANDS.map((b) => {
          const n = props.counts[b.id] ?? 0;
          const on = props.value === b.id;
          return (
            <button
              key={b.id}
              type="button"
              role="radio"
              aria-checked={on}
              disabled={n === 0}
              className={on ? "bsel-opt bsel-on" : "bsel-opt"}
              style={{ "--band": b.color } as React.CSSProperties}
              onClick={() => props.onChange(b.id)}
            >
              <span className="bsel-swatch" aria-hidden="true" />
              <span className="bsel-text">
                <span className="bsel-name num">
                  {b.centerUm.toFixed(1)} µm{" "}
                  <span className="bsel-dim">· {b.label}</span>
                </span>
                <span className="bsel-note">{b.note}</span>
              </span>
              <span className="bsel-count num">
                {n} {n === 1 ? "frame" : "frames"}
              </span>
            </button>
          );
        })}
      </div>
      <div className="bsel-scale" aria-hidden="true">
        <div className="bsel-bar">
          {NIR_BANDS.map((b) => (
            <span
              key={b.id}
              className={
                props.value === b.id ? "bsel-seg bsel-seg-on" : "bsel-seg"
              }
              style={
                {
                  "--band": b.color,
                  left: `${((b.minUm - lo) / (hi - lo)) * 100}%`,
                  width: `${((b.maxUm - b.minUm) / (hi - lo)) * 100}%`,
                } as React.CSSProperties
              }
            />
          ))}
        </div>
        <div className="bsel-ticks num">
          <span>{lo.toFixed(2)} µm</span>
          <span>{hi.toFixed(1)} µm</span>
        </div>
      </div>
    </div>
  );
}

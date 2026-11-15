// RA/Dec inputs + FOV slider + preset pills for the five official targets.
// Validation mirrors the backend ranges (see lib/coords); raw user input is
// never echoed back in error messages.
import { useEffect, useState } from "react";
import {
  FOV_MAX_DEG,
  FOV_MIN_DEG,
  parseDecToDeg,
  parseRaToDeg,
} from "../../lib/coords";
import { PRESET_TARGETS } from "../../lib/presets";
import "./controls.css";

export interface CoordinateValue {
  ra: string;
  dec: string;
  fov: number;
}

export interface CoordinateSearchBarProps {
  value: CoordinateValue;
  onSubmit: (v: CoordinateValue) => void;
  loading?: boolean;
}

export function CoordinateSearchBar(props: CoordinateSearchBarProps) {
  const { value, onSubmit, loading = false } = props;
  const [ra, setRa] = useState(value.ra);
  const [dec, setDec] = useState(value.dec);
  const [fov, setFov] = useState(value.fov);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => setRa(value.ra), [value.ra]);
  useEffect(() => setDec(value.dec), [value.dec]);
  useEffect(() => setFov(value.fov), [value.fov]);

  const submit = () => {
    if (parseRaToDeg(ra) === null) {
      setError("Invalid RA. Use decimal degrees (0-360) or sexagesimal hours, e.g. 08h55m10.8s.");
      return;
    }
    if (parseDecToDeg(dec) === null) {
      setError("Invalid Dec. Use decimal degrees (-90..90) or sexagesimal, e.g. -07d14m42.5s.");
      return;
    }
    setError(null);
    onSubmit({ ra: ra.trim(), dec: dec.trim(), fov });
  };

  const onKey = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") submit();
  };

  return (
    <div className="csb" role="search" aria-label="Sky position search">
      <div className="csb-row">
        <label className="csb-field csb-ra">
          <span className="csb-label">RA</span>
          <input
            type="text"
            inputMode="text"
            placeholder="deg or hms"
            value={ra}
            onChange={(e) => setRa(e.currentTarget.value)}
            onKeyDown={onKey}
            aria-label="Right ascension"
            aria-invalid={error !== null && parseRaToDeg(ra) === null}
          />
        </label>
        <label className="csb-field csb-dec">
          <span className="csb-label">Dec</span>
          <input
            type="text"
            inputMode="text"
            placeholder="deg or dms"
            value={dec}
            onChange={(e) => setDec(e.currentTarget.value)}
            onKeyDown={onKey}
            aria-label="Declination"
            aria-invalid={error !== null && parseDecToDeg(dec) === null}
          />
        </label>
        <label className="csb-field csb-fov">
          <span className="csb-label">FOV</span>
          <input
            type="range"
            min={FOV_MIN_DEG}
            max={FOV_MAX_DEG}
            step={0.01}
            value={fov}
            onChange={(e) => setFov(Number(e.currentTarget.value))}
            aria-label="Field of view in degrees"
          />
          <output className="csb-fov-val num">{fov.toFixed(2)}°</output>
        </label>
        <button
          type="button"
          className="csb-go"
          onClick={submit}
          disabled={loading}
          aria-label={loading ? "Loading…" : "Go: load time-lapse"}
        >
          {loading ? "Loading…" : "Go"}
        </button>
      </div>
      {error !== null ? (
        <p className="csb-error" role="alert">
          {error}
        </p>
      ) : null}
      <div className="csb-presets" aria-label="Official targets">
        {PRESET_TARGETS.map((t) => (
          <button
            key={t.id}
            type="button"
            className="csb-pill"
            title={t.note ?? `${t.ra} ${t.dec}`}
            onClick={() => {
              setRa(t.ra);
              setDec(t.dec);
              setFov(t.fov);
              setError(null);
              onSubmit({ ra: t.ra, dec: t.dec, fov: t.fov });
            }}
          >
            {t.label}
          </button>
        ))}
      </div>
    </div>
  );
}

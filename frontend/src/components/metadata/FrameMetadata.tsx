// Frame metadata card: observation time, detector, wavelength, collection /
// release and the API-provided flag names. Nothing about flag semantics is
// hardcoded; the optional legend (bit -> description, straight from the FITS
// header) is shown verbatim in a collapsible block.
import type { Frame } from "../../lib/types";
import { mjdToIso } from "../../lib/coords";
import { FlagBadges } from "./FlagBadges";
import "./metadata.css";

export interface FrameMetadataProps {
  frame: Frame | null;
  unit?: string;
  flagLegend?: Record<string, string>;
}

function fmtUm(v: number | undefined): string {
  return v === undefined || !Number.isFinite(v) ? "—" : `${v.toFixed(2)} µm`;
}

export function FrameMetadata({ frame, unit, flagLegend }: FrameMetadataProps) {
  if (frame === null) {
    return (
      <section className="md-card" aria-label="Frame metadata">
        <p className="md-empty">No frame selected.</p>
      </section>
    );
  }
  const flags = frame.flags ?? [];
  const legendEntries = Object.entries(flagLegend ?? {}).sort(
    (a, b) => Number(a[0]) - Number(b[0]),
  );
  return (
    <section className="md-card" aria-label="Frame metadata">
      <dl className="md-grid">
        <dt>Observed</dt>
        <dd>
          {mjdToIso(frame.obs_mjd_start)}
          {frame.obs_mjd_end !== frame.obs_mjd_start
            ? ` – ${mjdToIso(frame.obs_mjd_end)}`
            : ""}{" "}
          <span className="md-dim">(MJD {frame.obs_mjd_start.toFixed(3)})</span>
        </dd>
        <dt>Detector</dt>
        <dd>D{frame.detector}</dd>
        <dt>Wavelength</dt>
        <dd>
          {fmtUm(frame.wavelength_um)}
          {frame.bandwidth_um !== undefined ? (
            <span className="md-dim"> (Δλ {fmtUm(frame.bandwidth_um)})</span>
          ) : null}
        </dd>
        <dt>Collection</dt>
        <dd className="md-mono">{frame.collection}</dd>
        {unit !== undefined ? (
          <>
            <dt>Unit</dt>
            <dd className="md-mono">{unit}</dd>
          </>
        ) : null}
        {frame.zodi_subtracted !== undefined ? (
          <>
            <dt>Zodiacal light</dt>
            <dd>{frame.zodi_subtracted ? "subtracted" : "not subtracted"}</dd>
          </>
        ) : null}
        <dt>Flags</dt>
        <dd>
          {flags.length > 0 ? (
            <FlagBadges flags={flags} legend={flagLegend} />
          ) : (
            <span className="md-dim">none</span>
          )}
        </dd>
      </dl>
      {legendEntries.length > 0 ? (
        <details className="md-legend">
          <summary>Flag legend (FITS header)</summary>
          <dl>
            {legendEntries.map(([bit, desc]) => (
              <div key={bit} className="md-legend-row">
                <dt className="md-mono">bit {bit}</dt>
                <dd>{desc}</dd>
              </div>
            ))}
          </dl>
        </details>
      ) : null}
    </section>
  );
}

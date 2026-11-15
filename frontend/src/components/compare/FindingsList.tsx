// Saved findings, newest first. Storage is local until the API contract
// gains a findings endpoint; the note says so instead of implying a server.
import { bandById } from "../../lib/bands";
import { FINDING_KIND_LABELS, type Finding } from "../../lib/findings";

export interface FindingsListProps {
  findings: readonly Finding[];
  storage: "local" | "remote";
  onRemove: (id: string) => void;
}

export function FindingsList(props: FindingsListProps) {
  const list = [...props.findings].reverse();
  return (
    <div className="fl">
      <p className="fl-note">
        {props.storage === "local"
          ? "Saved in this browser only (no findings endpoint in the API yet)."
          : "Saved on the server."}
      </p>
      {list.length === 0 ? (
        <p className="fl-empty">
          No findings yet. In Compare view, click any object that moves between
          epochs.
        </p>
      ) : (
        <ul className="fl-list" aria-label="Saved findings">
          {list.map((f) => {
            const band = bandById(f.band);
            return (
              <li key={f.id} className="fl-item">
                <span
                  className="fl-swatch"
                  style={
                    {
                      "--band": band?.color ?? "var(--accent)",
                    } as React.CSSProperties
                  }
                  aria-hidden="true"
                />
                <div className="fl-body">
                  <div className="fl-head num">
                    <span>
                      {FINDING_KIND_LABELS[f.kind]} · ({f.x}, {f.y})
                    </span>
                    <span className="fl-dim">
                      {band !== null
                        ? `${band.centerUm.toFixed(1)} µm`
                        : f.band}{" "}
                      · {f.epoch}
                    </span>
                  </div>
                  {f.ra_deg !== null && f.dec_deg !== null ? (
                    <div className="fl-dim num">
                      RA {f.ra_deg.toFixed(5)}° · Dec {f.dec_deg.toFixed(5)}°
                    </div>
                  ) : null}
                  {f.note !== "" ? <p className="fl-text">{f.note}</p> : null}
                </div>
                <button
                  type="button"
                  className="tl-btn fl-remove"
                  aria-label={`Delete finding at ${f.x}, ${f.y}`}
                  onClick={() => props.onRemove(f.id)}
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
                    <path
                      d="M4 4l8 8M12 4l-8 8"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                    />
                  </svg>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

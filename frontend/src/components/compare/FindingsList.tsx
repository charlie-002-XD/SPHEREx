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
  const list = props.findings.map((f, i) => ({ f, n: i + 1 })).reverse();
  return (
    <div className="fl">
      <p className="fl-note">
        {props.storage === "local"
          ? "Saved in this browser only (no findings endpoint in the API yet)."
          : "Saved on the server."}
      </p>
      {list.length === 0 ? (
        <p className="fl-empty">No reports yet.</p>
      ) : (
        <ul className="fl-list" aria-label="Saved findings">
          {list.map(({ f, n }) => {
            const band = bandById(f.band);
            return (
              <li key={f.id} className="fl-item">
                <div
                  className="fl-body"
                  style={
                    {
                      "--band": band?.color ?? "var(--accent)",
                    } as React.CSSProperties
                  }
                >
                  <div className="fl-head num">
                    <span>
                      #{n} · ({f.x}, {f.y})
                    </span>
                    <span className="fl-band">
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
                  <p className="fl-text">
                    <span className="fl-kind">
                      {FINDING_KIND_LABELS[f.kind]}
                    </span>
                    {f.note !== "" ? ` — ${f.note}` : null}
                  </p>
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

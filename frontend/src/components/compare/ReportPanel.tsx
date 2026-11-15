// Right column of the sky layout: "New report" (help text, or the finding
// form once a point is marked) and the saved findings list.
import { FindingForm } from "./FindingForm";
import { FindingsList } from "./FindingsList";
import type { PendingMark } from "./EpochComparator";
import type { Finding, FindingKind } from "../../lib/findings";

export interface ReportPanelProps {
  pending: PendingMark | null;
  findings: readonly Finding[];
  storage: "local" | "remote";
  onSave: (kind: FindingKind, note: string) => Promise<void>;
  onCancel: () => void;
  onSpectrum: (raDeg: number, decDeg: number) => void;
  onRemove: (id: string) => void;
}

export function ReportPanel(props: ReportPanelProps) {
  return (
    <>
      <section className="sky-card sky-panel" aria-labelledby="rp-new">
        <h2 id="rp-new" className="sky-card-title">
          New report
        </h2>
        {props.pending !== null ? (
          <FindingForm
            key={`${props.pending.frameId}-${props.pending.x}-${props.pending.y}`}
            pending={props.pending}
            onSave={props.onSave}
            onCancel={props.onCancel}
            onSpectrum={props.onSpectrum}
          />
        ) : (
          <p className="sky-help">
            Compare both epochs and <strong>click</strong> any object that
            changes position (asteroid, comet or planet) to mark its
            coordinates.
          </p>
        )}
      </section>
      <section
        className="sky-card sky-panel sky-findings"
        aria-labelledby="rp-findings"
      >
        <h2 id="rp-findings" className="sky-card-title">
          <span>
            Findings{" "}
            <span className="sky-count">({props.findings.length})</span>
          </span>
          <span className="sky-status">
            {props.storage === "local" ? "This browser" : "Server"}
          </span>
        </h2>
        <FindingsList
          findings={props.findings}
          storage={props.storage}
          onRemove={props.onRemove}
        />
      </section>
    </>
  );
}

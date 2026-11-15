// Right column of the sky layout: "New report" (help text, or the finding
// form once a point is marked) and the saved findings list, each in a
// collapsible panel.
import { FindingForm } from "./FindingForm";
import { FindingsList } from "./FindingsList";
import type { PendingMark } from "./EpochComparator";
import { SkyPanel } from "../layout/SkyPanel";
import type { Finding, FindingKind } from "../../lib/findings";

export interface ReportPanelProps {
  pending: PendingMark | null;
  findings: readonly Finding[];
  storage: "local" | "remote";
  reportOpen: boolean;
  findingsOpen: boolean;
  onToggleReport: (open: boolean) => void;
  onToggleFindings: (open: boolean) => void;
  onSave: (kind: FindingKind, note: string) => Promise<void>;
  onCancel: () => void;
  onSpectrum: (raDeg: number, decDeg: number) => void;
  onRemove: (id: string) => void;
}

export function ReportPanel(props: ReportPanelProps) {
  return (
    <>
      <SkyPanel
        id="rp-new"
        title="New report"
        open={props.reportOpen}
        onToggle={props.onToggleReport}
      >
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
      </SkyPanel>
      <SkyPanel
        id="rp-findings"
        className="sky-findings"
        title={
          <>
            Findings{" "}
            <span className="sky-count">({props.findings.length})</span>
          </>
        }
        aside={
          <span className="sky-status">
            {props.storage === "local" ? "This browser" : "Server"}
          </span>
        }
        open={props.findingsOpen}
        onToggle={props.onToggleFindings}
      >
        <FindingsList
          findings={props.findings}
          storage={props.storage}
          onRemove={props.onRemove}
        />
      </SkyPanel>
    </>
  );
}

// Compare view: picks epochs A/B for the selected band from the fetched
// frames and composes the comparator, its controls and the finding form.
import { useEffect, useMemo, useState } from "react";
import {
  EpochComparator,
  type CompareMode,
  type PendingMark,
} from "./EpochComparator";
import { BLINK_DEFAULT_MS, CompareControls } from "./CompareControls";
import { FindingForm } from "./FindingForm";
import {
  bandById,
  formatDeltaT,
  NIR_BANDS,
  selectEpochPair,
} from "../../lib/bands";
import type { Finding, FindingKind } from "../../lib/findings";
import type { DecodedFrame } from "../../lib/types";
import type { StretchKind } from "../../lib/stretch";
import type { ColormapName } from "../../lib/colormaps";

export interface CompareViewProps {
  frames: readonly DecodedFrame[];
  width: number;
  height: number;
  stretch: StretchKind;
  cmap: ColormapName;
  bandId: string;
  findings: readonly Finding[];
  pending: PendingMark | null;
  onMark: (m: PendingMark | null) => void;
  onSave: (kind: FindingKind, note: string) => Promise<Finding>;
  onSpectrum: (raDeg: number, decDeg: number) => void;
}

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

export function CompareView(props: CompareViewProps) {
  const [mode, setMode] = useState<CompareMode>("slider");
  const [playing, setPlaying] = useState(() => !prefersReducedMotion());
  const [intervalMs, setIntervalMs] = useState(BLINK_DEFAULT_MS);
  const [toast, setToast] = useState<string | null>(null);

  const band = bandById(props.bandId) ?? NIR_BANDS[0] ?? null;
  const pair = useMemo(
    () => (band !== null ? selectEpochPair(props.frames, band) : null),
    [props.frames, band],
  );

  useEffect(() => {
    if (toast === null) return;
    const id = window.setTimeout(() => setToast(null), 3000);
    return () => window.clearTimeout(id);
  }, [toast]);

  if (band === null || pair === null) {
    return (
      <p className="cmp-empty" role="status">
        No frames in this band for this position. Pick another band in Controls.
      </p>
    );
  }

  return (
    <>
      <CompareControls
        mode={mode}
        onMode={setMode}
        playing={playing}
        onPlaying={setPlaying}
        intervalMs={intervalMs}
        onInterval={setIntervalMs}
        disabled={pair.b === null}
      />
      <EpochComparator
        pair={pair}
        band={band}
        width={props.width}
        height={props.height}
        stretch={props.stretch}
        cmap={props.cmap}
        mode={mode}
        intervalMs={intervalMs}
        playing={playing}
        findings={props.findings}
        pending={props.pending}
        onMark={props.onMark}
      />
      <p className="cmp-meta num">
        {pair.b !== null && pair.dtDays !== null
          ? `Δt = ${formatDeltaT(pair.dtDays)} (${pair.dtDays.toFixed(1)} d) · D${pair.a.meta.detector} ${pair.a.meta.wavelength_um.toFixed(2)} µm → D${pair.b.meta.detector} ${pair.b.meta.wavelength_um.toFixed(2)} µm`
          : "Only one epoch in this band: nothing to compare yet."}
      </p>
      {props.pending !== null ? (
        <FindingForm
          pending={props.pending}
          onCancel={() => props.onMark(null)}
          onSpectrum={props.onSpectrum}
          onSave={async (kind, note) => {
            const f = await props.onSave(kind, note);
            setToast(`Finding saved at (${f.x}, ${f.y}).`);
          }}
        />
      ) : null}
      <p className="cmp-toast" role="status" aria-live="polite">
        {toast ?? ""}
      </p>
    </>
  );
}

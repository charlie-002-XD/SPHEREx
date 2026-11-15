// Classify the pending mark (asteroid / comet / planet), add a note and save
// it, or sample a spectrum at the same sky position.
import { useEffect, useState, type FormEvent } from "react";
import {
  FINDING_KINDS,
  FINDING_KIND_LABELS,
  NOTE_MAX,
  type FindingKind,
} from "../../lib/findings";
import { bandById } from "../../lib/bands";
import type { PendingMark } from "./EpochComparator";

export interface FindingFormProps {
  pending: PendingMark;
  onSave: (kind: FindingKind, note: string) => Promise<void>;
  onCancel: () => void;
  onSpectrum: (raDeg: number, decDeg: number) => void;
}

export function FindingForm(props: FindingFormProps) {
  const { pending } = props;
  const [kind, setKind] = useState<FindingKind>("asteroid");
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => setError(null), [pending]);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    try {
      await props.onSave(kind, note.trim());
      setNote("");
    } catch {
      setError("Could not save the finding.");
    } finally {
      setSaving(false);
    }
  };

  const band = bandById(pending.band);
  const hasSky = pending.raDeg !== null && pending.decDeg !== null;

  return (
    <form
      className="ff"
      aria-label="New finding"
      onSubmit={(e) => {
        void submit(e);
      }}
    >
      <dl className="ff-data num">
        <div>
          <dt>X</dt>
          <dd>{pending.x}</dd>
        </div>
        <div>
          <dt>Y</dt>
          <dd>{pending.y}</dd>
        </div>
        <div>
          <dt>RA</dt>
          <dd>
            {pending.raDeg !== null ? `${pending.raDeg.toFixed(5)}°` : "—"}
          </dd>
        </div>
        <div>
          <dt>Dec</dt>
          <dd>
            {pending.decDeg !== null ? `${pending.decDeg.toFixed(5)}°` : "—"}
          </dd>
        </div>
        <div>
          <dt>Band</dt>
          <dd>{band !== null ? `${band.centerUm.toFixed(1)} µm` : "—"}</dd>
        </div>
        <div>
          <dt>Epoch</dt>
          <dd>{pending.epoch}</dd>
        </div>
      </dl>
      <div className="ms-group" role="radiogroup" aria-label="Object type">
        {FINDING_KINDS.map((k) => (
          <button
            key={k}
            type="button"
            role="radio"
            aria-checked={kind === k}
            className={kind === k ? "ms-opt ms-on" : "ms-opt"}
            onClick={() => setKind(k)}
          >
            {FINDING_KIND_LABELS[k]}
          </button>
        ))}
      </div>
      <label className="ff-note">
        <span className="visually-hidden">Note</span>
        <textarea
          value={note}
          maxLength={NOTE_MAX}
          rows={2}
          placeholder="Note: brightness, direction of motion…"
          onChange={(e) => setNote(e.currentTarget.value)}
        />
      </label>
      {error !== null ? (
        <p className="ff-error" role="alert">
          {error}
        </p>
      ) : null}
      <div className="ff-actions">
        <button type="submit" className="ff-btn ff-primary" disabled={saving}>
          {saving ? "Saving…" : "Save finding"}
        </button>
        <button
          type="button"
          className="ff-btn"
          disabled={!hasSky}
          onClick={() => {
            if (pending.raDeg !== null && pending.decDeg !== null) {
              props.onSpectrum(pending.raDeg, pending.decDeg);
            }
          }}
        >
          Spectrum
        </button>
        <button type="button" className="ff-btn" onClick={props.onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}

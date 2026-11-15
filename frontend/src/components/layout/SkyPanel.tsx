// Collapsible glass panel for the sky layout's side columns. The header is
// a 44px disclosure button; the body is hidden (not unmounted from layout
// flow) when collapsed so the column keeps its place in the grid.
import type { ReactNode } from "react";

export interface SkyPanelProps {
  id: string;
  title: ReactNode;
  /** Extra header content after the toggle (counts, status). */
  aside?: ReactNode;
  open: boolean;
  onToggle: (open: boolean) => void;
  className?: string;
  children: ReactNode;
}

export function SkyPanel(props: SkyPanelProps) {
  const { id, open } = props;
  const cls = ["sky-card", "sky-panel", props.className ?? ""];
  if (!open) cls.push("sky-panel-closed");
  return (
    <section className={cls.join(" ").trim()} aria-labelledby={`${id}-title`}>
      <h2 className="sky-card-title">
        <button
          type="button"
          id={`${id}-title`}
          className="sky-panel-toggle"
          aria-expanded={open}
          aria-controls={`${id}-body`}
          onClick={() => props.onToggle(!open)}
        >
          <span>{props.title}</span>
          <svg
            className="sky-panel-chev"
            viewBox="0 0 16 16"
            aria-hidden="true"
            focusable="false"
          >
            <path
              d="M4 6l4 4 4-4"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
        {props.aside ?? null}
      </h2>
      <div id={`${id}-body`} className="sky-panel-body" hidden={!open}>
        {props.children}
      </div>
    </section>
  );
}

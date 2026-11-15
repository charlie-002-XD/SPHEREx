// App shell. Mobile-first: a single-line header (name + target menu), the
// viewer on top, a compact search row, and secondary controls in a bottom
// sheet with a grab-handle toggle. From 640px up the sheet becomes a slim
// collapsible side panel (collapses to a vertical rail). The optional
// `drawer` (spectrum) is docked into the layout, never floating: under the
// image by default, beside it on wide landscape screens, so it can never
// cover the search row, the transport or the controls panel. In embed mode
// (?embed=1) only the viewer area is rendered so it fills an iframe.
import { useState, type ReactNode } from "react";
import type { CoordinateValue } from "../controls/CoordinateSearchBar";
import { TargetMenu, type MenuTarget } from "./TargetMenu";
import { useMediaQuery } from "./useMediaQuery";
import "./app-shell.css";
import "./sky.css";

export const DESKTOP_QUERY = "(min-width: 640px)";

export interface AppShellProps {
  embed: boolean;
  targets: readonly MenuTarget[];
  onSelectTarget: (v: CoordinateValue) => void;
  search: ReactNode;
  panel: ReactNode;
  /** Docked secondary view (spectrum); null when closed. */
  drawer?: ReactNode;
  /** Extra header actions rendered after the target menu (About button). */
  headerActions?: ReactNode;
  /** One-line notice under the header (first-visit hint); null when none. */
  notice?: ReactNode;
  /** Attribution line rendered in embed mode only. */
  credit?: ReactNode;
  /** "sky": dark three-column comparator layout (bands | viewer | report). */
  variant?: "default" | "sky";
  /** Sky layout: left column (band selector, display controls). */
  left?: ReactNode;
  /** Sky layout: right column (new report, findings). */
  right?: ReactNode;
  /** Sky layout: one-line description after the app name. */
  subtitle?: ReactNode;
  /** Sky layout: quiet data line on the right of the header. */
  headerMeta?: ReactNode;
  children: ReactNode;
}

function AppMark() {
  return (
    <svg
      className="app-mark"
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      <circle
        cx="8"
        cy="8"
        r="6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <circle cx="8" cy="8" r="2" fill="currentColor" />
    </svg>
  );
}

// Sky layout. Phones: one scrolling column (viewer, report, bands). From
// 640px: viewer beside a side column holding both panels. From 1200px:
// bands | viewer | report, each column scrolling on its own.
function SkyShell(props: AppShellProps) {
  const drawer = props.drawer ?? null;
  return (
    <div className="app app-sky">
      <div className="sky-bg" aria-hidden="true" />
      <header className="app-header sky-header">
        <div className="sky-heading">
          <p className="sky-eyebrow">SPHEREx · near-infrared sky</p>
          <h1 className="app-title sky-title">
            <AppMark />
            SPHEREx Blink
            {props.subtitle !== undefined ? (
              <span className="sky-subtitle">· {props.subtitle}</span>
            ) : null}
          </h1>
        </div>
        <div className="app-actions">
          {props.headerMeta !== undefined ? (
            <div className="sky-meta num">{props.headerMeta}</div>
          ) : null}
          <TargetMenu targets={props.targets} onSelect={props.onSelectTarget} />
          {props.headerActions ?? null}
        </div>
      </header>
      {props.notice ?? null}
      <div className="sky-body">
        <div className="sky-center">
          <div className="app-search sky-card">{props.search}</div>
          <main className="app-main sky-main">{props.children}</main>
          {drawer !== null ? (
            <div className="app-drawer sky-card sky-drawer">{drawer}</div>
          ) : null}
        </div>
        <aside className="sky-left" aria-label="Bands and display controls">
          {props.left ?? null}
        </aside>
        <aside className="sky-right" aria-label="Reports and findings">
          {props.right ?? null}
        </aside>
      </div>
    </div>
  );
}

export function AppShell(props: AppShellProps) {
  const { embed, targets, onSelectTarget, search, panel, children } = props;
  const drawer = props.drawer ?? null;
  const desktop = useMediaQuery(DESKTOP_QUERY);
  const [panelOpen, setPanelOpen] = useState(desktop);

  if (embed) {
    return (
      <div className="app app-embed">
        <main className="app-main">{children}</main>
        {props.credit ?? null}
      </div>
    );
  }

  if (props.variant === "sky") return <SkyShell {...props} />;

  return (
    <div className="app">
      <header className="app-header">
        <h1 className="app-title">
          <svg
            className="app-mark"
            viewBox="0 0 16 16"
            aria-hidden="true"
            focusable="false"
          >
            <circle
              cx="8"
              cy="8"
              r="6"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
            />
            <circle cx="8" cy="8" r="2" fill="currentColor" />
          </svg>
          SPHEREx Blink
        </h1>
        <div className="app-actions">
          <TargetMenu targets={targets} onSelect={onSelectTarget} />
          {props.headerActions ?? null}
        </div>
      </header>
      {props.notice ?? null}
      <div className="app-body">
        <div
          className={drawer !== null ? "app-view app-view-drawer" : "app-view"}
        >
          <div className="app-search">{search}</div>
          <main className="app-main">{children}</main>
          {drawer !== null ? <div className="app-drawer">{drawer}</div> : null}
        </div>
        <aside
          className={panelOpen ? "app-panel" : "app-panel app-panel-closed"}
          aria-label="Secondary controls"
        >
          <button
            type="button"
            className="app-panel-toggle"
            aria-expanded={panelOpen}
            aria-controls="app-panel-body"
            onClick={() => setPanelOpen((o) => !o)}
          >
            <span className="app-panel-grip" aria-hidden="true" />
            <span className="app-panel-toggle-label">Controls</span>
            <svg
              className="app-chev"
              viewBox="0 0 16 16"
              aria-hidden="true"
              focusable="false"
            >
              <path
                d="M6 4l4 4-4 4"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.6"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
          <div
            className="app-panel-body"
            id="app-panel-body"
            hidden={!panelOpen}
          >
            {panel}
          </div>
        </aside>
      </div>
    </div>
  );
}

// App shell. Mobile-first: a single-line header (name + target menu), the
// viewer on top, a compact search row, and secondary controls in a bottom
// sheet with a grab-handle toggle. From 640px up the sheet becomes a slim
// collapsible 320px side panel (collapses to a vertical rail). In embed
// mode (?embed=1) only the viewer area is rendered so it can fill an
// iframe with no scrollbars.
import { useState, type ReactNode } from "react";
import type { CoordinateValue } from "../controls/CoordinateSearchBar";
import { TargetMenu, type MenuTarget } from "./TargetMenu";
import { useMediaQuery } from "./useMediaQuery";
import "./app-shell.css";

export const DESKTOP_QUERY = "(min-width: 640px)";

export interface AppShellProps {
  embed: boolean;
  targets: readonly MenuTarget[];
  onSelectTarget: (v: CoordinateValue) => void;
  search: ReactNode;
  panel: ReactNode;
  children: ReactNode;
}

export function AppShell(props: AppShellProps) {
  const { embed, targets, onSelectTarget, search, panel, children } = props;
  const desktop = useMediaQuery(DESKTOP_QUERY);
  const [panelOpen, setPanelOpen] = useState(desktop);

  if (embed) {
    return (
      <div className="app app-embed">
        <main className="app-main">{children}</main>
      </div>
    );
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1 className="app-title">SPHEREx Blink</h1>
        <TargetMenu targets={targets} onSelect={onSelectTarget} />
      </header>
      <div className="app-body">
        <div className="app-view">
          <div className="app-search">{search}</div>
          <main className="app-main">{children}</main>
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
          <div className="app-panel-body" id="app-panel-body" hidden={!panelOpen}>
            {panel}
          </div>
        </aside>
      </div>
    </div>
  );
}

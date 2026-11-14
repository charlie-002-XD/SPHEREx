// Full-area states for the main viewer region: empty (no query yet or no
// data), loading (single skeleton block, no spinners) and error with a
// Retry action. They fill .app-main the same way the canvas does.
export interface EmptyStateProps {
  title?: string;
  hint?: string;
}

export function EmptyState(props: EmptyStateProps) {
  const {
    title = "Point at the sky",
    hint = "Enter RA/Dec above or pick a target to load a SPHEREx time-lapse.",
  } = props;
  return (
    <div className="state">
      <div className="state-inner">
        <svg
          className="state-icon"
          viewBox="0 0 48 48"
          aria-hidden="true"
          focusable="false"
        >
          <circle
            cx="24"
            cy="24"
            r="15"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <circle cx="24" cy="24" r="3" fill="currentColor" />
          <path
            d="M24 3v8M24 37v8M3 24h8M37 24h8"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
          />
        </svg>
        <p className="state-title">{title}</p>
        <p className="state-hint">{hint}</p>
      </div>
    </div>
  );
}

export function LoadingSkeleton() {
  return (
    <div className="state-loading" role="status">
      <div className="skel" aria-hidden="true" />
      <span className="visually-hidden">Loading frames…</span>
    </div>
  );
}

export interface ErrorStateProps {
  message: string;
  onRetry: () => void;
}

export function ErrorState(props: ErrorStateProps) {
  const { message, onRetry } = props;
  return (
    <div className="state state-error" role="alert">
      <div className="state-inner">
        <svg
          className="state-icon"
          viewBox="0 0 48 48"
          aria-hidden="true"
          focusable="false"
        >
          <circle
            cx="24"
            cy="24"
            r="19"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <path
            d="M24 14v12"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
          />
          <circle cx="24" cy="33" r="1.8" fill="currentColor" />
        </svg>
        <p className="state-title">Could not load frames</p>
        <p className="state-hint">{message}</p>
        <button type="button" className="state-retry" onClick={onRetry}>
          Retry
        </button>
      </div>
    </div>
  );
}

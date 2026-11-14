// Error boundary for optional overlays: a failing overlay must never take
// the viewer down with it. Renders a subdued fallback instead of throwing.
import { Component, type ReactNode } from "react";

export interface OverlayErrorBoundaryProps {
  fallback?: ReactNode;
  children: ReactNode;
}

interface State {
  failed: boolean;
}

export class OverlayErrorBoundary extends Component<
  OverlayErrorBoundaryProps,
  State
> {
  override state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  override componentDidCatch(err: unknown): void {
    // Non-fatal by design; log for debugging only.
    console.warn("Overlay failed:", err);
  }

  override render(): ReactNode {
    if (this.state.failed) {
      return (
        this.props.fallback ?? (
          <p className="ao-error" role="status">
            Sky overlay unavailable.
          </p>
        )
      );
    }
    return this.props.children;
  }
}

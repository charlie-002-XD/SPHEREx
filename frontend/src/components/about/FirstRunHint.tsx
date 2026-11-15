// First-visit hint: a single dismissible line under the header, shown only
// when there is no stored dismissal. localStorage is optional — every
// access is wrapped so the app still works when storage is blocked (the
// hint simply stays dismissible for the session). Hidden in embed mode and
// in ?mock=1 runs so e2e tests are unaffected, unless ?hint=1 forces it.
import { useState } from "react";

const STORAGE_KEY = "spherex-blink.hint.dismissed";

function searchParams(): URLSearchParams {
  try {
    return new URLSearchParams(window.location.search);
  } catch {
    return new URLSearchParams();
  }
}

function wasDismissed(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

function initialVisible(): boolean {
  if (typeof window === "undefined") return false;
  const p = searchParams();
  const embed = p.get("embed") === "1" || p.get("embed") === "true";
  if (embed) return false;
  if (p.get("hint") === "1") return true;
  if (p.get("mock") === "1") return false;
  return !wasDismissed();
}

export function FirstRunHint() {
  const [visible, setVisible] = useState<boolean>(initialVisible);
  if (!visible) return null;
  const dismiss = () => {
    setVisible(false);
    try {
      window.localStorage.setItem(STORAGE_KEY, "1");
    } catch {
      // Storage blocked: hidden for this session only, which is fine.
    }
  };
  return (
    <p className="app-hint" role="note">
      <span className="app-hint-text">
        Press play, or click the image to see its spectrum. About explains
        the modes.
      </span>
      <button
        type="button"
        className="app-hint-close"
        aria-label="Dismiss hint"
        onClick={dismiss}
      >
        <svg
          viewBox="0 0 16 16"
          aria-hidden="true"
          focusable="false"
          className="app-hint-close-icon"
        >
          <path
            d="M4 4l8 8M12 4l-8 8"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
          />
        </svg>
      </button>
    </p>
  );
}

// Two-way sync between app state and the URL query string via
// history.replaceState. All parsing/validation lives in lib/urlState so the
// hook stays thin and the logic is unit-testable without a DOM.
import { useCallback, useEffect, useState } from "react";
import {
  parseUrlState,
  serializeUrlState,
  type UrlState,
} from "../lib/urlState";

function readCurrent(): UrlState {
  if (typeof window === "undefined") return parseUrlState("");
  return parseUrlState(window.location.search);
}

export function useUrlState(): [
  UrlState,
  (patch: Partial<UrlState>) => void,
] {
  const [state, setState] = useState<UrlState>(readCurrent);

  useEffect(() => {
    const onPop = () => setState(readCurrent());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const update = useCallback((patch: Partial<UrlState>) => {
    setState((prev) => {
      const next = { ...prev, ...patch };
      if (typeof window !== "undefined") {
        // ?mock=1 is not app state but must survive every URL rewrite so a
        // reload or shared link stays offline.
        const mock = new URLSearchParams(window.location.search).get("mock");
        let q = serializeUrlState(next);
        if (mock !== null) {
          const m = `mock=${encodeURIComponent(mock)}`;
          q = q === "" ? `?${m}` : `${q}&${m}`;
        }
        window.history.replaceState(null, "", q === "" ? window.location.pathname : q);
      }
      return next;
    });
  }, []);

  return [state, update];
}

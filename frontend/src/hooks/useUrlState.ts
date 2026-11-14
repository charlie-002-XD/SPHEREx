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
        const q = serializeUrlState(next);
        window.history.replaceState(null, "", q === "" ? window.location.pathname : q);
      }
      return next;
    });
  }, []);

  return [state, update];
}

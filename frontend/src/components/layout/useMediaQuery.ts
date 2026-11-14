// Media query hook safe to call without a DOM (SSR, vitest node env):
// falls back when matchMedia is unavailable and keeps a live subscription
// in the browser so breakpoint changes are picked up at runtime.
import { useEffect, useState } from "react";

export function useMediaQuery(query: string, fallback = false): boolean {
  const [matches, setMatches] = useState<boolean>(() =>
    typeof matchMedia === "function" ? matchMedia(query).matches : fallback,
  );

  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const mql = matchMedia(query);
    const onChange = () => setMatches(mql.matches);
    onChange();
    if (typeof mql.addEventListener === "function") {
      mql.addEventListener("change", onChange);
      return () => mql.removeEventListener("change", onChange);
    }
    // Safari < 14
    mql.addListener(onChange);
    return () => mql.removeListener(onChange);
  }, [query]);

  return matches;
}

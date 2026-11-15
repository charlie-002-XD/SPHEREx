// Embed-mode attribution: a tiny non-intrusive line that credits the NASA
// SPHEREx data source and links back to the full app, keeping the credit
// alive when the viewer is embedded in an iframe.
import { useMemo } from "react";
import "./about.css";

export function EmbedCredit() {
  // Same view, embed flag removed: opening the credit lands on the full
  // app at the position the embed was showing.
  const href = useMemo(() => {
    try {
      const u = new URL(window.location.href);
      u.searchParams.delete("embed");
      const q = u.searchParams.toString();
      return `${u.pathname}${q === "" ? "" : `?${q}`}`;
    } catch {
      return "/";
    }
  }, []);

  return (
    <p className="embed-credit">
      <a href={href} target="_blank" rel="noopener noreferrer">
        SPHEREx Blink — NASA SPHEREx data via IRSA
      </a>
    </p>
  );
}

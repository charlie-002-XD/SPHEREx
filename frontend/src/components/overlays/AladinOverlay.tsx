// Aladin Lite v3 sky overlay (lazy, pinned CDN build) centered on the viewer
// position, with Gaia DR3 and AllWISE catalog layers queried via CDS/VizieR.
// The overlay is collapsible and never loads Aladin until first opened. Any
// failure renders a subdued fallback — the viewer keeps working (the export
// is wrapped in an error boundary).
import { useEffect, useMemo, useRef, useState } from "react";
import {
  ALADIN_ATTRIBUTION,
  CATALOGS,
  loadAladin,
  type AladinGlobal,
  type AladinInstance,
  type CatalogKey,
} from "./aladin";
import { OverlayErrorBoundary } from "./ErrorBoundary";
import "./overlays.css";

export interface AladinOverlayProps {
  raDeg: number;
  decDeg: number;
  fovDeg: number;
  // Background HiPS survey; AllWISE matches the infrared context.
  survey?: string;
  defaultOpen?: boolean;
}

type OverlayStatus = "idle" | "loading" | "ready" | "error";

function AladinOverlayInner(props: AladinOverlayProps) {
  const { raDeg, decDeg, fovDeg, survey = "P/allWISE/color" } = props;
  const [open, setOpen] = useState(props.defaultOpen ?? false);
  const [status, setStatus] = useState<OverlayStatus>("idle");
  const [catOn, setCatOn] = useState<Record<CatalogKey, boolean>>({
    gaia: true,
    allwise: true,
  });
  const hostRef = useRef<HTMLDivElement | null>(null);
  const aGlobal = useRef<AladinGlobal | null>(null);
  const aladin = useRef<AladinInstance | null>(null);

  // Lazy init on first open. The effect keys on `open` only: it must not
  // depend on `status`, because the setStatus("loading") it performs would
  // re-run the effect and flip `disposed`, invalidating the only handlers
  // that can reach "ready"/"error". loadAladin() dedupes the script load
  // and the close path resets status + instance explicitly, so no further
  // guard is needed (a StrictMode remount simply attaches a second
  // handler to the same pending load).
  useEffect(() => {
    if (!open) return;
    let disposed = false;
    setStatus("loading");
    loadAladin()
      .then((A) => {
        if (disposed) return;
        const host = hostRef.current;
        if (host === null) {
          setStatus("error");
          return;
        }
        aGlobal.current = A;
        aladin.current = A.aladin(host, {
          target: `${raDeg} ${decDeg}`,
          cooFrame: "ICRS",
          fov: Math.max(fovDeg * 4, 0.02),
          survey,
          projection: "SIN",
          showReticle: false,
          showFrame: true,
          showZoomControl: true,
          showFullscreenControl: false,
          showGotoControl: false,
          showLayersControl: false,
          showProjectionControl: false,
          showCooGridControl: false,
          showSimbadPointerControl: false,
          showShareControl: false,
          showContextMenu: false,
        });
        setStatus("ready");
      })
      .catch(() => {
        if (!disposed) setStatus("error");
      });
    return () => {
      disposed = true;
    };
    // Init once per open; ra/dec/fov sync happens in the effect below.
  }, [open]);

  // Keep the view centered on the viewer's position/FOV.
  useEffect(() => {
    const a = aladin.current;
    if (a === null || status !== "ready") return;
    a.gotoRaDec(raDeg, decDeg);
    a.setFov(Math.max(fovDeg * 4, 0.02));
  }, [raDeg, decDeg, fovDeg, status]);

  // Catalog cone center/radius, quantized so sub-arcsec moves don't re-query
  // VizieR on every render.
  const catCone = useMemo(
    () => ({
      ra: Number(raDeg.toFixed(4)),
      dec: Number(decDeg.toFixed(4)),
      radius: Number(
        Math.max((fovDeg * Math.SQRT2) / 2, 0.005).toFixed(4),
      ),
    }),
    [raDeg, decDeg, fovDeg],
  );

  useEffect(() => {
    const a = aladin.current;
    const A = aGlobal.current;
    if (a === null || A === null || status !== "ready") return;
    a.removeLayers();
    for (const def of CATALOGS) {
      if (!catOn[def.key]) continue;
      try {
        a.addCatalog(
          A.catalogFromVizieR(
            def.vizierId,
            { ra: catCone.ra, dec: catCone.dec },
            catCone.radius,
            {
              name: def.label,
              color: def.color,
              shape: "circle",
              sourceSize: 8,
              onClick: "showPopup",
              limit: 500,
            },
          ),
        );
      } catch {
        // A single catalog failure must not block the rest of the overlay.
      }
    }
  }, [status, catOn, catCone]);

  return (
    <section className="ao" aria-label="Sky catalog overlay">
      <button
        type="button"
        className="ao-toggle"
        aria-expanded={open}
        onClick={() => {
          if (open) {
            // The host div unmounts on close; the Aladin instance bound to it
            // is dead, so the next open re-initializes (script stays cached).
            aladin.current = null;
            setStatus("idle");
          }
          setOpen((v) => !v);
        }}
      >
        <span aria-hidden="true">{open ? "▾" : "▸"}</span> Sky overlay (Aladin)
      </button>
      {open ? (
        <>
          <div className="ao-cats" role="group" aria-label="Catalog layers">
            {CATALOGS.map((c) => (
              <label key={c.key} className="ao-cat">
                <input
                  type="checkbox"
                  checked={catOn[c.key]}
                  onChange={(e) =>
                    setCatOn((s) => ({ ...s, [c.key]: e.currentTarget.checked }))
                  }
                />
                <span className="ao-swatch" style={{ background: c.color }} />
                {c.label}
              </label>
            ))}
          </div>
          {status === "error" ? (
            <p className="ao-error" role="alert">
              Sky overlay unavailable — could not load Aladin Lite from CDS.
            </p>
          ) : (
            <>
              {status === "loading" ? (
                <p className="ao-status" role="status">
                  Loading Aladin Lite…
                </p>
              ) : null}
              <div ref={hostRef} className="ao-view" />
            </>
          )}
          <footer className="ao-foot">
            {ALADIN_ATTRIBUTION}
            {" · "}
            {CATALOGS.map((c) => c.attribution).join(" · ")}
          </footer>
        </>
      ) : null}
    </section>
  );
}

export function AladinOverlay(props: AladinOverlayProps) {
  return (
    <OverlayErrorBoundary>
      <AladinOverlayInner {...props} />
    </OverlayErrorBoundary>
  );
}

// "About" entry in the header: a quiet button that opens a native <dialog>
// explaining what the tool does, how to read each mode, detectors, flags,
// keyboard shortcuts and the data credits. The native modal gives us the
// focus trap, Escape handling and focus restore; a small fallback path
// (setAttribute("open") + a manual Tab cycle) keeps the same behavior in
// environments without HTMLDialogElement.showModal (older browsers, jsdom).
import { useRef } from "react";
import "./about.css";

const FOCUSABLE =
  'a[href], button:not([disabled]), input, select, textarea, summary, [tabindex]:not([tabindex="-1"])';

function openDialogEl(d: HTMLDialogElement): void {
  if (typeof d.showModal === "function") {
    d.showModal();
  } else {
    d.setAttribute("open", "");
  }
}

function closeDialogEl(d: HTMLDialogElement): void {
  if (typeof d.close === "function") {
    d.close();
  } else {
    d.removeAttribute("open");
  }
}

export function AboutDialog() {
  const dialogRef = useRef<HTMLDialogElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  // The element to return focus to. The native modal already restores the
  // previously focused element, but setting it explicitly keeps the jsdom
  // fallback path identical.
  const restoreRef = useRef<HTMLElement | null>(null);

  const openDialog = () => {
    const d = dialogRef.current;
    if (d === null || d.open) return;
    restoreRef.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : triggerRef.current;
    openDialogEl(d);
    // Deterministic initial focus; native showModal may already have moved
    // it to the first focusable control.
    if (!d.contains(document.activeElement)) {
      d.querySelector<HTMLElement>(".about-close")?.focus();
    }
  };

  const closeDialog = () => {
    const d = dialogRef.current;
    if (d === null || !d.open) return;
    closeDialogEl(d);
    restoreRef.current?.focus();
    restoreRef.current = null;
  };

  const onDialogClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    // Backdrop clicks land on the dialog element itself; clicks on the
    // dialog box padding also target it, so compare against the box rect.
    const d = dialogRef.current;
    if (d === null || e.target !== d) return;
    const r = d.getBoundingClientRect();
    const inside =
      e.clientX >= r.left &&
      e.clientX <= r.right &&
      e.clientY >= r.top &&
      e.clientY <= r.bottom;
    if (!inside) closeDialog();
  };

  const onDialogKeyDown = (e: React.KeyboardEvent<HTMLDialogElement>) => {
    const d = dialogRef.current;
    if (d === null) return;
    if (e.key === "Escape") {
      // Native modals emit `cancel` (handled below); this covers the
      // fallback path where there is no native Escape handling.
      e.preventDefault();
      closeDialog();
      return;
    }
    if (e.key === "Tab" && typeof d.showModal !== "function") {
      // Manual focus trap for the no-showModal path only; native modals
      // trap Tab themselves.
      const items = Array.from(d.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && (active === first || !d.contains(active))) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && (active === last || !d.contains(active))) {
        e.preventDefault();
        first?.focus();
      }
    }
  };

  return (
    <>
      <button
        type="button"
        ref={triggerRef}
        className="about-btn"
        aria-haspopup="dialog"
        onClick={openDialog}
      >
        <svg
          className="about-btn-icon"
          viewBox="0 0 16 16"
          aria-hidden="true"
          focusable="false"
        >
          <circle
            cx="8"
            cy="8"
            r="5.75"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          />
          <path
            d="M8 7.4v3.1"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
          />
          <circle cx="8" cy="5" r="0.9" fill="currentColor" />
        </svg>
        <span>About</span>
      </button>
      <dialog
        ref={dialogRef}
        className="about-dialog"
        aria-labelledby="about-title"
        onCancel={closeDialog}
        onClick={onDialogClick}
        onKeyDown={onDialogKeyDown}
      >
        <div className="about-head">
          <h2 id="about-title" className="about-title">
            About SPHEREx Blink
          </h2>
          <button
            type="button"
            className="about-close"
            aria-label="Close"
            onClick={closeDialog}
          >
            <svg
              viewBox="0 0 16 16"
              aria-hidden="true"
              focusable="false"
              className="about-close-icon"
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
        </div>
        <div className="about-body">
          <p>
            Enter a sky position (RA/Dec) and SPHEREx Blink plays a
            time-lapse of every NASA SPHEREx image covering that patch of
            sky. Blink two epochs or difference them to spot objects that
            move or change brightness.
          </p>

          <h3 className="about-h">How to read it</h3>
          <ul className="about-list">
            <li>
              <strong>Blink A/B</strong> toggles two epochs: anything that
              moved or changed jumps out.
            </li>
            <li>
              <strong>Diff</strong> shows what changed: a moving object
              leaves a bright spot where it is and a dark spot where it
              was.
            </li>
            <li>
              <strong>Co-add</strong> sums frames over time to reveal
              sources too faint in a single exposure.
            </li>
            <li>
              <strong>Stack</strong> combines the frames of each detector
              array.
            </li>
            <li>
              <strong>Click the image</strong> to see the spectrum from
              every exposure covering that point.
            </li>
          </ul>

          <h3 className="about-h">Detectors</h3>
          <p>
            D1–D6 are SPHEREx&apos;s six detector arrays; each records a
            different infrared band, so frames of different wavelengths are
            never subtracted or summed — Diff and Co-add only combine frames
            from the same detector.
          </p>

          <h3 className="about-h">Flags</h3>
          <p>
            Flag chips mark data-quality bits of the shown frame. Each
            name&apos;s meaning comes from that file&apos;s FITS header;
            the legend is under &quot;Current frame&quot; in the controls
            panel.
          </p>

          <h3 className="about-h">Keyboard</h3>
          <p className="about-note">
            With the viewer focused:
          </p>
          <dl className="about-keys">
            <div className="about-key-row">
              <dt>
                <kbd>Space</kbd> or <kbd>K</kbd>
              </dt>
              <dd>Play / pause</dd>
            </div>
            <div className="about-key-row">
              <dt>
                <kbd>&larr;</kbd> / <kbd>&rarr;</kbd>
              </dt>
              <dd>Previous / next frame</dd>
            </div>
            <div className="about-key-row">
              <dt>
                <kbd>Home</kbd> / <kbd>End</kbd>
              </dt>
              <dd>First / last frame</dd>
            </div>
            <div className="about-key-row">
              <dt>
                <kbd>L</kbd>
              </dt>
              <dd>Loop on / off</dd>
            </div>
            <div className="about-key-row">
              <dt>
                <kbd>B</kbd>
              </dt>
              <dd>Blink A/B comparator (not in embeds)</dd>
            </div>
          </dl>

          <h3 className="about-h">Data and credits</h3>
          <p>
            Images: NASA SPHEREx via the{" "}
            <a href="https://irsa.ipac.caltech.edu/Missions/spherex.html">
              NASA/IPAC Infrared Science Archive (IRSA)
            </a>
            . Catalog overlays: Gaia DR3 (ESA) and AllWISE (NASA/WISE) via
            CDS VizieR. Overlay viewer:{" "}
            <a href="https://aladin.cds.unistra.fr/AladinLite/">
              Aladin Lite (CDS)
            </a>
            .
          </p>
        </div>
        <div className="about-foot">
          {/* TODO(team): repository link once the remote is public. */}
          <p>Open source, Apache-2.0</p>
          <p>Built for the NASA Space Apps Challenge 2026</p>
        </div>
      </dialog>
    </>
  );
}

# Demo video script — 30 seconds

Hard limit: **≤ 30 s** (over → ineligible for Global Judging). English
narration or English subtitles if spoken in another language. Show the
project running — real UI, real data. Pace: identity up front, product
working in the middle, data credit and URL at the end.

Narration word target: ~75 words total (≈150 wpm feels relaxed at 30 s).
On-screen text captions carry the details; the voiceover stays minimal.

| Time | On screen | Narration |
|---|---|---|
| 0:00–0:04 | Title card: **SPHEREx Blink**, `TODO(team): team name + member names`, challenge "Planet X and SPHEREx" | "SPHEREx Blink — a time-lapse window into NASA's newest sky survey." |
| 0:04–0:09 | Quick montage: FITS file / archive table → the app's clean search box. Caption: "102 bands. Every six months. Locked in FITS." | "SPHEREx maps the whole sky in a hundred bands — but only experts could look at it." |
| 0:09–0:17 | Live capture: type/paste WISE 0855 coordinates → frames load → time-lapse plays; blink toggles; the source visibly shifts. Caption: `TODO(measured): "<N> epochs · moved <X>″"` | "Type coordinates — every epoch plays as a time-lapse. Watch WISE 0855 move." |
| 0:17–0:24 | Click a pixel → spectrum panel opens with band markers; flag badges appear; diff mode flashes a residual. | "Click for its spectrum — flags and all — or subtract epochs to find what changed." |
| 0:24–0:30 | Captions: "Live NASA/IRSA data · Gaia DR3 + AllWISE overlays · shareable URLs" then final card with `TODO(team): <deployed URL>` and "Apache-2.0". | "Real IRSA data, one shareable link at a time. Try it — the URL is on screen." |

## Recording checklist

- [ ] Record on the deployed site, not localhost; preload a warm target so
      the fetch doesn't eat the clock (`?mock=1` is a fallback, never the
      main take — judges should see real data).
- [ ] 1080p minimum, app fullscreen; hide bookmarks bar and cursor trails if
      distracting.
- [ ] No people on camera (simplifies the under-18 likeness rule entirely).
- [ ] No NASA logos or mission identifiers anywhere in the frame.
- [ ] Verify the public video link in a private/incognito window.
- [ ] Keep a `TODO(measured)` screenshot of the final video length shown in
      the editor (must read ≤ 0:30).

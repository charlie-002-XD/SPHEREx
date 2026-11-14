# Decision log (append-only; decision maker: Claude as PM, approved by the user)

## D-001 (2026-11-14) Product: SPHEREx Blink
Public web tool: RA/Dec + FOV -> time-lapse of SPHEREx images, blink/difference/co-add/stack modes, click-for-spectrum, flag badges, URL state for embedding, five official targets as presets. Why: it is the challenge requirement plus the optional features with the best value for judges. See `docs/problem-solution.md`.

## D-002 (2026-11-14) Stack
Rust backend (axum) mandatory. Frontend Vite + React + TypeScript strict, static build served by the Rust binary (one deploy, one URL, no CORS issues). Why: no SSR needed, fastest to build and embed. Supersedes the Drive plan's Next.js 15.

## D-003 (2026-11-14) Data strategy: hybrid
Live IRSA queries with a byte-capped cache plus pre-warm of the five targets. Collections and endpoints are config. Why: demo must not depend on IRSA latency; new releases need no code change.

## D-004 (2026-11-14) Haiku (app.haiku.ac) is study-only
It is a flashcard app, not an orchestrator or UI generator. Collection created for team study. Devin builds the UI.

## D-005 (2026-11-14) IRSA concurrency default 3
Third-party report of HTTP 503 at 4 concurrent cutout requests (unverified). Configurable `IRSA_MAX_CONCURRENCY`, retry with backoff. Supersedes the Drive plan's 16.

## D-006 (2026-11-14) Schedule target
Submit by Sun 2026-11-15 18:00 (deadline 23:59 local). MVP order in `docs/backlog.md`; stretch features only when MVP gates are green.

## D-007 (2026-11-14) Wave 1 dispatched to Devin
b1-validation, b3-fits-parser, f1-viewer in parallel (disjoint scopes). Merge only after `review.sh` + cargo/tsc gates pass and Claude reviews the diff.

## D-008 (2026-11-14) Vault and skills
All agents read `docs/vault/` first and write `docs/vault/handoffs/<task>.md` last. Plugin skills: `context-vault`, `wave-orchestration`.

## D-009 (2026-11-14) Devin first, Claude as fallback
All implementation tasks go to Devin. If Devin is unavailable (quota/credits exhausted, auth or service errors, run exits non-zero without commits), Claude implements the task itself in the same worktree/branch and scope, then hands control back to Devin when it recovers. Same gates and handoff apply. Source: user instruction 2026-11-14.

## D-010 (2026-11-14) Claude is PM/PO only; never implements
Supersedes the Claude-implements half of D-009. Claude decides, dispatches, reviews, merges and updates the vault. All code is written by agents: Devin first; if Devin is unavailable, spawn a general-purpose subagent (Agent tool) on the same worktree, branch, scope and gates. Source: user instruction 2026-11-14.

## D-011 (2026-11-14) UI must be responsive and minimalist
Mobile-first responsive layout (375 px to desktop and iframe embeds), minimalist visual design: the image is the hero, few controls visible, secondary controls (stretch, colormap, mode, overlays, metadata) in a collapsible panel or bottom sheet on mobile and a slim side panel on desktop. Neutral dark palette with one accent, system font stack, CSS variables for tokens, no UI framework or heavy dependency, touch targets >= 44 px, prefers-reduced-motion and prefers-color-scheme respected, WCAG AA contrast. Applies to f1, f2, f4 and is tested in f3. Source: user instruction 2026-11-14.

## D-012 (2026-11-14) b3 merged: custom pure-Rust FITS reader
fitrs 0.5 cannot read BINTABLE or tile compression and panics on bad input, so b3 wrote a bounds-checked reader (RICE_1, WCS TAN+SIP, WCS-WAVE). fitrs and byteorder become unused; b5 removes them. GZIP-compressed tiles unsupported (not delivered by cutouts).

## D-013 (2026-11-14) f2 merged; CDN runtime deps to be bundled
f2 loads uPlot 1.6.32 (SRI) from jsDelivr at runtime and Aladin Lite from a CDN. Integration should bundle uPlot via npm (judges' networks may block CDNs; FortiGuard); Aladin Lite stays CDN-loaded but behind an ErrorBoundary so the viewer works without it. Components are not wired into App.tsx yet: integration step wires SpectrumPanel, FrameMetadata and AladinOverlay.

## D-014 (2026-11-14) b2 merged; API defaults for responsiveness
Live IRSA confirmed: 503 'Server busy' is real (retries mandatory) and 422 'Arrays do not overlap' occurs when SIA footprint covers a point but the array does not (cutout skipped, partial results). IRSA can take minutes for 60 frames, so contract max_frames default is now 24 (max 60) and the API has a total deadline (REQUEST_DEADLINE_SECS=40) returning partial frames with truncated=true. Output images capped at OUTPUT_NPIX=192.

## D-015 (2026-11-14) Remote configured, nothing is pushed
origin = https://github.com/Luis000923/SPHEREx.git, added as a local git config only. No push, fetch or any network git operation until the user explicitly asks. Devin tasks already forbid pushing. CI workflow stays untriggered while nothing is pushed. Source: user instruction 2026-11-14.

## D-016 (2026-11-14) Visual review of mock UI (PM)
Checked ?mock=1 in Chrome at 375, 768 and 1440 px: no horizontal overflow, image is the hero, controls collapse into a bottom sheet on mobile and a side panel from 640 px, all touch targets >= 44 px at 375 px. Minor: segmented mode/detector buttons are 40 px tall at >= 640 px (below the 44 px rule); fix in polish if time allows. Mobile order is image, transport, search, sheet handle (acceptable, thumb-friendly). Not yet reviewed with real data or the spectrum panel open.

## D-017 (2026-11-14) Opus 5.5 agent for complex tasks
New agent spherex-complex-engineer (model claude-opus-5-5, plugin agents/ and repo .claude/agents/). Delegation criteria and commands are in the wave-orchestration skill. Devin stays the default for routine tasks. Branches opus/<id>, worktree .devin-worktrees/<id>-opus, commit trailer 'Generated-by: Claude Opus 5.5 (SI-assisted)'. First delegations: real-data integration and any gate that fails twice. Source: user instruction 2026-11-14.

## D-018 (2026-11-14) integration-live delegated to Opus
Reason per policy: spans backend and frontend, needs real IRSA data and numeric verification of WCS alignment and proper-motion displacement. Branch opus/integration-live.

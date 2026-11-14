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

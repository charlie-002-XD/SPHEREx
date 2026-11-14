# Handoff: submission-docs

Status: **done**. Branch `devin/submission-docs`. All deliverables drafted;
everything that needs real measurements or team data is marked
`TODO(measured):` / `TODO(team):`.

## Files

- `README.md` — rewritten: what it is, screenshot placeholders, feature
  list, quick start (backend / frontend dev / production / Docker pointing
  to `docs/DEPLOY.md`), URL-parameter table (`ra, dec, fov, mode, stretch,
  cmap, fps, frame, embed, mock`), API summary linking
  `shared/openapi-contract.json`, architecture diagram, data-release
  instructions (`IRSA_COLLECTIONS`, `IRSA_BASE_URL`), credits, limitations.
- `docs/submission/project-page.md` — one section per submission form
  field, in order, with `--- copy start/end ---` paste blocks: Project
  Name (+tagline), High-Level Summary (**101 words**, limit 120), Demo and
  Final Project link placeholders, Detailed Description (what/how/
  benefits/hope/tools), Use of SI (Claude Code orchestrated/reviewed,
  Devin implemented in isolated branches, Opus 5.5 delegated
  integration-live, humans set the idea; no SI media, no NASA branding),
  NASA Data (IRSA SPHEREx via SIA2 + cutouts, AllWISE & Gaia DR3 via CDS,
  IRSA tutorials), Partner Data & Other References (ESA Gaia DR3, CDS
  services, scientific papers, WiseView, and the full generated dependency
  list — 565 lines, every crate/npm package with version + license).
- `docs/submission/list-deps.sh` — generator for the dependency list
  (cargo metadata + package-lock.json licenses). Re-run if lockfiles
  change.
- `docs/submission/demo-slides.md` — 7-slide outline incl. title slide
  (team/member placeholders, challenge, per-slide screenshot placeholders,
  speaker notes <= 40 words each, NASA-data slide).
- `docs/submission/demo-video-script.md` — 30 s script, timestamped table
  (0:00-0:30) following guide pacing: identity first, product working,
  data credit + URL last; recording checklist.
- `docs/submission/checklist.md` — pre-submit checklist: all fields, both
  checkboxes, incognito link tests, FortiGuard classification, demo limits,
  under-18 likeness rule, SI field, English-only, deadline.
- `SI_USAGE.md` — updated: Claude Code (coordination, no app code), Devin
  (implementation + docs), Opus 5.5 (integration-live delegation).
- `docs/REFERENCES.md` — software section now carries the full generated
  dependency list; runtime-loaded Aladin Lite called out separately.

## Decisions / deviations

- **`scripts/list-deps.sh` was out of scope** (`scripts/` is not in the
  allowed paths), so the generator lives at `docs/submission/list-deps.sh`
  instead. Same function, documented in both files that cite it.
- Gaia DR3 is listed in the NASA Data field (per the task's field
  assignment) **and** under Partner Data, since it is an ESA mission —
  judges see it wherever they look; both statements are accurate.
- High-Level Summary is exactly 101 words (counted, under the 120 limit).
- No test-count or latency claims in the docs — unverified numbers stay as
  `TODO(measured):` for `integration-live` (WISE 0855 displacement, load
  times, screenshot files).

## Open issues

- `TODO(team):` placeholders remain where only humans can fill them (team
  name, member names, deployed URL, demo link).
- `TODO(measured):` placeholders remain for screenshots and the
  integration-live numbers — the orchestrator fills them.
- `docs/submission/screenshots/` directory does not exist yet; it is
  referenced by placeholder comments only.

## Verify

- `docs/submission/list-deps.sh` — ran clean, 565 deps lines generated.
- Relative-link check (python script scanning all `*.md`): **all OK**.
- TODO-marker check: every `TODO(` is `TODO(measured):` or `TODO(team):`.
- `wc -w` on the summary copy block: 101 words.
- No changes outside the allowed scope; no code touched.

# Use of Super Intelligence (SI/AI) tools

Rule: SI may accelerate development but must not be used to come up with the idea of how to solve the challenge. All SI use is disclosed here.

TODO(team): state accurately who defined the product idea. In this project Claude proposed the problem/solution definition (`docs/problem-solution.md`, decision D-001) from the challenge text and the user approved it; see `docs/vault/open-questions.md`. Do not submit this wording as is.

| Area | Tool | How it was used | Human review |
|---|---|---|---|
| Coordination, architecture review, OpenAPI contract, task breakdown, vault, scripts, project notes | Claude (Claude Code) | Proposed the problem/solution definition and plan from the challenge text (approved by the user), drafted the API contract, backlog, helper scripts and reviews; merged approved work. Wrote no application feature code. | Team |
| Rust backend modules, TypeScript frontend components, Dockerfile/CI, tests, documentation (incl. submission drafts) | Devin | Implemented tasks in `docs/backlog.md` in isolated branches; commits carry `Generated-by: Devin (SI-assisted)` | Claude + team review before merge |
| Live-data integration and numeric verification (task `integration-live`) | Claude Opus 5.5 agent | Cross-cutting integration of backend and frontend with real data; commits carry `Generated-by: Claude Opus 5.5 (SI-assisted)` | Claude + team review before merge |
| Visual design pass (tokens, layout, CSS, layout tests) | Claude Opus 5.5 (agent spherex-frontend-designer) | Task `f6-visual-polish`: design tokens, docked spectrum layout, component CSS, layout and contrast tests from the team's UI rules; commits carry `Generated-by: Claude Opus 5.5 (SI-assisted)` | Claude + team review before merge |
| Compare A/B sky viewer (branch `feat/frontend-sky-viewer`) | Devin | Ported the team's Planeta-X prototype UI (epoch slider, blink comparator, band selector, click-to-mark findings) into the TypeScript frontend; commits carry `Generated-by: Devin (SI-assisted)` | Team review before merge |

SI-generated: essentially all application code, tests, Dockerfile/CI and
documentation text. Not SI-generated: reported measurements and any
screenshots/recordings in the demo (real output of the running application).
TODO(team): authorship of the idea and product definition (see above).

No SI-generated images, audio or video are used. No NASA logos or branding
are generated.

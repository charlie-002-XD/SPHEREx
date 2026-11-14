# Use of Super Intelligence (SI/AI) tools

Rule: SI may accelerate development but the idea of how to solve the
challenge must be the team's. The human team chose the challenge, defined
the product (SPHEREx Blink), set the requirements and design rules, verified
results and made all scope decisions. All SI use is disclosed here.

| Area | Tool | How it was used | Human review |
|---|---|---|---|
<<<<<<< HEAD
| Coordination, architecture review, OpenAPI contract, task breakdown, project notes | Claude (Claude Code) | Drafted the API contract, backlog and reviews from the team's problem statement; merged approved work; did not write application code | Team |
| Rust backend modules, TypeScript frontend components, Dockerfile/CI, tests, documentation (incl. submission texts) | Devin | Implemented the tasks in `docs/backlog.md` in isolated branches; commits carry `Generated-by: Devin (SI-assisted)` | Claude + team review before merge |
| Live-data integration and numeric verification (one complex cross-cutting task) | Claude Opus 5.5 agent | Delegated `integration-live` under the same gates and review policy; commits carry `Generated-by: Claude Opus 5.5 (SI-assisted)` | Claude + team review before merge |
=======
| Planning, architecture review, contract, docs | Claude (Claude Code) | Drafted the API contract, task breakdown and reviews, from the team's problem definition | Team |
| Rust backend modules and TypeScript frontend components | Devin | Implemented tasks listed in `docs/backlog.md` in isolated branches; commits carry `Generated-by: Devin (SI-assisted)` | Claude + team review before merge |
| Visual design pass (tokens, layout, CSS, layout tests) | Claude Opus 5.5 (Claude Code, agent spherex-frontend-designer) | Task f6-visual-polish: design tokens, docked spectrum layout, component CSS and layout/contrast tests from the team's UI rules (D-011, D-016, D-019); commits carry `Generated-by: Claude Opus 5.5 (SI-assisted)` | Orchestrator + team review before merge |
>>>>>>> opus/f6-visual-polish

SI-generated: essentially all application code, tests, Dockerfile/CI and
documentation text. Not SI-generated: the idea of how to solve the
challenge, the product definition, reported measurements, and any
screenshots/recordings in the demo (real output of the running application).

No SI-generated images, audio or video are used. No NASA logos or branding
are generated.

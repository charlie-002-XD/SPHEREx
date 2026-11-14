# Use of Super Intelligence (SI/AI) tools

Rule: SI may accelerate development but the idea of how to solve the challenge must be the team's. Disclose all use.

| Area | Tool | How it was used | Human review |
|---|---|---|---|
| Planning, architecture review, contract, docs | Claude (Claude Code) | Drafted the API contract, task breakdown and reviews, from the team's problem definition | Team |
| Rust backend modules and TypeScript frontend components | Devin | Implemented tasks listed in `docs/backlog.md` in isolated branches; commits carry `Generated-by: Devin (SI-assisted)` | Claude + team review before merge |
| Visual design pass (tokens, layout, CSS, layout tests) | Claude Opus 5.5 (Claude Code, agent spherex-frontend-designer) | Task f6-visual-polish: design tokens, docked spectrum layout, component CSS and layout/contrast tests from the team's UI rules (D-011, D-016, D-019); commits carry `Generated-by: Claude Opus 5.5 (SI-assisted)` | Orchestrator + team review before merge |

No SI-generated images, audio or video are used. No NASA logos or branding are generated.

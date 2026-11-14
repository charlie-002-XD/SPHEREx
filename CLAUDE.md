# SPHEREx Sky Time-Lapse (NASA Space Apps 2026, "Planet X and SPHEREx")

Public web tool: enter RA/Dec, see a time-lapse of SPHEREx images of that patch. Rust backend (axum) + Vite/React/TypeScript frontend served by the backend.

## Read first
- `shared/openapi-contract.json`: single source of truth for the API. Never change it without the orchestrator; if it looks wrong, stop and report.
- Verified data-access facts: `/home/luis/Documentos/SPHEREx/spherex-devin/skills/spherex-hackathon/references/verified-data-access.md`.
- Rules/deadlines: `.../references/rules.md` (same folder). Submission closes Sun 2026-11-15 23:59 local.

## Rules for every agent
1. All code, comments, commits, docs, UI text in English.
2. Stay inside the allowed scope of your task. Do not edit other paths.
3. Rust: `#![forbid(unsafe_code)]`; no `.unwrap()`/`.expect()` outside tests and `main` bootstrap; errors as typed `AppError` (thiserror). Gate: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`.
4. TypeScript: `strict: true`, no `any`, zod at every boundary. Gate: `npx tsc --noEmit && npx vitest run && npx eslint .`.
5. Validate all inputs: `0 <= RA < 360`, `-90 <= Dec <= 90`, `0.01 <= FOV <= 0.5` degrees. Never echo raw user input in errors.
6. IRSA is a shared public service: default concurrency 3, retry with backoff on 503/429, cache aggressively.
7. Never hardcode FITS flag bit meanings, band counts (not always 102) or collection names: they come from headers/config.
8. No NASA logos or branding in anything generated. Cite every library and data source (see `docs/REFERENCES.md`).
9. Commit on your own branch, never push. End commit messages with `Generated-by: Devin (SI-assisted)` when Devin wrote them, and record SI-generated parts in `SI_USAGE.md`.
10. Unit tests use synthetic data; no test may call the real IRSA unless tagged live and opt-in.

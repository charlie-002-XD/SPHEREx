# Backlog (PM view). Status: todo / running / review / merged

| Id | Work | Owner | Depends | Status |
|---|---|---|---|---|
| p0 | Verify IRSA access, contract, CLAUDE.md, scaffolds | Claude | - | merged (this commit) |
| b1-validation | Coordinate parsing, validation, AppError | Devin | p0 | merged |
| b3-fits-parser | MEF parser, WCS, flag legend | Devin | p0 | merged |
| f1-viewer | WebGL viewer, controls, URL state | Devin | p0 | merged |
| b2-irsa-client | SIA2 + cutouts + cache | Devin | b1, b3 | merged |
| b4-science | Stack, diff, co-add, spectrum | Devin | b3 | merged |
| f2-spectrum | Spectrum panel, flags, Aladin | Devin | f1 | merged |
| b5-api-routes | Routes, config, static serving, limits | Devin | b1, b2, b4 | todo |
| f5-wire-spectrum | Wire spectrum/metadata/overlay, bundle uPlot, ?mock=1 | Devin | f2, f4 | merged |
| ops-docker-ci | Dockerfile, CI, DEPLOY.md | Devin | - | running |
| integration-live | Real-data end to end, numbers, screenshots (Opus agent) | Opus 5.5 | b5, f5 | blocked on b5 |
| f4-layout-design | Minimalist responsive app shell, tokens, embed mode | Devin | f1 | merged |
| f3-e2e-tests | Playwright | Devin | integration | todo |
| deploy | Dockerfile, CI, hosting, warm-up, FortiGuard | Claude | integration | todo |
| submission | README, SI_USAGE, REFERENCES, demo (<=30 s / <=7 slides), submit | all | deploy | todo |
| study | Team study collection in app.haiku.ac (content to confirm) | Claude | - | todo |

Hard stop: submit by Sun 2026-11-15 18:00 (deadline 23:59 local).

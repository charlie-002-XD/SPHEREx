# Open questions (unverified; owner in brackets)

- Unit of the cutout `size` parameter (degrees? arcmin?). Source: https://irsa.ipac.caltech.edu/ibe/cutouts.html [Claude, before b2]
- TAP endpoint URL for `spherex.plane` / `spherex.artifact` (SIA2 lags ~1 day; one third-party report says QR3 was TAP-only on 2026-09-18). [Claude, before b2]
- Does `fitrs` read the tile-compressed FLAGS HDU? If not, decompression strategy. [Devin b3, report in handoff]
- IRSA 503 under load: CONFIRMED live by b2 (concurrency limit itself still unmeasured; default 3 with retries works).
- Hosting target for the public URL and domain eligibility. [user]
- Cutout size unit: RESOLVED. size/center default to degrees; units px/arcsec/arcmin/deg/rad accepted; one value = square (https://irsa.ipac.caltech.edu/ibe/cutouts.html). Use size=<fov_deg>deg. [closed]
- Does fitrs read the tile-compressed FLAGS HDU? RESOLVED by b3: no; custom pure-Rust reader written. Real cutouts deliver FLAGS uncompressed (verified live 2026-11-14).
- SI field wording must be truthful about idea authorship. The draft says the team 'defined the product idea'; in this project Claude (PM role) proposed the problem/solution definition (D-001) from the challenge text and the user approved it. Rules say SI must not be used to come up with the solution idea. User must decide how to describe the human contribution accurately (or contribute/own the idea), and the draft must also say Opus 5.5 did integration-live AND the visual design pass, and that Claude Code wrote contract, scripts, vault and docs. [user, before submission]

# Open questions (unverified; owner in brackets)

- Unit of the cutout `size` parameter (degrees? arcmin?). Source: https://irsa.ipac.caltech.edu/ibe/cutouts.html [Claude, before b2]
- TAP endpoint URL for `spherex.plane` / `spherex.artifact` (SIA2 lags ~1 day; one third-party report says QR3 was TAP-only on 2026-09-18). [Claude, before b2]
- Does `fitrs` read the tile-compressed FLAGS HDU? If not, decompression strategy. [Devin b3, report in handoff]
- Is the IRSA 503-at-4-concurrent report true? [measure live once, record in facts.md]
- Hosting target for the public URL and domain eligibility. [user]
- Cutout size unit: RESOLVED. size/center default to degrees; units px/arcsec/arcmin/deg/rad accepted; one value = square (https://irsa.ipac.caltech.edu/ibe/cutouts.html). Use size=<fov_deg>deg. [closed]
- Does fitrs read the tile-compressed FLAGS HDU? RESOLVED by b3: no; custom pure-Rust reader written. Real cutouts deliver FLAGS uncompressed (verified live 2026-11-14).

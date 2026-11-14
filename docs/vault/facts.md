# Verified facts (with source and date)

Primary: `/home/luis/Documentos/SPHEREx/spherex-devin/skills/spherex-hackathon/references/verified-data-access.md` (IRSA endpoints, collections, FITS layout, limits) and `.../rules.md` (official rules, deadlines). Verified 2026-11-14 against spaceappschallenge.org and IRSA documentation.

Quick reference
- SIA2: `https://irsa.ipac.caltech.edu/SIA?COLLECTION=spherex_dr1|spherex_qr3(+_deep)&POS=circle ra dec radius_deg`.
- Cutout: `<access_url>?center=RA,Dec&size=<value>` (size units to confirm).
- FITS extensions: IMAGE (MJy/sr, zodi not subtracted), FLAGS (int32, legend in header MSKNnnnn/MSKDnnnn), VARIANCE, ZODI, EPSF, WCS-WAVE. Detector in header `DETECTOR` (1-6).
- Submission closes Sun 2026-11-15 23:59 local; demo <= 30 s video or <= 7 slides; English; cite all sources; SI use disclosed.

## Verified live by b3 (2026-11-14, spherex_qr3 detector D1 cutout)
- Cutout request `?center=337.8783221,-5.1891588&size=0.05` returned a 29x29 px cutout (~0.05 deg across): `size` is degrees by default.
- FLAGS arrives uncompressed in cutouts (BITPIX=32 IMAGE); legend keys MSKN0000='TRANSIENT' ... MSKN0028='HALO'.
- WCS-WAVE: 1 row, X/Y = 9J control coords 1..2040 (full frame), VALUES = 162E = 81 (wavelength, bandwidth) pairs, x fastest. Wavelength at cutout centre 1.108 um vs SIA em_max 1.117 um.
- Sky WCS is TAN + SIP; round trip pixel<->RA/Dec exact to ~1e-12 px.

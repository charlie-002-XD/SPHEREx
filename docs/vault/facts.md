# Verified facts (with source and date)

Primary: `/home/luis/Documentos/SPHEREx/spherex-devin/skills/spherex-hackathon/references/verified-data-access.md` (IRSA endpoints, collections, FITS layout, limits) and `.../rules.md` (official rules, deadlines). Verified 2026-11-14 against spaceappschallenge.org and IRSA documentation.

Quick reference
- SIA2: `https://irsa.ipac.caltech.edu/SIA?COLLECTION=spherex_dr1|spherex_qr3(+_deep)&POS=circle ra dec radius_deg`.
- Cutout: `<access_url>?center=RA,Dec&size=<value>` (size units to confirm).
- FITS extensions: IMAGE (MJy/sr, zodi not subtracted), FLAGS (int32, legend in header MSKNnnnn/MSKDnnnn), VARIANCE, ZODI, EPSF, WCS-WAVE. Detector in header `DETECTOR` (1-6).
- Submission closes Sun 2026-11-15 23:59 local; demo <= 30 s video or <= 7 slides; English; cite all sources; SI use disclosed.

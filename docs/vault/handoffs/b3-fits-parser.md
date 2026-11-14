# Handoff: b3-fits-parser

Status: **done**. All gates green. Validated against a real IRSA QR3 cutout.

## Files

- `backend/src/fits/mod.rs` — public API: `parse_frame(&[u8]) -> Result<Frame, FitsError>`,
  `Frame`, `FrameMeta`, `FlagLegend`/`FlagBit`, `flags_to_names`.
- `backend/src/fits/error.rs` — `FitsError` (thiserror).
- `backend/src/fits/header.rs` — 80-byte card parser (strings, bools, ints, reals with
  `D` exponents, `HIERARCH` long keys).
- `backend/src/fits/hdu.rs` — HDU iterator over `&[u8]`; data length
  `GCOUNT*(PCOUNT + |BITPIX|/8 * prod NAXISi)`, bounds-checked, 2880-block padding.
- `backend/src/fits/table.rs` — BINTABLE reader: fixed columns plus `P`/`Q`
  variable-length arrays + heap (`THEAP` honoured).
- `backend/src/fits/rice.rs` — RICE_1 decoder (port of cfitsio `fits_rdecomp`,
  8/16/32-bit pixels; fsbits/fsmax per width).
- `backend/src/fits/compimg.rs` — tile-compressed image decoder (ZIMAGE):
  `RICE_1`, `NOCOMPRESS`, `UNCOMPRESSED_DATA` fallback column, per-tile
  `ZSCALE`/`ZZERO`. `GZIP_*`/`HCOMPRESS`/`PLIO` -> `UnsupportedCompression`.
- `backend/src/fits/wcs.rs` — `SkyWcs`: TAN gnomonic + SIP forward
  (`A_*`/`B_*`); inverse via `AP_*`/`BP_*` as seed + fixed-point refinement.
  CD from `CDi_j` or `CDELT_i * PC_i_j`.
- `backend/src/fits/wave.rs` — `WaveMap` (`wavelength_at`, `bandwidth_at`),
  WCS-TAB grid (`X`,`Y`,`VALUES` columns), `PVi_3W` selects which VALUES
  element is wave/band, `CRPIXnW/CRVALnW/CDELTnW` pixel->table transform.
- `backend/tests/fits_tests.rs` — 13 tests.
- `backend/tests/fixtures/mod.rs` — synthetic MEF writer + RICE_1 encoder.

## What was built

`parse_frame(bytes)` looks extensions up by `EXTNAME` (case-insensitive,
trimmed): `IMAGE`, `FLAGS`, `VARIANCE`, `ZODI`, `WCS-WAVE` required,
`EPSF` optional (`header.has_epsf`). Missing required extension ->
`FitsError::MissingExtension { name, found }` with the sorted EXTNAME list.
`Frame { image, variance, zodi: Array2<f32>, flags: Array2<i32>, wcs, wavelength, header }`.
`FrameMeta` carries `detector` (int or string form), `mjd_obs/beg/avg/end`,
`date_obs/beg/end`, `exposure_s`, `telapse_s`, `unit`, `flag_legend`, `has_epsf`.

Flag legend comes only from `MSKNnnnn`/`MSKDnnnn` of the FLAGS header;
`flags_to_names(value, legend)` emits names in bit order, `BIT{n}` for
bits missing from the legend.

Conventions: 1-based FITS pixel coordinates everywhere public; arrays are
`Array2[y][x]`. `wavelength_at`/`bandwidth_at` clamp at grid edges.

## Verified against a real IRSA cutout (spherex_qr3, detector D1)

Fetched live 2026-11-14 (`ibe/data/spherex/qr3/level2/...?center=337.8783221,-5.1891588&size=0.05`):

- FLAGS arrives **uncompressed** in cutouts (BITPIX=32 IMAGE). Compressed
  path kept for robustness but only exercised on synthetic fixtures.
- MSKN0000='TRANSIENT' ... MSKN0028='HALO' legend confirmed.
- WCS-WAVE: 1 row, `X`/`Y` = '9J' (control coords 1..2040, full-frame),
  `VALUES`='162E' = 81 pairs (wavelength, bandwidth), x fastest.
- `wavelength_at` at cutout centre gave 1.108 um (SIA `em_max`=1.117 um).
- `pixel_to_radec(centre)` = (337.879, -5.189) vs requested centre
  (337.878, -5.189) — sub-pixel offset as expected for a 29x29 cutout.
- radec_to_pixel round-trip exact to ~1e-12 px after SIP refinement.

## Decisions / deviations

- **fitrs 0.5 not used.** It reads only from `File`, has no BINTABLE or
  tile-compression support, and `expect()`s on malformed input (violates
  "never panics"). A bounds-checked pure-Rust reader was written instead
  (task allows this: "implement the needed Rice/GZIP decompression").
  `fitrs` and `byteorder` are now unused dependencies in Cargo.toml —
  integration may drop them (Cargo.toml is outside my scope).
- **`AppError` does not exist yet** (it is b1's deliverable). The module
  returns `FitsError`; merge glue for b5/b1: map `FitsError` into
  `AppError::Fits`, e.g. `impl From<FitsError> for AppError { ... AppError::Fits(e.to_string()) }`.
- **RICE_1 only** for compressed tiles. GZIP_1/GZIP_2 need a DEFLATE
  implementation (no flate2 in scope; Cargo.toml off-limits). Real cutouts
  deliver FLAGS uncompressed; if a compressed SPHEREx product shows GZIP,
  add `flate2` or gate cfitsio behind a feature flag.
- **SIP inverse:** `AP_*`/`BP_*` polynomials are used as the seed of a
  fixed-point inversion of the forward SIP (their fit residual alone left
  ~0.0035 px error on the real file; iteration is exact).
- WCS-WAVE layout assumption: `VALUES[i]` pairs indexed `iy*nx + ix`
  (x fastest), consistent with the real file (strong lambda gradient along
  the outer index, matching detector physics).

## Open issues

- Compressed-FLAGS path validated only against synthetic fixtures (round
  trip through an encoder ported from cfitsio `fits_rcomp`), not a real
  compressed SPHEREx file — none are produced by the cutout service.
- `EPSF` detected but not decoded (task marks it optional).
- `docs/REFERENCES.md` (outside scope) should cite: CFITSIO `ricecomp.c`
  (Rice algorithm port, NASA/GSFC), FITS Tiled Image Compression
  Convention (fits.gsfc.nasa.gov/registry/tilecompression.html), FITS WCS
  Papers II/III (TAN projection, WCS-TAB), SIP convention (Shupe et al.).
- `SI_USAGE.md` (outside scope): this module is Devin-written, covered by
  the generic row already present.
- `size` cutout units still unconfirmed (open question for b2; a `size=0.05`
  request returned a 29x29 px cutout ~0.05 deg across — looks like degrees).

## Verify

```
cd backend
cargo fmt --check          # ok
cargo clippy --all-targets -- -D warnings   # ok
cargo test                 # 13/13 pass
SPHEREX_FITS_REAL=/path/to/cutout.fits cargo test --test fits_tests real_file  # opt-in, passes
```

Answers the open question in `open-questions.md`: *"Does fitrs read the
tile-compressed FLAGS HDU?"* — **No** (no BINTABLE/ZIMAGE support), so a
custom reader was implemented; note cutouts deliver FLAGS uncompressed anyway.

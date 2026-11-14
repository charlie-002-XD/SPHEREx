# Handoff: b4-science

Status: **done**. All gates green (`fmt`, `clippy -D warnings`, 36 tests).

## Files

- `backend/src/science/mod.rs` — module docs + re-exports.
- `backend/src/science/error.rs` — `ScienceError` (thiserror):
  `EmptyInput`, `ShapeMismatch`, `GridMismatch`, `WavelengthMismatch`
  (names both wavelengths + tolerance), `UnknownWavelength`,
  `InvalidDetector`, `InvalidAperture`, `InvalidGrid`, `Wcs(#[from] FitsError)`.
- `backend/src/science/grid.rs` — `LinearTanWcs` (TAN, no SIP; public
  `crpix`/`crval`/`cd` matching the contract `Wcs` schema) and
  `OutputGrid` (`centered(ra, dec, fov, npix)` north-up/east-left,
  `from_frame`, `identity` for tests).
- `backend/src/science/image.rs` — `ScienceImage` (image/variance/flags/
  n_contrib `Array2[y][x]` + grid, detector, representative wavelength,
  mjd, `zodi_subtracted`), `StackOptions`, `stack_images` (inverse-variance,
  `w = 1/variance`; NaN/zero/negative variance or `flags &
  bad_flags_mask != 0` -> weight 0), `stack_detectors`
  (`&BTreeMap<u8, Vec<&ScienceImage>>` -> `StackResult { per_detector,
  combined }`), `group_frames` (detector x wavelength clusters, sorted by
  epoch), `mask_from_names` (legend-name -> bitmask helper),
  `central_wavelength`, `frame_epoch`, `DEFAULT_WAVELENGTH_TOLERANCE_UM`
  = 0.05.
- `backend/src/science/reproject.rs` — `reproject(frame, grid, opts)`:
  per output pixel `grid.wcs.pixel_to_radec` -> `frame.wcs.radec_to_pixel`;
  `Interp::Nearest`/`Bilinear` (bilinear: coefficients renormalized over
  finite neighbours; variance propagated as `sum(c_i^2 * var_i)`; flags
  always nearest); `subtract_zodi` (IMAGE - ZODI) default **on**.
- `backend/src/science/epoch.rs` — `subtract_epochs(later, earlier,
  tol)` (grid + wavelength-tolerance checks, var = va + vb),
  `coadd_epochs` (same checks, then inverse-variance mean),
  `temporal_stats` (`n`, `mean`, `std`, `min`, `max`, `peak_to_peak`,
  `last_minus_first`) — the moving-object helper for blink/diff modes.
- `backend/src/science/spectrum.rs` — `extract_spectrum(frames, ra,
  dec, aperture_px, subtract_zodi)` -> `Vec<SpectrumPoint>` sorted by
  wavelength: `value` = aperture **mean** (MJy/sr, per contract),
  `sigma = sqrt(sum var)/N`, `flags` = union of legend names over the
  aperture, wavelength from WCS-WAVE at the exact position (never a
  fixed band count). Uncovered / all-NaN frames skipped; empty vec =
  "no observations" (b5 maps to 404).
- `backend/src/science/encode.rs` — `image_to_le_bytes` +
  `image_to_b64` (row-major f32 LE, contract `data_b64`).
- `backend/tests/science_tests.rs` — 15 tests, synthetic data only.

## What was built / pipeline for b5

1. `group_frames(&[&Frame], tolerance_um)` -> `Vec<FrameGroup>`
   (detector 1-6 x wavelength cluster, members sorted by MJD).
2. `reproject(&Frame, &OutputGrid, &ReprojectOptions)` -> `ScienceImage`
   on the requested grid (e.g. `OutputGrid::centered(ra, dec, fov, npix)`).
3. `stack_images` / `stack_detectors`, `subtract_epochs` /
   `coadd_epochs`, `extract_spectrum`, `temporal_stats`.
4. `image_to_b64` for JSON output.

## Decisions / deviations

- **`stack_detectors` input is `&BTreeMap<u8, Vec<&ScienceImage>>`**
  (already-reprojected, same-grid images), not raw `Frame`s — the task
  requires a common pixel grid before epoch arithmetic, so reprojection
  is a separate explicit step. Detector keys outside 1..=6 ->
  `InvalidDetector`. `combined` stacks all arrays: a mixed-wavelength
  deep image (SPHEREx bands do not overlap), documented as such.
- **`SkyWcs` cannot be constructed outside `crate::fits`** (SIP fields
  private, no public constructor), so output grids use `LinearTanWcs`
  (same TAN math, no SIP). `OutputGrid::from_frame` copies the linear
  part of a frame WCS. No `fits/` edits (out of scope).
- **Bad-flags policy is caller-supplied**: bit meanings may not be
  hardcoded (rule 7), so `StackOptions::bad_flags_mask` is a parameter;
  default `-1` (every flagged pixel rejected) — conservative; b5 can
  build a narrower mask by legend names via `mask_from_names`
  (e.g. keep `SOURCE`/`TRANSIENT` informational).
- **Spectrum `value` is the aperture mean**, not the sum (contract field
  `value` says "Mean surface brightness in the aperture, MJy/sr"; a sum
  would carry wrong units). `sigma` = uncertainty of the mean.
- **Wavelength tolerance is enforced in `coadd_epochs` too**, not only
  `subtract_epochs` — the task groups "before any epoch arithmetic".
- Representative `wavelength_um` of a `ScienceImage`/stack = mean of the
  contributing central wavelengths (WCS-WAVE at image centre). Epoch
  diff/coadd refuse `|Δλ| > tol` with `WavelengthMismatch` naming both.
- ZODI is a model: subtraction never adds variance.

## Open issues

- `ScienceError` -> `AppError` mapping is b5's glue (suggestion:
  `ShapeMismatch`/`GridMismatch`/`WavelengthMismatch`/`UnknownWavelength`/
  `InvalidDetector`/`InvalidAperture`/`InvalidGrid` -> 400;
  `EmptyInput` + empty `extract_spectrum` result -> 404;
  `Wcs`/`EmptyInput`-from-stacks -> 500 or upstream as appropriate).
- `OutputGrid` has no inverse (`radec_to_pixel`): not needed by the
  current pipeline; add if the frontend needs pixel->sky on output
  coordinates beyond `pixel_to_radec`.
- Bilinear variance propagation assumes independent input pixels —
  reasonable for resampling, documented in `Interp`.
- Choose `npix` for `OutputGrid::centered` in b5 (e.g. round(fov/native
  pixel scale)); out of scope here.

## Verify

```
cd backend
cargo fmt --check          # ok
cargo clippy --all-targets -- -D warnings   # ok
cargo test                 # 36 pass: 15 science + 13 fits + 8 validation
```

Required tests present: 32x32 moved source (10,10)->(12,10) gives
+100/-100/0; 128x128 with NaN/Inf dead pixels stays finite where any
valid input exists; 2-frame inverse-variance result matches the
hand-computed value (e.g. (3*4 + 5*16)/20 = 4.6, var = 0.05).

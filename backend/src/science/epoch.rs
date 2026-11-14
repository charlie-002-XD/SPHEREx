//! Epoch arithmetic on same-grid [`ScienceImage`]s: subtraction,
//! co-addition and per-pixel temporal statistics for moving-object
//! (blink/difference) work.
//!
//! Before any arithmetic, epochs must share the same [`OutputGrid`]
//! (reproject raw frames first via [`crate::science::reproject`]) and
//! their central wavelengths must agree within a tolerance
//! ([`crate::science::DEFAULT_WAVELENGTH_TOLERANCE_UM`], 0.05 um): a
//! SPHEREx exposure's wavelength varies across pixels, so the same
//! target can land at different wavelengths in different epochs and
//! differencing across colours is meaningless.

use ndarray::Array2;

use super::error::ScienceError;
use super::image::{check_compatible, check_wavelengths, stack_images, ScienceImage, StackOptions};

/// `later - earlier`, per pixel.
///
/// Both epochs must be on the same grid and within `tolerance_um` in
/// central wavelength (else `GridMismatch`/`ShapeMismatch`/
/// `WavelengthMismatch`/`UnknownWavelength`). Output:
/// `image = later - earlier` where both are finite (else NaN),
/// `variance = var_later + var_earlier`, `flags` = bitwise OR, and
/// `n_contrib` = sum of contributors. Metadata (`detector`,
/// `wavelength_um`, `mjd`) is taken from `later`.
pub fn subtract_epochs(
    later: &ScienceImage,
    earlier: &ScienceImage,
    tolerance_um: f64,
) -> Result<ScienceImage, ScienceError> {
    check_compatible(later, earlier)?;
    check_wavelengths(later, earlier, tolerance_um)?;
    let (ny, nx) = (later.grid.ny, later.grid.nx);
    let mut out = ScienceImage::blank(later.grid);
    for y in 0..ny {
        for x in 0..nx {
            out.flags[[y, x]] = later.flags[[y, x]] | earlier.flags[[y, x]];
            let a = later.image[[y, x]];
            let b = earlier.image[[y, x]];
            if !(a.is_finite() && b.is_finite()) {
                continue;
            }
            out.image[[y, x]] = a - b;
            out.n_contrib[[y, x]] =
                later.n_contrib[[y, x]].saturating_add(earlier.n_contrib[[y, x]]);
            let (va, vb) = (later.variance[[y, x]], earlier.variance[[y, x]]);
            if va.is_finite() && vb.is_finite() {
                out.variance[[y, x]] = va + vb;
            }
        }
    }
    out.detector = later.detector;
    out.wavelength_um = later.wavelength_um;
    out.mjd = later.mjd;
    out.zodi_subtracted = later.zodi_subtracted && earlier.zodi_subtracted;
    Ok(out)
}

/// Inverse-variance co-addition of two epochs: `w = 1/variance` as in
/// [`stack_images`], subject to the same grid and wavelength-tolerance
/// checks as [`subtract_epochs`] plus `opts.bad_flags_mask`.
pub fn coadd_epochs(
    a: &ScienceImage,
    b: &ScienceImage,
    tolerance_um: f64,
    opts: &StackOptions,
) -> Result<ScienceImage, ScienceError> {
    check_compatible(a, b)?;
    check_wavelengths(a, b, tolerance_um)?;
    stack_images(&[a, b], opts)
}

/// Per-pixel statistics across an ordered sequence of same-grid epochs,
/// usable to drive blink/difference modes for moving objects.
///
/// Non-finite pixels are skipped; statistics over `n = 0` stay NaN.
#[derive(Debug, Clone)]
pub struct TemporalStats {
    /// Number of finite epoch values per pixel.
    pub n: Array2<u16>,
    /// Temporal mean flux (MJy/sr).
    pub mean: Array2<f32>,
    /// Population standard deviation across epochs.
    pub std: Array2<f32>,
    /// Minimum epoch flux per pixel.
    pub min: Array2<f32>,
    /// Maximum epoch flux per pixel.
    pub max: Array2<f32>,
    /// `max - min` per pixel: peak excursion across the sequence.
    pub peak_to_peak: Array2<f32>,
    /// `last - first` per pixel (sequence order): net change, e.g. a
    /// moving source's trailing/leading signature.
    pub last_minus_first: Array2<f32>,
}

/// Compute [`TemporalStats`] over `epochs` in sequence order.
pub fn temporal_stats(epochs: &[&ScienceImage]) -> Result<TemporalStats, ScienceError> {
    let (first, rest) = epochs.split_first().ok_or(ScienceError::EmptyInput)?;
    for e in rest {
        check_compatible(first, e)?;
    }
    let (ny, nx) = (first.grid.ny, first.grid.nx);
    let shape = (ny, nx);
    let mut st = TemporalStats {
        n: Array2::from_elem(shape, 0u16),
        mean: Array2::from_elem(shape, f32::NAN),
        std: Array2::from_elem(shape, f32::NAN),
        min: Array2::from_elem(shape, f32::NAN),
        max: Array2::from_elem(shape, f32::NAN),
        peak_to_peak: Array2::from_elem(shape, f32::NAN),
        last_minus_first: Array2::from_elem(shape, f32::NAN),
    };
    let (last, _) = epochs.split_last().ok_or(ScienceError::EmptyInput)?;
    for y in 0..ny {
        for x in 0..nx {
            let (mut sum, mut sq, mut n) = (0.0f64, 0.0f64, 0u16);
            let (mut mn, mut mx) = (f64::INFINITY, f64::NEG_INFINITY);
            for e in epochs {
                let v = e.image[[y, x]];
                if !v.is_finite() {
                    continue;
                }
                let v = f64::from(v);
                sum += v;
                sq += v * v;
                n = n.saturating_add(1);
                mn = mn.min(v);
                mx = mx.max(v);
            }
            st.n[[y, x]] = n;
            if n == 0 {
                continue;
            }
            let mean = sum / f64::from(n);
            let var = (sq / f64::from(n) - mean * mean).max(0.0);
            st.mean[[y, x]] = mean as f32;
            st.std[[y, x]] = var.sqrt() as f32;
            st.min[[y, x]] = mn as f32;
            st.max[[y, x]] = mx as f32;
            st.peak_to_peak[[y, x]] = (mx - mn) as f32;
            let (f0, f1) = (first.image[[y, x]], last.image[[y, x]]);
            if f0.is_finite() && f1.is_finite() {
                st.last_minus_first[[y, x]] = f1 - f0;
            }
        }
    }
    Ok(st)
}

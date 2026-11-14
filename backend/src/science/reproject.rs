//! Reproject a raw [`Frame`] onto an [`OutputGrid`] through its sky WCS.
//!
//! For each output pixel `(x, y)` (1-based) the sky position comes from
//! `grid.wcs.pixel_to_radec`; the input sample position is then
//! `frame.wcs.radec_to_pixel(ra, dec)`. Output pixels outside the input
//! footprint, behind the tangent plane, or with no finite neighbour keep
//! `image = variance = NaN` and `n_contrib = 0`.

use crate::fits::Frame;

use super::error::ScienceError;
use super::grid::OutputGrid;
use super::image::ScienceImage;

/// Sampling method for [`reproject`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interp {
    /// Nearest-neighbour: exact value of the closest input pixel centre.
    /// Preserves fluxes and flags verbatim; edges stay sharp.
    Nearest,
    /// Bilinear over the four nearest input pixel centres, coefficients
    /// renormalized over the finite subset of each quantity. Variance
    /// propagates as `sum(c_i^2 * var_i)` (independent pixels); a
    /// neighbour with non-finite or non-positive variance is dropped
    /// from the variance sum, like the flux.
    #[default]
    Bilinear,
}

/// Options for [`reproject`].
#[derive(Debug, Clone, Copy)]
pub struct ReprojectOptions {
    /// Interpolation method for flux/variance/zodi. The flag bitmask is
    /// always nearest-sampled (interpolation of a bitmask is
    /// meaningless).
    pub interp: Interp,
    /// Subtract the ZODI model (`IMAGE - ZODI`) while resampling.
    /// SPHEREx `IMAGE` is not zodi-subtracted; the default `true`
    /// matches difference/stack modes, raw display passes `false`.
    /// A NaN ZODI sample leaves the flux untouched.
    pub subtract_zodi: bool,
}

impl Default for ReprojectOptions {
    fn default() -> Self {
        Self {
            interp: Interp::Bilinear,
            subtract_zodi: true,
        }
    }
}

/// Resample `frame` onto `grid`.
///
/// The result inherits `detector`, epoch (`MJD-AVG`, else `MJD-OBS`) and
/// a representative wavelength: the WCS-WAVE value at the input pixel
/// mapping back to the output grid's centre.
pub fn reproject(
    frame: &Frame,
    grid: &OutputGrid,
    opts: &ReprojectOptions,
) -> Result<ScienceImage, ScienceError> {
    let (ny, nx) = frame.image.dim();
    let mut out = ScienceImage::blank(*grid);
    out.detector = frame.header.detector;
    out.mjd = frame.header.mjd_avg.or(frame.header.mjd_obs);
    out.zodi_subtracted = opts.subtract_zodi;

    for oy in 0..grid.ny {
        for ox in 0..grid.nx {
            let (ra, dec) = grid.wcs.pixel_to_radec(ox as f64 + 1.0, oy as f64 + 1.0);
            let Ok((sx, sy)) = frame.wcs.radec_to_pixel(ra, dec) else {
                continue;
            };
            let Some((val, var, fl)) = sample(frame, sx, sy, nx, ny, opts.interp) else {
                continue;
            };
            let mut val = val;
            if opts.subtract_zodi {
                let z = match opts.interp {
                    Interp::Nearest => sample_nearest(&frame.zodi, sx, sy, nx, ny),
                    Interp::Bilinear => bilinear_f32(&frame.zodi, sx, sy, nx, ny),
                };
                if z.is_finite() {
                    val -= z;
                }
            }
            out.image[[oy, ox]] = val;
            out.variance[[oy, ox]] = var;
            out.flags[[oy, ox]] = fl;
            out.n_contrib[[oy, ox]] = u16::from(val.is_finite());
        }
    }

    let (cra, cdec) = grid
        .wcs
        .pixel_to_radec(grid.nx as f64 / 2.0 + 0.5, grid.ny as f64 / 2.0 + 0.5);
    out.wavelength_um = frame
        .wcs
        .radec_to_pixel(cra, cdec)
        .map(|(sx, sy)| frame.wavelength.wavelength_at(sx, sy))
        .unwrap_or(f64::NAN);
    Ok(out)
}

/// Footprint test: sample positions with centre coverage of the array.
/// `(sx, sy)` are 1-based source pixel coordinates.
fn in_footprint(sx: f64, sy: f64, nx: usize, ny: usize) -> bool {
    sx.is_finite()
        && sy.is_finite()
        && sx >= 0.5
        && sx <= nx as f64 + 0.5
        && sy >= 0.5
        && sy <= ny as f64 + 0.5
}

/// 0-based index of the nearest pixel centre to 1-based coord `s`.
fn nearest_idx(s: f64, n: usize) -> usize {
    (s.round() as i64).clamp(1, n as i64) as usize - 1
}

/// Up-to-4 bilinear neighbour cells of `(sx, sy)` clipped to `nx * ny`:
/// `(ix0, iy0, coeff)` tuples and their count.
fn bilinear_cells(sx: f64, sy: f64, nx: usize, ny: usize) -> ([(usize, usize, f64); 4], usize) {
    let jx = sx.floor() as i64;
    let jy = sy.floor() as i64;
    let fx = sx - jx as f64;
    let fy = sy - jy as f64;
    let mut cells = [(0usize, 0usize, 0.0f64); 4];
    let mut k = 0;
    for (iy, wy) in [(jy, 1.0 - fy), (jy + 1, fy)] {
        for (ix, wx) in [(jx, 1.0 - fx), (jx + 1, fx)] {
            if (1..=nx as i64).contains(&ix) && (1..=ny as i64).contains(&iy) {
                cells[k] = ((ix - 1) as usize, (iy - 1) as usize, wx * wy);
                k += 1;
            }
        }
    }
    (cells, k)
}

/// Bilinear sample of an f32 image: coefficients renormalized over the
/// finite neighbours; NaN when none are finite.
fn bilinear_f32(a: &ndarray::Array2<f32>, sx: f64, sy: f64, nx: usize, ny: usize) -> f32 {
    let (cells, k) = bilinear_cells(sx, sy, nx, ny);
    let (mut num, mut den) = (0.0f64, 0.0f64);
    for &(ix, iy, c) in &cells[..k] {
        let v = a[[iy, ix]];
        if v.is_finite() {
            num += c * f64::from(v);
            den += c;
        }
    }
    if den > 0.0 {
        (num / den) as f32
    } else {
        f32::NAN
    }
}

/// Bilinear variance propagation `sum(c_i^2 v_i)` over neighbours with
/// finite, positive variance (coefficients renormalized on that subset).
fn bilinear_var(a: &ndarray::Array2<f32>, sx: f64, sy: f64, nx: usize, ny: usize) -> f32 {
    let (cells, k) = bilinear_cells(sx, sy, nx, ny);
    let (mut acc, mut den) = (0.0f64, 0.0f64);
    for &(ix, iy, c) in &cells[..k] {
        let v = a[[iy, ix]];
        if v.is_finite() && v > 0.0 {
            acc += c * c * f64::from(v);
            den += c;
        }
    }
    if den > 0.0 {
        (acc / (den * den)) as f32
    } else {
        f32::NAN
    }
}

/// Nearest-neighbour sample of an f32 image (any value, including NaN).
fn sample_nearest(a: &ndarray::Array2<f32>, sx: f64, sy: f64, nx: usize, ny: usize) -> f32 {
    a[[nearest_idx(sy, ny), nearest_idx(sx, nx)]]
}

/// Sample `frame` at source coords `(sx, sy)` -> `(flux, variance,
/// flags)`; `None` when outside the footprint.
fn sample(
    frame: &Frame,
    sx: f64,
    sy: f64,
    nx: usize,
    ny: usize,
    interp: Interp,
) -> Option<(f32, f32, i32)> {
    if !in_footprint(sx, sy, nx, ny) {
        return None;
    }
    let flags = frame.flags[[nearest_idx(sy, ny), nearest_idx(sx, nx)]];
    let (val, var) = match interp {
        Interp::Nearest => (
            sample_nearest(&frame.image, sx, sy, nx, ny),
            sample_nearest(&frame.variance, sx, sy, nx, ny),
        ),
        Interp::Bilinear => (
            bilinear_f32(&frame.image, sx, sy, nx, ny),
            bilinear_var(&frame.variance, sx, sy, nx, ny),
        ),
    };
    Some((val, var, flags))
}

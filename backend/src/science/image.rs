//! `ScienceImage`: a flux/variance/flags raster on an [`OutputGrid`],
//! plus inverse-variance stacking and detector x wavelength grouping of
//! raw frames.
//!
//! A `ScienceImage` is produced from a raw `Frame` by
//! [`crate::science::reproject`] (or built synthetically in tests) and is
//! the currency of all epoch arithmetic: stacking, subtraction,
//! co-addition and temporal statistics.

use std::collections::BTreeMap;

use ndarray::Array2;

use crate::fits::{FlagLegend, Frame};

use super::error::ScienceError;
use super::grid::OutputGrid;

/// Default wavelength tolerance (um) for epoch arithmetic:
/// frames/epochs whose central wavelengths differ by more than this are
/// never differenced or co-added.
pub const DEFAULT_WAVELENGTH_TOLERANCE_UM: f64 = 0.05;

/// A science raster on an [`OutputGrid`]: surface brightness, variance,
/// flags and per-pixel provenance.
///
/// `image`/`variance`/`flags`/`n_contrib` are `Array2[y][x]`; all sky
/// coordinates remain 1-based FITS pixels.
#[derive(Debug, Clone)]
pub struct ScienceImage {
    /// Grid (dims + linear TAN WCS) this raster lives on. Epoch
    /// arithmetic requires identical grids.
    pub grid: OutputGrid,
    /// Surface brightness, MJy/sr; NaN where no valid data.
    pub image: Array2<f32>,
    /// Variance, (MJy/sr)^2; NaN where unknown.
    pub variance: Array2<f32>,
    /// Flag bitmask (nearest-resampled, union over contributors).
    /// Bit meanings come from the FITS legend, never hardcoded.
    pub flags: Array2<i32>,
    /// Contributing exposures per pixel.
    pub n_contrib: Array2<u16>,
    /// Detector number when the raster is single-detector (1..=6).
    pub detector: Option<u8>,
    /// Representative central wavelength (um): mean of the contributing
    /// frames' central wavelengths; NaN when unknown/mixed beyond
    /// tolerance. Used only for the epoch wavelength-tolerance check.
    pub wavelength_um: f64,
    /// Representative epoch (mean MJD of contributors), when known.
    pub mjd: Option<f64>,
    /// True when the ZODI model was already subtracted from `image`.
    pub zodi_subtracted: bool,
}

impl ScienceImage {
    /// NaN-filled raster on `grid` (no contributors, no metadata).
    pub fn blank(grid: OutputGrid) -> Self {
        let shape = (grid.ny, grid.nx);
        Self {
            grid,
            image: Array2::from_elem(shape, f32::NAN),
            variance: Array2::from_elem(shape, f32::NAN),
            flags: Array2::from_elem(shape, 0),
            n_contrib: Array2::from_elem(shape, 0),
            detector: None,
            wavelength_um: f64::NAN,
            mjd: None,
            zodi_subtracted: false,
        }
    }

    /// `[nx, ny]` of the raster.
    pub fn dims(&self) -> [usize; 2] {
        let (ny, nx) = self.image.dim();
        [nx, ny]
    }
}

/// Options for [`stack_images`] / [`stack_detectors`].
#[derive(Debug, Clone, Copy)]
pub struct StackOptions {
    /// Bitmask of "bad" FLAGS bits: an input pixel whose flags share any
    /// bit with this mask gets weight 0. Bit meanings are
    /// caller-supplied from the FITS legend (see [`mask_from_names`]).
    /// The default `-1` treats every flagged pixel as bad; pass a
    /// narrower mask to keep informational bits (e.g. `SOURCE`).
    pub bad_flags_mask: i32,
}

impl Default for StackOptions {
    fn default() -> Self {
        Self { bad_flags_mask: -1 }
    }
}

/// Build a bad-flags bitmask from legend names, e.g.
/// `mask_from_names(legend, &["HOT", "SATURATED"])`; unknown names are
/// ignored. Bit numbers come from the FITS legend (`MSKNnnnn`), never
/// hardcoded.
pub fn mask_from_names(legend: &FlagLegend, names: &[&str]) -> i32 {
    let mut mask = 0u32;
    for (bit, b) in legend {
        if names.iter().any(|n| *n == b.name) {
            mask |= 1u32 << bit;
        }
    }
    mask as i32
}

/// Inverse-variance weighted mean of same-grid images.
///
/// Per pixel, each input contributes `w = 1/variance` when its flux is
/// finite, `variance` is finite and `> 0`, and
/// `flags & opts.bad_flags_mask == 0`; otherwise `w = 0`.
/// Output: `image = sum(w_i f_i)/sum(w_i)`, `variance = 1/sum(w_i)`;
/// pixels with zero total weight stay NaN. `flags` is the bitwise OR of
/// all inputs that had finite flux (including rejected ones, so the
/// reason for rejection remains visible); `n_contrib` counts accepted
/// inputs.
///
/// `wavelength_um`/`mjd` of the result are the unweighted means of the
/// inputs' representative values; `detector` survives only if every
/// input shares it.
pub fn stack_images(
    images: &[&ScienceImage],
    opts: &StackOptions,
) -> Result<ScienceImage, ScienceError> {
    let (first, rest) = images.split_first().ok_or(ScienceError::EmptyInput)?;
    for im in rest {
        check_compatible(first, im)?;
    }
    let grid = first.grid;
    let (ny, nx) = (grid.ny, grid.nx);
    let mut out = ScienceImage::blank(grid);
    for y in 0..ny {
        for x in 0..nx {
            let mut num = 0.0f64;
            let mut wsum = 0.0f64;
            let mut n = 0u16;
            let mut fl = 0i32;
            for im in images {
                let v = im.image[[y, x]];
                if !v.is_finite() {
                    continue;
                }
                fl |= im.flags[[y, x]];
                let var = im.variance[[y, x]];
                if !(var.is_finite() && var > 0.0) || im.flags[[y, x]] & opts.bad_flags_mask != 0 {
                    continue;
                }
                let w = 1.0 / f64::from(var);
                num += w * f64::from(v);
                wsum += w;
                n = n.saturating_add(1);
            }
            out.flags[[y, x]] = fl;
            if wsum > 0.0 {
                out.image[[y, x]] = (num / wsum) as f32;
                out.variance[[y, x]] = (1.0 / wsum) as f32;
                out.n_contrib[[y, x]] = n;
            }
        }
    }
    out.detector = common_detector(images);
    out.wavelength_um = mean_f64(images.iter().map(|im| im.wavelength_um));
    out.mjd = mean_opt(images.iter().map(|im| im.mjd));
    out.zodi_subtracted = images.iter().all(|im| im.zodi_subtracted);
    Ok(out)
}

/// Result of [`stack_detectors`].
#[derive(Debug)]
pub struct StackResult {
    /// One inverse-variance stack per detector array (D1..D6) present in
    /// the input.
    pub per_detector: BTreeMap<u8, ScienceImage>,
    /// Inverse-variance stack across all arrays. This is a
    /// mixed-wavelength deep image (SPHEREx detector bands do not
    /// overlap): `wavelength_um` is the mean of the contributors'
    /// central wavelengths and `detector` is `None`, so epoch
    /// subtraction against a single-band epoch fails the tolerance
    /// check as intended.
    pub combined: ScienceImage,
}

/// Inverse-variance combination per detector array (D1..D6) and across
/// arrays.
///
/// `images_by_detector` maps detector number -> same-grid images to
/// combine; build it with [`group_frames`] + [`crate::science::reproject`]
/// (or filter `ScienceImage`s by their `detector` field). Keys outside
/// `1..=6` are rejected; empty groups are skipped. Within a detector the
/// stack is wavelength-agnostic (a deeper broadband image); epoch
/// *differencing* still enforces the wavelength tolerance through
/// [`crate::science::subtract_epochs`].
pub fn stack_detectors(
    images_by_detector: &BTreeMap<u8, Vec<&ScienceImage>>,
    opts: &StackOptions,
) -> Result<StackResult, ScienceError> {
    if images_by_detector.is_empty() {
        return Err(ScienceError::EmptyInput);
    }
    let mut per_detector = BTreeMap::new();
    let mut all: Vec<&ScienceImage> = Vec::new();
    for (&det, group) in images_by_detector {
        if !(1..=6).contains(&det) {
            return Err(ScienceError::InvalidDetector { detector: det });
        }
        if group.is_empty() {
            continue;
        }
        let mut s = stack_images(group, opts)?;
        s.detector = Some(det);
        per_detector.insert(det, s);
        all.extend(group.iter().copied());
    }
    if all.is_empty() {
        return Err(ScienceError::EmptyInput);
    }
    let mut combined = stack_images(&all, opts)?;
    combined.detector = None;
    Ok(StackResult {
        per_detector,
        combined,
    })
}

/// One detector x wavelength cluster: frames sorted by epoch.
#[derive(Debug)]
pub struct FrameGroup<'a> {
    /// Detector array, 1..=6.
    pub detector: u8,
    /// Mean central wavelength (um) of the member frames.
    pub wavelength_um: f64,
    /// Members sorted by epoch (`MJD-AVG`, else `MJD-OBS`, else 0).
    pub frames: Vec<&'a Frame>,
}

/// Group raw frames by detector (1-6) and central wavelength before any
/// epoch arithmetic.
///
/// Each exposure's wavelength varies across pixels, so the clustering
/// key is the WCS-WAVE value at the frame centre. Within a detector,
/// frames sorted by that wavelength are clustered greedily: a new
/// cluster starts when `|lambda - first cluster lambda| >
/// tolerance_um`. Members of a cluster are sorted by epoch. Frames with
/// a missing or out-of-range `DETECTOR` keyword are skipped. Frames with
/// a NaN central wavelength each form a single-member cluster (epoch
/// arithmetic on them fails later with
/// [`ScienceError::UnknownWavelength`]).
pub fn group_frames<'a>(
    frames: &[&'a Frame],
    tolerance_um: f64,
) -> Result<Vec<FrameGroup<'a>>, ScienceError> {
    if frames.is_empty() {
        return Err(ScienceError::EmptyInput);
    }
    let mut by_det: BTreeMap<u8, Vec<(&'a Frame, f64)>> = BTreeMap::new();
    for f in frames {
        match f.header.detector {
            Some(det) if (1..=6).contains(&det) => {
                by_det
                    .entry(det)
                    .or_default()
                    .push((*f, central_wavelength(f)));
            }
            _ => continue,
        }
    }
    let mut groups = Vec::new();
    for (det, mut list) in by_det {
        list.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut cluster: Vec<(&'a Frame, f64)> = Vec::new();
        let mut anchor = f64::NAN;
        for (f, w) in list {
            if cluster.is_empty() {
                anchor = w;
                cluster.push((f, w));
            } else if (w - anchor).abs() <= tolerance_um {
                cluster.push((f, w));
            } else {
                groups.push(make_group(det, cluster));
                cluster = vec![(f, w)];
                anchor = w;
            }
        }
        if !cluster.is_empty() {
            groups.push(make_group(det, cluster));
        }
    }
    Ok(groups)
}

/// Central wavelength (um) of a frame: WCS-WAVE at the image centre.
pub fn central_wavelength(frame: &Frame) -> f64 {
    let (ny, nx) = frame.image.dim();
    frame
        .wavelength
        .wavelength_at((nx as f64 + 1.0) / 2.0, (ny as f64 + 1.0) / 2.0)
}

/// Representative epoch of a frame: `MJD-AVG`, else `MJD-OBS`, else NaN.
pub fn frame_epoch(frame: &Frame) -> f64 {
    frame
        .header
        .mjd_avg
        .or(frame.header.mjd_obs)
        .unwrap_or(f64::NAN)
}

/// Same shape and same grid required for per-pixel arithmetic.
pub(crate) fn check_compatible(a: &ScienceImage, b: &ScienceImage) -> Result<(), ScienceError> {
    if a.dims() != b.dims() {
        return Err(ScienceError::ShapeMismatch {
            a: a.dims(),
            b: b.dims(),
        });
    }
    if a.grid != b.grid {
        return Err(ScienceError::GridMismatch);
    }
    Ok(())
}

/// The wavelength-tolerance check shared by epoch subtraction and
/// co-addition. NaN wavelengths are refused; a difference larger than
/// `tolerance_um` errors naming both wavelengths.
pub(crate) fn check_wavelengths(
    a: &ScienceImage,
    b: &ScienceImage,
    tolerance_um: f64,
) -> Result<(), ScienceError> {
    if !a.wavelength_um.is_finite() || !b.wavelength_um.is_finite() {
        return Err(ScienceError::UnknownWavelength);
    }
    if (a.wavelength_um - b.wavelength_um).abs() > tolerance_um {
        return Err(ScienceError::WavelengthMismatch {
            wave_a_um: a.wavelength_um,
            wave_b_um: b.wavelength_um,
            tolerance_um,
        });
    }
    Ok(())
}

fn make_group<'a>(detector: u8, mut members: Vec<(&'a Frame, f64)>) -> FrameGroup<'a> {
    members.sort_by(|a, b| frame_epoch(a.0).total_cmp(&frame_epoch(b.0)));
    let wavelength_um = mean_f64(members.iter().map(|m| m.1));
    FrameGroup {
        detector,
        wavelength_um,
        frames: members.into_iter().map(|m| m.0).collect(),
    }
}

fn common_detector(images: &[&ScienceImage]) -> Option<u8> {
    let d = images.first()?.detector;
    images.iter().all(|im| im.detector == d).then_some(d)?
}

/// Unweighted mean of the finite entries; NaN if none.
fn mean_f64(it: impl Iterator<Item = f64>) -> f64 {
    let (mut s, mut n) = (0.0f64, 0u64);
    for v in it {
        if v.is_finite() {
            s += v;
            n += 1;
        }
    }
    if n > 0 {
        s / n as f64
    } else {
        f64::NAN
    }
}

fn mean_opt(it: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let (mut s, mut n) = (0.0f64, 0u64);
    for v in it.flatten() {
        if v.is_finite() {
            s += v;
            n += 1;
        }
    }
    (n > 0).then_some(s / n as f64)
}

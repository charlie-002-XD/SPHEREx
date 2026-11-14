//! Tests for the science engine (`spherex_backend::science`).
//!
//! Arithmetic tests build `ScienceImage`s directly on `OutputGrid`s;
//! tests needing a real `Frame` (grouping, reprojection, spectrum) build
//! synthetic MEF cutouts through `fixtures` + `parse_frame`. No network.

mod fixtures;

use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use fixtures::*;
use spherex_backend::fits::{parse_frame, Frame};
use spherex_backend::science::*;

// ---------------------------------------------------------------------
// ScienceImage helpers
// ---------------------------------------------------------------------

/// Flat ScienceImage on an identity grid: `image = val`,
/// `variance = var`, `wavelength_um = wave`.
fn sci(nx: usize, ny: usize, val: f32, var: f32, wave: f64) -> ScienceImage {
    let mut im = ScienceImage::blank(OutputGrid::identity(nx, ny));
    im.image.fill(val);
    im.variance.fill(var);
    im.n_contrib.fill(1);
    im.wavelength_um = wave;
    im
}

// ---------------------------------------------------------------------
// Synthetic Frame builder (plain TAN, no SIP)
// ---------------------------------------------------------------------

/// Contents of a synthetic MEF cutout for [`frame`].
struct FrameSpec {
    detector: i64,
    wave_um: f64,
    mjd: f64,
    nx: usize,
    ny: usize,
    image: Vec<f32>,
    variance: Vec<f32>,
    flags: Vec<i32>,
    zodi: Vec<f32>,
}

/// Minimal MEF cutout: IMAGE (TAN WCS, DETECTOR, MJD), FLAGS (legend:
/// TRANSIENT/SATURATED/HOT), VARIANCE, ZODI, WCS-WAVE 2x2 grid of
/// constant `wave_um`.
fn frame(spec: &FrameSpec) -> Frame {
    let (nx, ny) = (spec.nx, spec.ny);
    let mut f = primary_hdu();
    let img_extra = vec![
        card_str("CTYPE1", "RA---TAN"),
        card_str("CTYPE2", "DEC--TAN"),
        card_flt("CRPIX1", nx as f64 / 2.0 + 0.5),
        card_flt("CRPIX2", ny as f64 / 2.0 + 0.5),
        card_flt("CRVAL1", 150.0),
        card_flt("CRVAL2", 2.0),
        card_flt("CD1_1", -0.001),
        card_flt("CD1_2", 0.0),
        card_flt("CD2_1", 0.0),
        card_flt("CD2_2", 0.001),
        card_int("DETECTOR", spec.detector),
        card_flt("MJD-AVG", spec.mjd),
        card_flt("MJD-OBS", spec.mjd - 0.001),
        card_str("BUNIT", "MJy / sr"),
    ];
    f.extend(image_hdu(
        "IMAGE",
        -32,
        nx,
        ny,
        &f32_data(&spec.image),
        &img_extra,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        nx,
        ny,
        &i32_data(&spec.flags),
        &[
            card_str("MSKN0000", "TRANSIENT"),
            card_str("MSKN0004", "SATURATED"),
            card_str("MSKN0010", "HOT"),
        ],
    ));
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        nx,
        ny,
        &f32_data(&spec.variance),
        &[],
    ));
    f.extend(image_hdu("ZODI", -32, nx, ny, &f32_data(&spec.zodi), &[]));
    let xs = vec![1i32, nx as i32];
    let ys = vec![1i32, ny as i32];
    let mut values = Vec::new();
    for _ in 0..(xs.len() * ys.len()) {
        values.push(spec.wave_um as f32);
        values.push((spec.wave_um / 40.0) as f32);
    }
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));
    parse_frame(&f).expect("synthetic frame must parse")
}

/// Flat-frame convenience: image `val`, variance `var`, flags 0, zodi `z`.
fn flat_frame(
    detector: i64,
    wave_um: f64,
    mjd: f64,
    dims: (usize, usize),
    val: f32,
    var: f32,
    z: f32,
) -> Frame {
    let (nx, ny) = dims;
    frame(&FrameSpec {
        detector,
        wave_um,
        mjd,
        nx,
        ny,
        image: vec![val; nx * ny],
        variance: vec![var; nx * ny],
        flags: vec![0; nx * ny],
        zodi: vec![z; nx * ny],
    })
}

// ---------------------------------------------------------------------
// Required test: moved point source subtraction
// ---------------------------------------------------------------------

#[test]
fn moved_point_source_subtraction() {
    // 32x32: source at (x=10, y=10) in t1 moves to (x=12, y=10) in t2.
    let mut t1 = sci(32, 32, 0.0, 1.0, 1.5);
    let mut t2 = sci(32, 32, 0.0, 1.0, 1.5);
    t1.image[[10, 10]] = 100.0;
    t2.image[[10, 12]] = 100.0;

    let d = subtract_epochs(&t2, &t1, DEFAULT_WAVELENGTH_TOLERANCE_UM).expect("subtract failed");
    assert_eq!(d.image[[10, 12]], 100.0, "+100 at (12,10)");
    assert_eq!(d.image[[10, 10]], -100.0, "-100 at (10,10)");
    for y in 0..32 {
        for x in 0..32 {
            if (y, x) == (10, 10) || (y, x) == (10, 12) {
                continue;
            }
            assert_eq!(d.image[[y, x]], 0.0, "nonzero at ({x},{y})");
        }
    }
    assert_eq!(d.variance[[10, 10]], 2.0);
    assert_eq!(d.n_contrib[[10, 10]], 2);
    assert_eq!(d.wavelength_um, 1.5);
}

// ---------------------------------------------------------------------
// Required test: NaN/Inf dead pixels stay finite where any valid input
// ---------------------------------------------------------------------

#[test]
fn stack_stays_finite_with_nan_inf_dead_pixels() {
    const N: usize = 128;
    let mut a = sci(N, N, 1.0, 0.04, 1.5);
    let mut b = sci(N, N, 2.0, 0.04, 1.5);
    for y in 0..N {
        for x in 0..N {
            let i = y * N + x;
            if i.is_multiple_of(7) {
                a.image[[y, x]] = f32::NAN;
            }
            if i.is_multiple_of(11) {
                b.image[[y, x]] = f32::INFINITY;
            }
            if i.is_multiple_of(13) {
                b.image[[y, x]] = f32::NEG_INFINITY;
            }
            if i.is_multiple_of(97) {
                a.image[[y, x]] = f32::NAN;
                b.image[[y, x]] = f32::NAN;
            }
        }
    }
    let s = stack_images(&[&a, &b], &StackOptions { bad_flags_mask: 0 }).expect("stack failed");
    for y in 0..N {
        for x in 0..N {
            let any_valid = a.image[[y, x]].is_finite() || b.image[[y, x]].is_finite();
            assert_eq!(s.image[[y, x]].is_finite(), any_valid, "at ({x},{y})");
            if any_valid {
                let want_n =
                    u16::from(a.image[[y, x]].is_finite()) + u16::from(b.image[[y, x]].is_finite());
                assert_eq!(s.n_contrib[[y, x]], want_n);
                assert!(s.variance[[y, x]].is_finite() && s.variance[[y, x]] > 0.0);
            }
        }
    }
}

// ---------------------------------------------------------------------
// Required test: hand-computed inverse-variance result
// ---------------------------------------------------------------------

#[test]
fn inverse_variance_matches_hand_computed() {
    // Two frames: weights 1/0.25 = 4 and 1/0.0625 = 16.
    // Pixel (0,0): (3*4 + 5*16)/20 = 4.6; var = 1/20 = 0.05.
    let mut a = sci(2, 2, 0.0, 0.25, 1.5);
    let mut b = sci(2, 2, 0.0, 0.0625, 1.5);
    a.image = ndarray::array![[3.0f32, 10.0], [1.0, -4.0]];
    b.image = ndarray::array![[5.0f32, 20.0], [3.0, 8.0]];
    let s = stack_images(&[&a, &b], &StackOptions { bad_flags_mask: 0 }).expect("stack failed");
    let want = |va: f64, vb: f64| (va * 4.0 + vb * 16.0) / 20.0;
    for y in 0..2 {
        for x in 0..2 {
            let e = want(f64::from(a.image[[y, x]]), f64::from(b.image[[y, x]]));
            assert!(
                (f64::from(s.image[[y, x]]) - e).abs() < 1e-6,
                "at ({x},{y}): {} vs {e}",
                s.image[[y, x]]
            );
            assert!((f64::from(s.variance[[y, x]]) - 0.05).abs() < 1e-6);
            assert_eq!(s.n_contrib[[y, x]], 2);
        }
    }
    assert!((s.image[[0, 0]] - 4.6).abs() < 1e-6);
}

// ---------------------------------------------------------------------
// Wavelength tolerance and grid checks
// ---------------------------------------------------------------------

#[test]
fn subtract_refuses_wavelength_mismatch() {
    let a = sci(4, 4, 1.0, 1.0, 1.50);
    let b = sci(4, 4, 2.0, 1.0, 1.60);
    let err = subtract_epochs(&b, &a, 0.05).unwrap_err();
    let msg = err.to_string();
    match err {
        ScienceError::WavelengthMismatch {
            wave_a_um,
            wave_b_um,
            tolerance_um,
        } => {
            assert_eq!(wave_a_um, 1.60);
            assert_eq!(wave_b_um, 1.50);
            assert_eq!(tolerance_um, 0.05);
        }
        other => panic!("expected WavelengthMismatch, got {other:?}"),
    }
    assert!(
        msg.contains("1.6000") && msg.contains("1.5000"),
        "msg: {msg}"
    );

    // Inside the tolerance it succeeds; NaN wavelength is refused.
    let b_ok = sci(4, 4, 2.0, 1.0, 1.53);
    assert!(subtract_epochs(&b_ok, &a, 0.05).is_ok());
    let b_nan = {
        let mut b = sci(4, 4, 2.0, 1.0, 1.5);
        b.wavelength_um = f64::NAN;
        b
    };
    assert!(matches!(
        subtract_epochs(&b_nan, &a, 0.05),
        Err(ScienceError::UnknownWavelength)
    ));
    // Same refusal applies to co-addition.
    assert!(matches!(
        coadd_epochs(&b, &a, 0.05, &StackOptions::default()),
        Err(ScienceError::WavelengthMismatch { .. })
    ));
}

#[test]
fn epoch_arithmetic_requires_same_grid() {
    let a = sci(4, 4, 1.0, 1.0, 1.5);
    let b_shape = sci(5, 4, 1.0, 1.0, 1.5);
    assert!(matches!(
        subtract_epochs(&a, &b_shape, 0.05),
        Err(ScienceError::ShapeMismatch { .. })
    ));
    // Same dims, different WCS.
    let mut b_wcs = sci(4, 4, 1.0, 1.0, 1.5);
    b_wcs.grid.wcs.crval = [150.0, 2.0];
    assert!(matches!(
        subtract_epochs(&a, &b_wcs, 0.05),
        Err(ScienceError::GridMismatch)
    ));
}

// ---------------------------------------------------------------------
// Flag masking, co-addition, stack_detectors
// ---------------------------------------------------------------------

#[test]
fn bad_flags_get_zero_weight() {
    let mut a = sci(2, 2, 100.0, 1.0, 1.5);
    let b = sci(2, 2, 1.0, 1.0, 1.5);
    a.flags[[0, 0]] = 1 << 4; // SATURATED bit
                              // Mask = all bits: flagged pixel drops out -> stack = b there.
    let s = stack_images(&[&a, &b], &StackOptions { bad_flags_mask: -1 }).unwrap();
    assert_eq!(s.image[[0, 0]], 1.0);
    assert_eq!(s.n_contrib[[0, 0]], 1);
    // The flag is still reported on the output.
    assert_eq!(s.flags[[0, 0]] & (1 << 4), 1 << 4);
    // Mask = 0: flags informational only -> both contribute.
    let s = stack_images(&[&a, &b], &StackOptions { bad_flags_mask: 0 }).unwrap();
    assert!((s.image[[0, 0]] - 50.5).abs() < 1e-6);
    assert_eq!(s.n_contrib[[0, 0]], 2);
}

#[test]
fn mask_from_names_uses_legend() {
    let fr = flat_frame(1, 1.0, 61000.0, (2, 2), 1.0, 1.0, 0.0);
    let legend = &fr.header.flag_legend;
    let mask = mask_from_names(legend, &["SATURATED", "HOT", "NOSUCH"]);
    assert_eq!(mask, (1 << 4) | (1 << 10));
}

#[test]
fn coadd_is_inverse_variance_mean() {
    let mut a = sci(3, 3, 2.0, 0.25, 1.5);
    let mut b = sci(3, 3, 6.0, 0.25, 1.5);
    a.detector = Some(1);
    b.detector = Some(1);
    let c = coadd_epochs(&a, &b, 0.05, &StackOptions::default()).unwrap();
    assert!((c.image[[1, 1]] - 4.0).abs() < 1e-6);
    assert!((f64::from(c.variance[[1, 1]]) - 0.125).abs() < 1e-6);
    assert_eq!(c.detector, Some(1));
}

#[test]
fn stack_detectors_per_array_and_combined() {
    let mut a1 = sci(4, 4, 1.0, 0.25, 1.0);
    let mut a2 = sci(4, 4, 3.0, 0.25, 1.0);
    a1.detector = Some(1);
    a2.detector = Some(1);
    let mut b1 = sci(4, 4, 9.0, 0.25, 2.0);
    b1.detector = Some(2);
    let mut by_det: BTreeMap<u8, Vec<&ScienceImage>> = BTreeMap::new();
    by_det.insert(1, vec![&a1, &a2]);
    by_det.insert(2, vec![&b1]);
    let r = stack_detectors(&by_det, &StackOptions { bad_flags_mask: 0 }).unwrap();
    assert!((r.per_detector[&1].image[[0, 0]] - 2.0).abs() < 1e-6);
    assert_eq!(r.per_detector[&1].detector, Some(1));
    assert!((r.per_detector[&2].image[[0, 0]] - 9.0).abs() < 1e-6);
    // Combined across arrays: (1+3+9)/3 with equal weights.
    assert!((r.combined.image[[0, 0]] - (13.0 / 3.0)).abs() < 1e-6);
    assert_eq!(r.combined.detector, None);
    assert_eq!(r.combined.n_contrib[[0, 0]], 3);

    // Detector key outside 1..=6 rejected.
    let mut bad: BTreeMap<u8, Vec<&ScienceImage>> = BTreeMap::new();
    bad.insert(7, vec![&a1]);
    assert!(matches!(
        stack_detectors(&bad, &StackOptions::default()),
        Err(ScienceError::InvalidDetector { detector: 7 })
    ));
}

// ---------------------------------------------------------------------
// Grouping by detector and wavelength (real Frame via fixtures)
// ---------------------------------------------------------------------

#[test]
fn group_frames_by_detector_and_wavelength() {
    let f1 = flat_frame(1, 1.00, 61000.0, (4, 4), 1.0, 1.0, 0.0);
    let f2 = flat_frame(1, 1.02, 61001.0, (4, 4), 1.0, 1.0, 0.0);
    let f3 = flat_frame(1, 1.20, 61002.0, (4, 4), 1.0, 1.0, 0.0);
    let f4 = flat_frame(3, 1.00, 61003.0, (4, 4), 1.0, 1.0, 0.0);
    let groups = group_frames(&[&f3, &f1, &f4, &f2], 0.05).unwrap();
    assert_eq!(groups.len(), 3, "{groups:?}");
    // Detector 1: clusters {1.00, 1.02} and {1.20}; detector 3: {1.00}.
    let d1: Vec<&FrameGroup> = groups.iter().filter(|g| g.detector == 1).collect();
    assert_eq!(d1.len(), 2);
    assert!(d1.iter().any(|g| g.frames.len() == 2));
    assert!(d1
        .iter()
        .any(|g| g.frames.len() == 1 && (g.wavelength_um - 1.2).abs() < 1e-3));
    let two = d1.iter().find(|g| g.frames.len() == 2).unwrap();
    // Members sorted by epoch: f1 (61000) then f2 (61001).
    assert_eq!(two.frames[0].header.mjd_avg, Some(61000.0));
    assert_eq!(two.frames[1].header.mjd_avg, Some(61001.0));
    assert!(groups
        .iter()
        .any(|g| g.detector == 3 && g.frames.len() == 1));
}

// ---------------------------------------------------------------------
// Reprojection
// ---------------------------------------------------------------------

#[test]
fn reproject_onto_same_grid_is_identity() {
    let nx = 8;
    let ny = 6;
    let mut img = vec![0.0f32; nx * ny];
    for y in 0..ny {
        for x in 0..nx {
            img[y * nx + x] = x as f32 * 0.1 + y as f32;
        }
    }
    let fr = frame(&FrameSpec {
        detector: 1,
        wave_um: 1.5,
        mjd: 61000.0,
        nx,
        ny,
        image: img.clone(),
        variance: vec![0.5; nx * ny],
        flags: vec![0; nx * ny],
        zodi: vec![0.25; nx * ny],
    });
    let grid = OutputGrid::from_frame(nx, ny, &fr.wcs);
    // Without zodi subtraction the bilinear resample at exact pixel
    // centres reproduces the input.
    let out = reproject(
        &fr,
        &grid,
        &ReprojectOptions {
            interp: Interp::Bilinear,
            subtract_zodi: false,
        },
    )
    .unwrap();
    for y in 0..ny {
        for x in 0..nx {
            assert!(
                (out.image[[y, x]] - img[y * nx + x]).abs() < 1e-5,
                "({x},{y}): {} vs {}",
                out.image[[y, x]],
                img[y * nx + x]
            );
            assert_eq!(out.n_contrib[[y, x]], 1);
        }
    }
    assert_eq!(out.detector, Some(1));
    assert_eq!(out.mjd, Some(61000.0));
    assert!((out.wavelength_um - 1.5).abs() < 1e-3);
    // With zodi subtraction, IMAGE - ZODI = gradient - 0.25.
    let out_z = reproject(&fr, &grid, &ReprojectOptions::default()).unwrap();
    assert!((out_z.image[[3, 3]] - (img[3 * nx + 3] - 0.25)).abs() < 1e-5);
    assert!(out_z.zodi_subtracted);
    assert!(!out.zodi_subtracted);
}

#[test]
fn reproject_outside_footprint_is_nan() {
    // Frame covers only the left half of a wider output grid.
    let fr = flat_frame(1, 1.5, 61000.0, (8, 8), 2.0, 1.0, 0.0);
    // Output grid: same pixel scale, but twice as wide, same CRPIX/CRVAL.
    let mut grid = OutputGrid::from_frame(8, 8, &fr.wcs);
    grid.nx = 16;
    let out = reproject(&fr, &grid, &ReprojectOptions::default()).unwrap();
    assert!(out.image[[4, 3]].is_finite());
    for y in 0..8 {
        for x in 12..16 {
            assert!(out.image[[y, x]].is_nan(), "({x},{y})");
            assert_eq!(out.n_contrib[[y, x]], 0);
        }
    }
}

// ---------------------------------------------------------------------
// Spectrum extraction
// ---------------------------------------------------------------------

#[test]
fn extract_spectrum_aperture_and_sort() {
    let nx = 9;
    let ny = 9;
    // Flat 1.0 field + a 100 source at the centre pixel; flags bit HOT.
    let mut img = vec![1.0f32; nx * ny];
    img[4 * nx + 4] = 100.0;
    let mut fl = vec![0i32; nx * ny];
    fl[4 * nx + 4] = 1 << 10;
    let make = |wave: f64, mjd: f64| {
        frame(&FrameSpec {
            detector: 2,
            wave_um: wave,
            mjd,
            nx,
            ny,
            image: img.clone(),
            variance: vec![0.09; nx * ny],
            flags: fl.clone(),
            zodi: vec![0.0; nx * ny],
        })
    };
    let f_hi = make(1.5, 61001.0);
    let f_lo = make(1.1, 61000.0);
    // Query at CRVAL -> the 9-px aperture straddles the source.
    let pts = extract_spectrum(&[&f_hi, &f_lo], 150.0, 2.0, 3, false).unwrap();
    assert_eq!(pts.len(), 2);
    // Sorted by wavelength.
    assert!(pts[0].wavelength_um < pts[1].wavelength_um);
    let p = &pts[0];
    assert!((p.wavelength_um - 1.1).abs() < 1e-3);
    // Mean over the 3x3 aperture: (8*1 + 100)/9 = 12.0.
    assert!((p.value - 12.0).abs() < 1e-6);
    // sigma = sqrt(9 * 0.09) / 9 = 0.1.
    assert!((p.sigma - 0.1).abs() < 1e-6);
    assert_eq!(p.detector, Some(2));
    assert_eq!(p.obs_mjd, Some(61000.0));
    assert_eq!(p.flags, vec!["HOT".to_string()]);

    // Zodi subtraction: zodi = 0.5 -> value drops by 0.5.
    let fz = {
        let mut img2 = vec![1.0f32; nx * ny];
        img2[4 * nx + 4] = 100.0;
        frame(&FrameSpec {
            detector: 2,
            wave_um: 1.1,
            mjd: 61000.0,
            nx,
            ny,
            image: img2,
            variance: vec![0.09; nx * ny],
            flags: vec![0; nx * ny],
            zodi: vec![0.5; nx * ny],
        })
    };
    let pz = &extract_spectrum(&[&fz], 150.0, 2.0, 3, true).unwrap()[0];
    assert!((pz.value - 11.5).abs() < 1e-6);

    // Position far outside -> no coverage.
    assert!(extract_spectrum(&[&f_hi], 10.0, -80.0, 3, false)
        .unwrap()
        .is_empty());
    // Even aperture rejected.
    assert!(matches!(
        extract_spectrum(&[&f_hi], 150.0, 2.0, 4, false),
        Err(ScienceError::InvalidAperture { aperture_px: 4 })
    ));
}

// ---------------------------------------------------------------------
// Temporal stats (moving-object helper)
// ---------------------------------------------------------------------

#[test]
fn temporal_stats_track_a_moving_source() {
    let mut e: Vec<ScienceImage> = (0..3).map(|_| sci(4, 4, 0.0, 1.0, 1.5)).collect();
    // Source moving along x at row 2: +100 at x=0,1,2 across epochs.
    for (i, im) in e.iter_mut().enumerate() {
        im.image[[2, i]] = 100.0;
    }
    let refs: Vec<&ScienceImage> = e.iter().collect();
    let st = temporal_stats(&refs).unwrap();
    // Pixel (0,2): sequence [100, 0, 0] -> mean 33.33, ptp 100.
    assert!((st.mean[[2, 0]] - 100.0 / 3.0).abs() < 1e-5);
    assert_eq!(st.min[[2, 0]], 0.0);
    assert_eq!(st.max[[2, 0]], 100.0);
    assert_eq!(st.peak_to_peak[[2, 0]], 100.0);
    // Std of [100,0,0] = sqrt((20000/3 - (100/3)^2)... = sqrt(2/9)*100 ~ 47.14
    assert!((st.std[[2, 0]] - 100.0 * (2.0f32 / 9.0).sqrt()).abs() < 1e-4);
    // Net change first->last at (0,2): 0 - 100 = -100.
    assert_eq!(st.last_minus_first[[2, 0]], -100.0);
    assert_eq!(st.n[[2, 0]], 3);
}

// ---------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------

#[test]
fn image_encodes_as_f32_le_base64() {
    let img = ndarray::array![[1.0f32, -2.5], [f32::NAN, 4.0]];
    let bytes = image_to_le_bytes(&img);
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
    assert_eq!(&bytes[4..8], &(-2.5f32).to_le_bytes());
    let b64 = image_to_b64(&img);
    let raw = STANDARD.decode(&b64).expect("b64 decode");
    assert_eq!(raw, bytes);
    // NaN survives as a float32 NaN bit pattern.
    assert!(f32::from_le_bytes(raw[8..12].try_into().unwrap()).is_nan());
}

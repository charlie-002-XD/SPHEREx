//! Tests for the SPHEREx MEF parser (`spherex_backend::fits`).
//!
//! All fixtures are synthetic; no test touches the network or real IRSA.
//! An optional opt-in test (`SPHEREX_FITS_REAL=/path/to.fits`) runs the
//! parser against a real IRSA cutout when a file is provided locally.

mod fixtures;

use fixtures::*;
use spherex_backend::fits::{flags_to_names, parse_frame, FitsError};

const NX: usize = 4;
const NY: usize = 3;

/// Standard WCS cards (TAN + SIP) appended to the IMAGE header.
fn wcs_cards(sip: bool) -> Vec<String> {
    let mut c = vec![
        card_str("CTYPE1", "RA---TAN-SIP"),
        card_str("CTYPE2", "DEC--TAN-SIP"),
        card_flt("CRPIX1", 2.0),
        card_flt("CRPIX2", 2.0),
        card_flt("CRVAL1", 150.0),
        card_flt("CRVAL2", 2.0),
        card_flt("CD1_1", 0.001),
        card_flt("CD1_2", 0.0002),
        card_flt("CD2_1", -0.0002),
        card_flt("CD2_2", 0.001),
        card_flt("CDELT1", 1.0),
        card_flt("CDELT2", 1.0),
        // WCS-WAVE linear transform (identity here).
        card_flt("CRPIX1W", 0.0),
        card_flt("CRPIX2W", 0.0),
        card_flt("CRVAL1W", 0.0),
        card_flt("CRVAL2W", 0.0),
        card_int("CDELT1W", 1),
        card_int("CDELT2W", 1),
        card_int("PV1_3W", 1),
        card_int("PV2_3W", 2),
    ];
    if sip {
        c.extend([
            card_int("A_ORDER", 1),
            card_flt("A_0_0", 0.001),
            card_flt("A_1_0", 1.0e-5),
            card_flt("A_0_1", -2.0e-5),
            card_int("B_ORDER", 1),
            card_flt("B_0_0", -0.002),
            card_flt("B_1_0", 3.0e-5),
            card_flt("B_0_1", 1.0e-5),
        ]);
    }
    c
}

/// Metadata cards for the IMAGE header.
fn meta_cards() -> Vec<String> {
    vec![
        card_int("DETECTOR", 3),
        card_str("DATE-OBS", "2026-05-20T21:38:04.643"),
        card_flt("MJD-OBS", 61180.901442627),
        card_flt("MJD-BEG", 61180.901442627),
        card_flt("MJD-AVG", 61180.902108817),
        card_flt("MJD-END", 61180.902775006),
        card_flt("XPOSURE", 113.5826),
        card_flt("TELAPSE", 118.1873),
        card_str("BUNIT", "MJy / sr"),
    ]
}

/// FLAGS legend cards: bits 0, 10, 21 only (never hardcoded in the parser).
fn legend_cards() -> Vec<String> {
    vec![
        card_str("MSKN0000", "TRANSIENT"),
        card_str("MSKD0000", "Transient detected during SUR"),
        card_str("MSKN0010", "HOT"),
        card_str("MSKD0010", "Hot pixel"),
        card_str("MSKN0021", "SOURCE"),
        card_str("MSKD0021", "Pixel mapped to a known source"),
    ]
}

/// Pixel data used across tests.
fn image_pixels() -> Vec<f32> {
    let mut v = vec![1.0f32; NX * NY];
    v[0] = f32::NAN;
    v[5] = -2.5;
    v
}

fn flag_pixels() -> Vec<i32> {
    let mut v = vec![0i32; NX * NY];
    v[0] = 1 << 0 | 1 << 21; // TRANSIENT + SOURCE
    v[7] = 1 << 10; // HOT
    v[11] = -1; // all bits
    v
}

/// WCS-WAVE table data: 3x3 grid, values = (wave, band) pairs.
fn wave_grid() -> (Vec<i32>, Vec<i32>, Vec<f32>) {
    let xs = vec![1i32, 2, 3];
    let ys = vec![1i32, 2, 3];
    let mut values = Vec::new();
    for &y in &ys {
        for &x in &xs {
            // wave(x,y) = 1.0 + 0.1*(x-1) + 0.02*(y-1); band = wave/40
            let w = 1.0f32 + 0.1 * (x - 1) as f32 + 0.02 * (y - 1) as f32;
            values.push(w);
            values.push(w / 40.0);
        }
    }
    (xs, ys, values)
}

/// Assemble a complete synthetic MEF.
fn build_mef(compressed_flags: bool) -> Vec<u8> {
    let mut f = primary_hdu();
    let mut img_extra = wcs_cards(true);
    img_extra.extend(meta_cards());
    f.extend(image_hdu(
        "IMAGE",
        -32,
        NX,
        NY,
        &f32_data(&image_pixels()),
        &img_extra,
    ));
    if compressed_flags {
        f.extend(compressed_image_hdu(
            "FLAGS",
            [NX, NY],
            [NX, 1],
            &flag_pixels(),
            &[],
            &legend_cards(),
        ));
    } else {
        f.extend(image_hdu(
            "FLAGS",
            32,
            NX,
            NY,
            &i32_data(&flag_pixels()),
            &legend_cards(),
        ));
    }
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        NX,
        NY,
        &f32_data(&[0.01; NX * NY]),
        &[],
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        NX,
        NY,
        &f32_data(&[0.5; NX * NY]),
        &[],
    ));
    let (xs, ys, values) = wave_grid();
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));
    f
}

// ---------------------------------------------------------------------

#[test]
fn parses_basic_frame() {
    let frame = parse_frame(&build_mef(false)).expect("parse failed");
    assert_eq!(frame.image.dim(), (NY, NX));
    assert!(frame.image[[0, 0]].is_nan());
    assert_eq!(frame.image[[1, 1]], -2.5);
    assert_eq!(frame.variance[[0, 0]], 0.01);
    assert_eq!(frame.zodi[[2, 3]], 0.5);
    assert_eq!(frame.flags[[0, 0]], (1 << 0) | (1 << 21));
    assert_eq!(frame.flags[[2, 3]], -1);
    assert_eq!(frame.header.detector, Some(3));
    assert_eq!(frame.header.mjd_beg, Some(61180.901442627));
    assert_eq!(frame.header.mjd_end, Some(61180.902775006));
    assert_eq!(frame.header.exposure_s, Some(113.5826));
    assert_eq!(frame.header.unit.as_deref(), Some("MJy / sr"));
    assert!(!frame.header.has_epsf);

    // Legend parsed from MSKN/MSKD keywords.
    let legend = &frame.header.flag_legend;
    assert_eq!(legend[&0].name, "TRANSIENT");
    assert_eq!(
        legend[&0].description.as_deref(),
        Some("Transient detected during SUR")
    );
    assert_eq!(legend[&10].name, "HOT");
    assert_eq!(legend[&21].name, "SOURCE");
    assert_eq!(legend.len(), 3);

    // flags_to_names on the actual flag values.
    let names = flags_to_names(frame.flags[[0, 0]], legend);
    assert_eq!(names, vec!["TRANSIENT".to_string(), "SOURCE".to_string()]);
    assert_eq!(flags_to_names(1 << 10, legend), vec!["HOT".to_string()]);
}

#[test]
fn flags_to_names_handles_unknown_and_negative() {
    let mut legend = std::collections::BTreeMap::new();
    legend.insert(
        3u32,
        spherex_backend::fits::FlagBit {
            name: "FOO".into(),
            description: None,
        },
    );
    assert!(flags_to_names(0, &legend).is_empty());
    assert_eq!(flags_to_names(1 << 3, &legend), vec!["FOO".to_string()]);
    // Unknown bit 5 -> synthetic name; i32::MIN -> bit 31.
    assert_eq!(flags_to_names(i32::MIN, &legend), vec!["BIT31".to_string()]);
    let all = flags_to_names(-1, &legend);
    assert_eq!(all.len(), 32);
    assert_eq!(all[3], "FOO");
    assert_eq!(all[31], "BIT31");
}

#[test]
fn missing_extension_lists_found() {
    // Drop VARIANCE: rebuild a MEF without it.
    let mut f = primary_hdu();
    let mut img_extra = wcs_cards(false);
    img_extra.extend(meta_cards());
    f.extend(image_hdu(
        "IMAGE",
        -32,
        NX,
        NY,
        &f32_data(&image_pixels()),
        &img_extra,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        NX,
        NY,
        &i32_data(&flag_pixels()),
        &legend_cards(),
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        NX,
        NY,
        &f32_data(&[0.5; NX * NY]),
        &[],
    ));
    let (xs, ys, values) = wave_grid();
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));

    let err = parse_frame(&f).unwrap_err();
    match err {
        FitsError::MissingExtension { name, found } => {
            assert_eq!(name, "VARIANCE");
            assert!(found.contains("IMAGE"));
            assert!(found.contains("FLAGS"));
            assert!(found.contains("WCS-WAVE"));
            assert!(!found.contains("VARIANCE"));
        }
        other => panic!("expected MissingExtension, got {other:?}"),
    }
}

#[test]
fn rejects_empty_and_short_inputs() {
    assert!(matches!(parse_frame(&[]), Err(FitsError::NotFits(_))));
    assert!(matches!(
        parse_frame(&[0u8; 100]),
        Err(FitsError::NotFits(_))
    ));
    // Valid-looking start but not SIMPLE.
    assert!(matches!(
        parse_frame(&vec![b'X'; 5760]),
        Err(FitsError::NotFits(_))
    ));
}

#[test]
fn truncated_file_errors_not_panics() {
    let mef = build_mef(false);
    // Truncate mid-way through the FLAGS image data.
    let cut = mef.len() * 3 / 4;
    let r = parse_frame(&mef[..cut]);
    assert!(r.is_err(), "truncated file parsed without error");
    // Truncate inside the first header.
    assert!(parse_frame(&mef[..1000]).is_err());
    // Only the primary HDU.
    let just_primary = primary_hdu();
    assert!(matches!(
        parse_frame(&just_primary),
        Err(FitsError::MissingExtension { .. })
    ));
}

#[test]
fn corrupt_file_errors_not_panics() {
    let mef = build_mef(false);
    // Corrupt NAXIS1 of the IMAGE HDU (first extension) to something huge.
    let mut bad = mef.clone();
    // Find the NAXIS1 card of the first extension: header 2 starts at 2880*2.
    // Simpler: corrupt declared data size via BITPIX in primary? Use garbage HDU.
    // Append a bogus extension with NAXIS product larger than file.
    let mut bogus = Vec::new();
    bogus.extend(header_block(&[
        card_str("XTENSION", "IMAGE"),
        card_int("BITPIX", -32),
        card_int("NAXIS", 2),
        card_int("NAXIS1", 1_000_000),
        card_int("NAXIS2", 1_000_000),
        card_int("PCOUNT", 0),
        card_int("GCOUNT", 1),
        card_str("EXTNAME", "ZZZ"),
    ]));
    bad.extend(bogus);
    // Parser should stop at the first corrupt HDU and report, not panic.
    assert!(parse_frame(&bad).is_err());
}

#[test]
fn wcs_reference_and_roundtrip() {
    let frame = parse_frame(&build_mef(true)).expect("parse failed");
    let w = &frame.wcs;
    // At crpix, radec == crval modulo the SIP constant term (here A_0_0=0.001px).
    let (ra, dec) = w.pixel_to_radec(2.0, 2.0);
    assert!((ra - 150.0).abs() < 1e-4, "ra {ra}");
    assert!((dec - 2.0).abs() < 1e-4, "dec {dec}");
    // Round trip through the (iterative) SIP inversion.
    for &(x, y) in &[(1.0, 1.0), (3.7, 2.4), (2.0, 2.0)] {
        let (ra, dec) = w.pixel_to_radec(x, y);
        let (xp, yp) = w.radec_to_pixel(ra, dec).expect("inverse failed");
        assert!((xp - x).abs() < 1e-6, "x {xp} vs {x}");
        assert!((yp - y).abs() < 1e-6, "y {yp} vs {y}");
    }
    // RA wraps into [0, 360).
    let (ra, _) = w.pixel_to_radec(1.0, 1.0);
    assert!((0.0..360.0).contains(&ra));
}

#[test]
fn radec_to_pixel_rejects_behind_plane() {
    let frame = parse_frame(&build_mef(false)).expect("parse failed");
    // Antipode of crval is behind the tangent plane.
    assert!(frame.wcs.radec_to_pixel(330.0, -2.0).is_err());
}

#[test]
fn unsupported_projection_errors() {
    let mut f = primary_hdu();
    let mut img_extra = wcs_cards(false);
    img_extra[0] = card_str("CTYPE1", "RA---SIN");
    img_extra.extend(meta_cards());
    f.extend(image_hdu(
        "IMAGE",
        -32,
        NX,
        NY,
        &f32_data(&image_pixels()),
        &img_extra,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        NX,
        NY,
        &i32_data(&flag_pixels()),
        &legend_cards(),
    ));
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        NX,
        NY,
        &f32_data(&[0.01; NX * NY]),
        &[],
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        NX,
        NY,
        &f32_data(&[0.5; NX * NY]),
        &[],
    ));
    let (xs, ys, values) = wave_grid();
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));
    assert!(matches!(
        parse_frame(&f),
        Err(FitsError::UnsupportedProjection(_))
    ));
}

#[test]
fn compressed_flags_decode_correctly() {
    // Tiles of 4x1 -> 3 tiles; exercise also a raw-fallback tile.
    let pixels = flag_pixels();
    let mut f = primary_hdu();
    let mut img_extra = wcs_cards(false);
    img_extra.extend(meta_cards());
    f.extend(image_hdu(
        "IMAGE",
        -32,
        NX,
        NY,
        &f32_data(&image_pixels()),
        &img_extra,
    ));
    // Tile 1 stored raw via UNCOMPRESSED_DATA.
    f.extend(compressed_image_hdu(
        "FLAGS",
        [NX, NY],
        [NX, 1],
        &pixels,
        &[1],
        &legend_cards(),
    ));
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        NX,
        NY,
        &f32_data(&[0.01; NX * NY]),
        &[],
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        NX,
        NY,
        &f32_data(&[0.5; NX * NY]),
        &[],
    ));
    let (xs, ys, values) = wave_grid();
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));

    let frame = parse_frame(&f).expect("compressed parse failed");
    for y in 0..NY {
        for x in 0..NX {
            assert_eq!(
                frame.flags[[y, x]],
                pixels[y * NX + x],
                "mismatch at ({x},{y})"
            );
        }
    }
}

#[test]
fn wavelength_bilinear_lookup() {
    let frame = parse_frame(&build_mef(false)).expect("parse failed");
    let wm = &frame.wavelength;
    // Control points: wave = 1.0 + 0.1*(x-1) + 0.02*(y-1).
    for &(x, y, want) in &[(1.0, 1.0, 1.0), (3.0, 3.0, 1.24), (2.0, 2.0, 1.12)] {
        let got = wm.wavelength_at(x, y);
        assert!((got - want).abs() < 1e-6, "at ({x},{y}): {got} vs {want}");
    }
    // Midpoint between control points = average of the four corners of
    // cell (1,1)-(2,2): mean of 1.0, 1.1, 1.02, 1.12 = 1.06.
    assert!((wm.wavelength_at(1.5, 1.5) - 1.06).abs() < 1e-6);
    // Bandwidth = wave/40.
    assert!((wm.bandwidth_at(2.0, 2.0) - 1.12 / 40.0).abs() < 1e-6);
}

#[test]
fn detector_as_string() {
    let mut f = primary_hdu();
    let mut img_extra = wcs_cards(false);
    img_extra.push(card_str("DETECTOR", "4"));
    f.extend(image_hdu(
        "IMAGE",
        -32,
        NX,
        NY,
        &f32_data(&image_pixels()),
        &img_extra,
    ));
    f.extend(image_hdu(
        "FLAGS",
        32,
        NX,
        NY,
        &i32_data(&flag_pixels()),
        &legend_cards(),
    ));
    f.extend(image_hdu(
        "VARIANCE",
        -32,
        NX,
        NY,
        &f32_data(&[0.01; NX * NY]),
        &[],
    ));
    f.extend(image_hdu(
        "ZODI",
        -32,
        NX,
        NY,
        &f32_data(&[0.5; NX * NY]),
        &[],
    ));
    let (xs, ys, values) = wave_grid();
    f.extend(wcs_wave_hdu(&xs, &ys, &values, &[]));
    let frame = parse_frame(&f).expect("parse failed");
    assert_eq!(frame.header.detector, Some(4));
}

/// Opt-in integration test against a real IRSA cutout stored locally.
/// Run: `SPHEREX_FITS_REAL=/path/cutout.fits cargo test real_file`.
#[test]
fn real_file_if_provided() {
    let path = match std::env::var("SPHEREX_FITS_REAL") {
        Ok(p) => p,
        Err(_) => return,
    };
    let bytes = std::fs::read(&path).expect("cannot read fixture file");
    let frame = parse_frame(&bytes).expect("real file failed to parse");
    assert!(frame.header.detector.is_some());
    assert!(frame.header.mjd_beg.is_some() || frame.header.mjd_obs.is_some());
    assert!(!frame.header.flag_legend.is_empty());
    let (ny, nx) = frame.image.dim();
    assert!(ny > 0 && nx > 0);
    assert_eq!(frame.flags.dim(), (ny, nx));
    // WCS round trip at the centre pixel.
    let (ra, dec) = frame.wcs.pixel_to_radec(nx as f64 / 2.0, ny as f64 / 2.0);
    eprintln!("center radec: {ra} {dec}");
    let (x, y) = frame.wcs.radec_to_pixel(ra, dec).expect("inverse failed");
    eprintln!(
        "roundtrip px: {x} {y} want {} {}",
        nx as f64 / 2.0,
        ny as f64 / 2.0
    );
    assert!((x - nx as f64 / 2.0).abs() < 1e-4);
    assert!((y - ny as f64 / 2.0).abs() < 1e-4);
    let w = frame
        .wavelength
        .wavelength_at(nx as f64 / 2.0, ny as f64 / 2.0);
    let bw = frame
        .wavelength
        .bandwidth_at(nx as f64 / 2.0, ny as f64 / 2.0);
    eprintln!("lambda at center: {w} +- {bw} um");
    assert!(w.is_finite() && w > 0.5 && w < 5.5, "wavelength {w}");
}

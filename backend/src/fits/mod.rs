//! SPHEREx multi-extension FITS (MEF) parsing.
//!
//! Parses an in-memory cutout (`&[u8]`) into a [`Frame`]: IMAGE, FLAGS,
//! VARIANCE, ZODI and WCS-WAVE extensions are required; EPSF is optional.
//! Extensions are looked up by `EXTNAME`, never by position.
//!
//! All pixel inputs/outputs use 1-based FITS pixel coordinates (pixel
//! `(1,1)` is the first array element), matching the headers verbatim.
//! `Array2` images are indexed `[y, x]` (row-major, FITS first axis = x).
//!
//! Note: `crate::error::AppError` is owned by task b1; this module returns
//! [`FitsError`], which the API layer should map into `AppError::Fits`.

mod compimg;
mod error;
mod hdu;
mod header;
mod rice;
mod table;
mod wave;
mod wcs;

use std::collections::BTreeMap;

use ndarray::Array2;

pub use error::FitsError;
pub use wave::WaveMap;
pub use wcs::SkyWcs;

use hdu::Hdu;
use header::{CardValue, Header};

/// One entry of the FLAGS bit legend.
#[derive(Debug, Clone, PartialEq)]
pub struct FlagBit {
    /// Bit name, e.g. `"TRANSIENT"` (from `MSKNnnnn`).
    pub name: String,
    /// Human description (from `MSKDnnnn`), when present.
    pub description: Option<String>,
}

/// Bit number -> flag info, parsed from `MSKNnnnn`/`MSKDnnnn` keywords of
/// the FLAGS header. Bit meanings are never hardcoded.
pub type FlagLegend = BTreeMap<u32, FlagBit>;

/// Names of the flag bits set in `value`, in ascending bit order.
/// Bits with no legend entry are reported as `"BIT{n}"`.
pub fn flags_to_names(value: i32, legend: &FlagLegend) -> Vec<String> {
    let mut names = Vec::new();
    let bits = value as u32;
    for bit in 0..32 {
        if bits & (1u32 << bit) != 0 {
            match legend.get(&bit) {
                Some(b) => names.push(b.name.clone()),
                None => names.push(format!("BIT{bit}")),
            }
        }
    }
    names
}

/// Header metadata carried by a parsed frame.
#[derive(Debug, Clone, Default)]
pub struct FrameMeta {
    /// `DETECTOR` keyword (1-6), from the IMAGE header (primary as
    /// fallback). `None` when absent.
    pub detector: Option<u8>,
    /// `MJD-OBS` (observation time).
    pub mjd_obs: Option<f64>,
    /// `MJD-BEG` (start of observation).
    pub mjd_beg: Option<f64>,
    /// `MJD-AVG` (midpoint).
    pub mjd_avg: Option<f64>,
    /// `MJD-END` (end of observation).
    pub mjd_end: Option<f64>,
    /// `DATE-OBS` ISO-8601 string.
    pub date_obs: Option<String>,
    /// `DATE-BEG` ISO-8601 string.
    pub date_beg: Option<String>,
    /// `DATE-END` ISO-8601 string.
    pub date_end: Option<String>,
    /// `XPOSURE`/`EXPTIME` seconds.
    pub exposure_s: Option<f64>,
    /// `TELAPSE` seconds.
    pub telapse_s: Option<f64>,
    /// `BUNIT` of the IMAGE extension (e.g. `"MJy / sr"`).
    pub unit: Option<String>,
    /// Flag legend from the FLAGS header (`MSKNnnnn`/`MSKDnnnn`).
    pub flag_legend: FlagLegend,
    /// True when an `EPSF` extension is present (contents not decoded).
    pub has_epsf: bool,
}

/// A parsed SPHEREx cutout frame.
#[derive(Debug)]
pub struct Frame {
    /// Calibrated flux, MJy/sr (`Array2[y][x]`). NaN preserved.
    pub image: Array2<f32>,
    /// Variance, (MJy/sr)^2.
    pub variance: Array2<f32>,
    /// Model zodiacal background, MJy/sr.
    pub zodi: Array2<f32>,
    /// Per-pixel flag bitmask.
    pub flags: Array2<i32>,
    /// Celestial WCS (TAN, with SIP when present).
    pub wcs: SkyWcs,
    /// Per-pixel wavelength/bandwidth map (`WCS-WAVE`).
    pub wavelength: WaveMap,
    /// Header metadata (detector, times, flag legend).
    pub header: FrameMeta,
}

/// Parse a SPHEREx cutout MEF from `bytes`.
pub fn parse_frame(bytes: &[u8]) -> Result<Frame, FitsError> {
    let hdus = hdu::parse_hdus(bytes)?;

    let ext = |name: &'static str| -> Result<&Hdu, FitsError> {
        find_ext(&hdus, name).ok_or_else(|| FitsError::MissingExtension {
            name,
            found: extname_list(&hdus),
        })
    };

    let hdu_image = ext("IMAGE")?;
    let hdu_flags = ext("FLAGS")?;
    let hdu_var = ext("VARIANCE")?;
    let hdu_zodi = ext("ZODI")?;
    let hdu_wave = ext("WCS-WAVE")?;

    let (image, dims) = read_f32_image(hdu_image, bytes, "IMAGE")?;
    let (variance, vd) = read_f32_image(hdu_var, bytes, "VARIANCE")?;
    let (zodi, zd) = read_f32_image(hdu_zodi, bytes, "ZODI")?;
    if vd != dims || zd != dims {
        return Err(FitsError::BadData {
            ext: "VARIANCE/ZODI".into(),
            what: format!("dimensions {vd:?}/{zd:?} do not match IMAGE {dims:?}"),
        });
    }
    let flags = read_flags(hdu_flags, bytes, &dims)?;

    let wcs = wcs::SkyWcs::from_header(&hdu_image.header)?;
    let wavelength = wave::WaveMap::new(hdu_wave, bytes, &hdu_image.header)?;

    let primary = hdus.first().map(|h| &h.header);
    let meta = frame_meta(&hdu_image.header, primary, &hdu_flags.header);
    let meta = FrameMeta {
        has_epsf: find_ext(&hdus, "EPSF").is_some(),
        ..meta
    };

    Ok(Frame {
        image,
        variance,
        zodi,
        flags,
        wcs,
        wavelength,
        header: meta,
    })
}

/// Find an HDU by EXTNAME (trim + case-insensitive).
fn find_ext<'h>(hdus: &'h [Hdu], name: &str) -> Option<&'h Hdu> {
    hdus.iter().find(|h| {
        h.extname()
            .is_some_and(|e| e.trim().eq_ignore_ascii_case(name))
    })
}

/// Sorted list of the EXTNAMEs present (for error messages).
fn extname_list(hdus: &[Hdu]) -> String {
    let mut names: Vec<String> = hdus.iter().filter_map(|h| h.extname()).collect();
    names.sort();
    names.join(", ")
}

/// Decode an uncompressed or tile-compressed image HDU of ints to f32
/// plus its `[nx, ny]` dims.
fn read_f32_image(
    hdu: &Hdu,
    bytes: &[u8],
    ext: &'static str,
) -> Result<(Array2<f32>, [usize; 2]), FitsError> {
    if hdu.is_compressed_image() {
        let img = compimg::decompress(hdu, bytes)?;
        let dims = check_dims(&img.dims, ext)?;
        let data: Vec<f32> = match img.pixels {
            compimg::Pixels::Int(v) => v.into_iter().map(|x| x as f32).collect(),
            compimg::Pixels::Float(v) => v.into_iter().map(|x| x as f32).collect(),
        };
        let arr = to_array2(dims, data, ext)?;
        return Ok((arr, dims));
    }
    let bitpix = hdu.header.req_i64("BITPIX")?;
    let dims = image_dims(hdu, ext)?;
    let raw = &bytes[hdu.data.clone()];
    let npix = dims[0] * dims[1];
    let data: Vec<f32> = match bitpix {
        -32 => decode_be(raw, npix, 4, |c| {
            f32::from_be_bytes([c[0], c[1], c[2], c[3]])
        })?,
        -64 => decode_be(raw, npix, 8, |c| {
            f64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f32
        })?,
        8 => decode_be(raw, npix, 1, |c| f32::from(c[0]))?,
        16 => decode_be(raw, npix, 2, |c| i16::from_be_bytes([c[0], c[1]]) as f32)?,
        32 => decode_be(raw, npix, 4, |c| {
            i32::from_be_bytes([c[0], c[1], c[2], c[3]]) as f32
        })?,
        other => {
            return Err(FitsError::BadData {
                ext: ext.into(),
                what: format!("unsupported BITPIX {other}"),
            })
        }
    };
    // BLANK maps to NaN for integer arrays; BSCALE/BZERO applied if present.
    let data = apply_scaling(hdu, data);
    let arr = to_array2(dims, data, ext)?;
    Ok((arr, dims))
}

/// Decode FLAGS: uncompressed BITPIX=32 image, or tile-compressed (ZIMAGE).
fn read_flags(hdu: &Hdu, bytes: &[u8], dims: &[usize; 2]) -> Result<Array2<i32>, FitsError> {
    if hdu.is_compressed_image() {
        let img = compimg::decompress(hdu, bytes)?;
        let d = check_dims(&img.dims, "FLAGS")?;
        if d != *dims {
            return Err(FitsError::BadData {
                ext: "FLAGS".into(),
                what: format!("dimensions {d:?} do not match IMAGE {dims:?}"),
            });
        }
        let data: Vec<i32> = match img.pixels {
            compimg::Pixels::Int(v) => v
                .into_iter()
                .map(|x| i32::try_from(x).unwrap_or(i32::MAX))
                .collect(),
            compimg::Pixels::Float(v) => v.into_iter().map(|x| x as i32).collect(),
        };
        return to_array2(d, data, "FLAGS");
    }
    let bitpix = hdu.header.req_i64("BITPIX")?;
    let d = image_dims(hdu, "FLAGS")?;
    if d != *dims {
        return Err(FitsError::BadData {
            ext: "FLAGS".into(),
            what: format!("dimensions {d:?} do not match IMAGE {dims:?}"),
        });
    }
    let raw = &bytes[hdu.data.clone()];
    let npix = d[0] * d[1];
    let data: Vec<i32> = match bitpix {
        32 => decode_be(raw, npix, 4, |c| {
            i32::from_be_bytes([c[0], c[1], c[2], c[3]])
        })?,
        16 => decode_be(raw, npix, 2, |c| {
            i32::from(i16::from_be_bytes([c[0], c[1]]))
        })?,
        8 => decode_be(raw, npix, 1, |c| i32::from(c[0]))?,
        other => {
            return Err(FitsError::BadData {
                ext: "FLAGS".into(),
                what: format!("unsupported BITPIX {other}"),
            })
        }
    };
    to_array2(d, data, "FLAGS")
}

/// `[NAXIS1, NAXIS2]` of a 2-D image HDU.
fn image_dims(hdu: &Hdu, ext: &str) -> Result<[usize; 2], FitsError> {
    let naxis = hdu.header.req_i64("NAXIS")?;
    if naxis != 2 {
        return Err(FitsError::BadData {
            ext: ext.into(),
            what: format!("expected NAXIS=2, got {naxis}"),
        });
    }
    let nx = hdu.header.req_i64("NAXIS1")?;
    let ny = hdu.header.req_i64("NAXIS2")?;
    if nx <= 0 || ny <= 0 {
        return Err(FitsError::BadData {
            ext: ext.into(),
            what: format!("non-positive dimensions {nx}x{ny}"),
        });
    }
    Ok([nx as usize, ny as usize])
}

/// Validate decompressed dims (must be 2-D, positive).
fn check_dims(dims: &[usize], ext: &str) -> Result<[usize; 2], FitsError> {
    if dims.len() != 2 || dims[0] == 0 || dims[1] == 0 {
        return Err(FitsError::BadData {
            ext: ext.into(),
            what: format!("bad decompressed dimensions {dims:?}"),
        });
    }
    Ok([dims[0], dims[1]])
}

/// Big-endian element decoding with bounds check.
fn decode_be<T>(
    raw: &[u8],
    npix: usize,
    elem: usize,
    f: impl Fn(&[u8]) -> T,
) -> Result<Vec<T>, FitsError> {
    let need = npix
        .checked_mul(elem)
        .ok_or(FitsError::Corrupt("pixel count overflow".into()))?;
    if raw.len() < need {
        return Err(FitsError::Truncated("image pixel data"));
    }
    Ok(raw[..need].chunks_exact(elem).map(f).collect())
}

/// Apply BSCALE/BZERO to integer-decoded data; pass floats through.
fn apply_scaling(hdu: &Hdu, mut data: Vec<f32>) -> Vec<f32> {
    let scale = hdu.header.opt_f64("BSCALE").unwrap_or(1.0);
    let zero = hdu.header.opt_f64("BZERO").unwrap_or(0.0);
    if scale != 1.0 || zero != 0.0 {
        for v in &mut data {
            *v = *v * scale as f32 + zero as f32;
        }
    }
    data
}

/// Build `Array2` of shape (ny, nx) from first-axis-fastest data.
fn to_array2<T: Clone>(dims: [usize; 2], data: Vec<T>, ext: &str) -> Result<Array2<T>, FitsError> {
    let (nx, ny) = (dims[0], dims[1]);
    if data.len() < nx * ny {
        return Err(FitsError::Truncated("image pixel data"));
    }
    Array2::from_shape_vec((ny, nx), data[..nx * ny].to_vec())
        .map_err(|_| FitsError::Corrupt(format!("{ext}: array shape mismatch")))
}

/// Collect FrameMeta from the IMAGE header (+ primary fallback + FLAGS for
/// the legend).
fn frame_meta(image_h: &Header, primary: Option<&Header>, flags_h: &Header) -> FrameMeta {
    let get_f64 = |k: &str| {
        image_h
            .opt_f64(k)
            .or_else(|| primary.and_then(|p| p.opt_f64(k)))
    };
    let get_str = |k: &str| {
        image_h
            .opt_str(k)
            .or_else(|| primary.and_then(|p| p.opt_str(k)))
    };
    let detector = image_h
        .opt_detector()
        .or_else(|| primary.and_then(|p| p.opt_detector()));
    FrameMeta {
        detector,
        mjd_obs: get_f64("MJD-OBS"),
        mjd_beg: get_f64("MJD-BEG"),
        mjd_avg: get_f64("MJD-AVG"),
        mjd_end: get_f64("MJD-END"),
        date_obs: get_str("DATE-OBS"),
        date_beg: get_str("DATE-BEG"),
        date_end: get_str("DATE-END"),
        exposure_s: get_f64("XPOSURE").or_else(|| get_f64("EXPTIME")),
        telapse_s: get_f64("TELAPSE"),
        unit: image_h.opt_str("BUNIT"),
        flag_legend: parse_flag_legend(flags_h),
        has_epsf: false,
    }
}

/// Parse `MSKNnnnn`/`MSKDnnnn` keywords of the FLAGS header into a legend.
fn parse_flag_legend(h: &Header) -> FlagLegend {
    let mut legend = FlagLegend::new();
    for key in h.keys() {
        if let Some(bit) = key.strip_prefix("MSKN").and_then(|s| s.parse::<u32>().ok()) {
            if let Some(CardValue::Str(name)) = h.get(key) {
                let desc_key = format!("MSKD{bit:04}");
                let description = h
                    .get(&desc_key)
                    .and_then(|v| v.as_str().map(str::to_string));
                legend.insert(
                    bit,
                    FlagBit {
                        name: name.trim().to_string(),
                        description: description.map(|d| d.trim().to_string()),
                    },
                );
            }
        }
    }
    legend
}

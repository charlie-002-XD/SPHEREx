//! Query-parameter decoding for the API routes (task b5).
//!
//! Every failure is an [`AppError::InvalidParameter`] whose message
//! describes the violated constraint and never echoes the raw input.
//! Coordinates go through the b1 grammar (`parse_coordinates`) and the
//! range check (`validate_query_params`).

use std::collections::HashMap;

use axum::extract::Query;
use axum::http::Uri;

use crate::api::validation::{parse_coordinates, validate_query_params, Query as SkyQuery};
use crate::error::AppError;

use super::dto::Mode;

/// Contract defaults for `/frames`.
const DEFAULT_FOV_DEG: f64 = 0.05;
const DEFAULT_MAX_FRAMES: usize = 24;
const MAX_FRAMES_LIMIT: usize = 60;

/// Validated `/api/v1/frames` parameters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FramesParams {
    /// `ra` resolved to degrees.
    pub ra_deg: f64,
    /// `dec` resolved to degrees.
    pub dec_deg: f64,
    /// `fov`, degrees (0.01-0.5; default 0.05).
    pub fov_deg: f64,
    /// `mode` (default `raw`).
    pub mode: Mode,
    /// Optional `detector` restriction (1-6).
    pub detector: Option<u8>,
    /// `max_frames` (1-60; default 24).
    pub max_frames: usize,
    /// `subtract_zodi` (default false).
    pub subtract_zodi: bool,
}

/// Validated `/api/v1/spectrum` parameters.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SpectrumParams {
    /// `ra` resolved to degrees.
    pub ra_deg: f64,
    /// `dec` resolved to degrees.
    pub dec_deg: f64,
    /// `aperture_px` (one of 1, 3, 5; default 3).
    pub aperture_px: u32,
}

/// Parse and validate the `/frames` query string.
pub(crate) fn frames_params(uri: &Uri) -> Result<FramesParams, AppError> {
    let m = query_map(uri)?;
    let (ra, dec) = coordinates(&m)?;
    let fov = opt_f64(&m, "fov", DEFAULT_FOV_DEG)?;
    let sky: SkyQuery = validate_query_params(ra, dec, fov)?;

    let mode = match m.get("mode") {
        None => Mode::Raw,
        Some(s) => match s.trim().to_ascii_lowercase().as_str() {
            "raw" => Mode::Raw,
            "diff" => Mode::Diff,
            "coadd" => Mode::Coadd,
            "stack" => Mode::Stack,
            _ => {
                return Err(AppError::invalid_parameter(
                    "mode: expected one of raw, diff, coadd, stack",
                ))
            }
        },
    };

    let detector =
        opt_bounded(&m, "detector", None, 1, 6, "expected an integer in [1, 6]")?.map(|d| d as u8);
    let max_frames = opt_bounded(
        &m,
        "max_frames",
        Some(DEFAULT_MAX_FRAMES),
        1,
        MAX_FRAMES_LIMIT,
        "expected an integer in [1, 60]",
    )?
    .unwrap_or(DEFAULT_MAX_FRAMES);
    let subtract_zodi = opt_bool(&m, "subtract_zodi", false)?;

    Ok(FramesParams {
        ra_deg: sky.ra_deg,
        dec_deg: sky.dec_deg,
        fov_deg: sky.fov_deg,
        mode,
        detector,
        max_frames,
        subtract_zodi,
    })
}

/// Parse and validate the `/spectrum` query string.
pub(crate) fn spectrum_params(uri: &Uri) -> Result<SpectrumParams, AppError> {
    let m = query_map(uri)?;
    let (ra, dec) = coordinates(&m)?;
    let aperture = opt_bounded(
        &m,
        "aperture_px",
        Some(3),
        1,
        5,
        "expected an integer in {1, 3, 5}",
    )?
    .unwrap_or(3);
    if !matches!(aperture, 1 | 3 | 5) {
        return Err(AppError::invalid_parameter(
            "aperture_px: expected one of 1, 3, 5",
        ));
    }
    Ok(SpectrumParams {
        ra_deg: ra,
        dec_deg: dec,
        aperture_px: aperture as u32,
    })
}

/// URL-decoded query string as a name -> value map (duplicate keys keep
/// the last occurrence).
fn query_map(uri: &Uri) -> Result<HashMap<String, String>, AppError> {
    Query::<HashMap<String, String>>::try_from_uri(uri)
        .map(|Query(m)| m)
        .map_err(|_| AppError::invalid_parameter("malformed query string"))
}

/// Required `ra` + `dec` pair through the b1 grammar.
fn coordinates(m: &HashMap<String, String>) -> Result<(f64, f64), AppError> {
    let ra = required(m, "ra")?;
    let dec = required(m, "dec")?;
    parse_coordinates(ra, dec)
}

fn required<'m>(m: &'m HashMap<String, String>, name: &'static str) -> Result<&'m str, AppError> {
    m.get(name)
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            AppError::invalid_parameter(format!("{name}: required parameter is missing"))
        })
}

/// Optional finite `f64`.
fn opt_f64(m: &HashMap<String, String>, name: &'static str, default: f64) -> Result<f64, AppError> {
    match m.get(name) {
        None => Ok(default),
        Some(s) => s
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| {
                AppError::invalid_parameter(format!("{name}: expected a finite number"))
            }),
    }
}

/// Optional integer in `[min, max]`; `default` applies when absent.
fn opt_bounded(
    m: &HashMap<String, String>,
    name: &'static str,
    default: Option<usize>,
    min: usize,
    max: usize,
    hint: &'static str,
) -> Result<Option<usize>, AppError> {
    let bad = || AppError::invalid_parameter(format!("{name}: {hint}"));
    match m.get(name) {
        None => Ok(default),
        Some(s) => {
            let v: usize = s.trim().parse().map_err(|_| bad())?;
            if (min..=max).contains(&v) {
                Ok(Some(v))
            } else {
                Err(bad())
            }
        }
    }
}

/// Optional boolean (`true/false`, `1/0` accepted, case-insensitive).
fn opt_bool(
    m: &HashMap<String, String>,
    name: &'static str,
    default: bool,
) -> Result<bool, AppError> {
    match m.get(name) {
        None => Ok(default),
        Some(s) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(AppError::invalid_parameter(format!(
                "{name}: expected a boolean (true/false)"
            ))),
        },
    }
}

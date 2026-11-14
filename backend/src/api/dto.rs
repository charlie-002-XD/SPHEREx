//! Serializable response bodies matching `shared/openapi-contract.json`
//! one-to-one. Non-finite `f64` values serialise as `null` (serde_json);
//! they only occur when upstream data lacks the field.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::science::LinearTanWcs;

/// `GET /api/v1/health` -> `Health`.
#[derive(Debug, Serialize)]
pub(crate) struct HealthBody {
    /// Always `"ok"`.
    pub status: &'static str,
    /// Crate version.
    pub version: &'static str,
    /// Configured SIA2 collections.
    pub collections: Vec<String>,
}

/// `GET /api/v1/targets` -> `Target` (array element).
#[derive(Debug, Clone, Serialize)]
pub struct TargetDto {
    /// Stable machine id (e.g. `"wise-0855"`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Right ascension, degrees.
    pub ra_deg: f64,
    /// Declination, degrees.
    pub dec_deg: f64,
    /// Suggested field of view, degrees.
    pub fov_deg: f64,
    /// Optional human note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The `mode` enum of `/frames`: raw | diff | coadd | stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    /// Reprojected cutouts as fetched.
    Raw,
    /// Each frame minus the first frame of its detector x wavelength
    /// group.
    Diff,
    /// One inverse-variance co-addition per group.
    Coadd,
    /// One inverse-variance stack per detector array.
    Stack,
}

/// Linear TAN WCS echo (`Wcs` schema), straight from [`LinearTanWcs`].
#[derive(Debug, Serialize)]
pub(crate) struct WcsDto {
    /// 1-based reference pixel `[x, y]`.
    pub crpix: [f64; 2],
    /// `[ra_deg, dec_deg]` at `crpix`.
    pub crval: [f64; 2],
    /// Degrees per pixel, `[[cd11, cd12], [cd21, cd22]]`.
    pub cd: [[f64; 2]; 2],
}

impl From<LinearTanWcs> for WcsDto {
    fn from(w: LinearTanWcs) -> Self {
        Self {
            crpix: w.crpix,
            crval: w.crval,
            cd: w.cd,
        }
    }
}

/// `Frame` schema element of `FramesResponse.frames`.
#[derive(Debug, Serialize)]
pub(crate) struct FrameDto {
    /// Stable id (hash of the access URL; derived hash for computed
    /// rasters).
    pub id: String,
    /// Exposure start, MJD.
    pub obs_mjd_start: f64,
    /// Exposure end, MJD.
    pub obs_mjd_end: f64,
    /// Detector array 1-6.
    pub detector: u8,
    /// Representative central wavelength (um) at the raster centre.
    pub wavelength_um: f64,
    /// Spectral bandwidth (um) when meaningful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_um: Option<f64>,
    /// Source IRSA collection(s) (`+`-joined when combined).
    pub collection: String,
    /// True when the ZODI model was subtracted from `data_b64`.
    pub zodi_subtracted: bool,
    /// Flag-bit names set in the central 3x3 pixels (FITS FLAGS legend).
    pub flags: Vec<String>,
    /// Output raster WCS.
    pub wcs: WcsDto,
    /// `width*height` float32 LE, row-major, base64.
    pub data_b64: String,
}

/// `query` echo object inside `FramesResponse`.
#[derive(Debug, Serialize)]
pub(crate) struct QueryEcho {
    /// Resolved right ascension, degrees.
    pub ra_deg: f64,
    /// Resolved declination, degrees.
    pub dec_deg: f64,
    /// Field of view used, degrees.
    pub fov_deg: f64,
    /// Composition mode applied.
    pub mode: Mode,
}

/// `FramesResponse` schema.
#[derive(Debug, Serialize)]
pub(crate) struct FramesResponseDto {
    /// Resolved query.
    pub query: QueryEcho,
    /// Raster width (pixels).
    pub width: usize,
    /// Raster height (pixels).
    pub height: usize,
    /// Pixel units (`BUNIT` of the source cutouts, e.g. `"MJy/sr"`).
    pub unit: String,
    /// Frames sorted by observation time.
    pub frames: Vec<FrameDto>,
    /// Bit number (as string) -> description, from the FITS FLAGS legend.
    pub flag_legend: BTreeMap<String, String>,
    /// True when more observations exist than were delivered.
    pub truncated: bool,
    /// Observations covering the position before any cap.
    pub total_observations: usize,
}

/// `SpectrumPoint` schema element.
#[derive(Debug, Serialize)]
pub(crate) struct SpectrumPointDto {
    /// Central wavelength (um) at the position.
    pub wavelength_um: f64,
    /// Spectral bandwidth (um) at the position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth_um: Option<f64>,
    /// Mean surface brightness in the aperture, MJy/sr.
    pub value: f64,
    /// 1-sigma uncertainty of the mean.
    pub sigma: f64,
    /// Observation epoch (MJD); `null` when the header lacks it.
    pub obs_mjd: Option<f64>,
    /// Detector array 1-6; `null` when the header lacks it.
    pub detector: Option<u8>,
    /// Flag-bit names set anywhere in the aperture.
    pub flags: Vec<String>,
}

/// `SpectrumResponse` schema.
#[derive(Debug, Serialize)]
pub(crate) struct SpectrumResponseDto {
    /// Resolved right ascension, degrees.
    pub ra_deg: f64,
    /// Resolved declination, degrees.
    pub dec_deg: f64,
    /// Units of `points[].value` (`"MJy/sr"`).
    pub unit: String,
    /// Samples sorted by wavelength.
    pub points: Vec<SpectrumPointDto>,
}

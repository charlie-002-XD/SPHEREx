//! Aperture spectrum at a sky position across a set of raw frames.
//!
//! For every frame covering `(ra, dec)` the flux is averaged over an
//! `aperture_px` square (contract: 1, 3 or 5 px) centred on the nearest
//! pixel. Wavelengths come from the per-frame WCS-WAVE map at the exact
//! position — the number of bands is data-dependent and never assumed.

use crate::fits::{flags_to_names, Frame};

use super::error::ScienceError;

/// One wavelength sample of a spectrum; mirrors `SpectrumPoint` in
/// `shared/openapi-contract.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumPoint {
    /// Central wavelength (um) at the position, from the frame's
    /// WCS-WAVE map.
    pub wavelength_um: f64,
    /// Spectral bandwidth (um) at the position.
    pub bandwidth_um: f64,
    /// Mean surface brightness in the aperture, MJy/sr (after the
    /// optional ZODI subtraction).
    pub value: f64,
    /// 1-sigma uncertainty of the aperture mean: `sqrt(sum(var_i))/N`
    /// over pixels with finite positive variance; NaN when none.
    pub sigma: f64,
    /// Observation epoch (`MJD-AVG`, else `MJD-OBS`), when known.
    pub obs_mjd: Option<f64>,
    /// Detector array 1..=6.
    pub detector: Option<u8>,
    /// Names of the flag bits set anywhere in the aperture, from the
    /// frame's FITS FLAGS legend.
    pub flags: Vec<String>,
}

/// Spectrum at `(ra_deg, dec_deg)` over `frames`, sorted by wavelength.
///
/// `aperture_px` must be a positive odd integer (the API contract
/// restricts it to 1, 3, 5). With `subtract_zodi` the measured flux is
/// `IMAGE - ZODI` (SPHEREx `IMAGE` is not zodi-subtracted); the variance
/// is left unchanged because ZODI is a model, not a measurement.
///
/// Frames that do not cover the position, or whose aperture contains no
/// finite pixels, are skipped; an empty result means "no observations"
/// (the API layer maps that to 404). `radec_to_pixel` failures
/// (point behind the tangent plane, SIP not converging) also skip the
/// frame.
pub fn extract_spectrum(
    frames: &[&Frame],
    ra_deg: f64,
    dec_deg: f64,
    aperture_px: u32,
    subtract_zodi: bool,
) -> Result<Vec<SpectrumPoint>, ScienceError> {
    if aperture_px == 0 || aperture_px.is_multiple_of(2) {
        return Err(ScienceError::InvalidAperture { aperture_px });
    }
    let half = (aperture_px / 2) as i64;
    let mut points = Vec::new();
    for frame in frames {
        let (ny, nx) = frame.image.dim();
        let Ok((px, py)) = frame.wcs.radec_to_pixel(ra_deg, dec_deg) else {
            continue;
        };
        if !(px >= 0.5 && px <= nx as f64 + 0.5 && py >= 0.5 && py <= ny as f64 + 0.5) {
            continue;
        }
        let cx = (px.round() as i64).clamp(1, nx as i64);
        let cy = (py.round() as i64).clamp(1, ny as i64);

        let (mut sum, mut n) = (0.0f64, 0u64);
        let (mut varsum, mut nvar) = (0.0f64, 0u64);
        let mut flagbits = 0i32;
        for iy in (cy - half)..=(cy + half) {
            for ix in (cx - half)..=(cx + half) {
                if !(1..=nx as i64).contains(&ix) || !(1..=ny as i64).contains(&iy) {
                    continue;
                }
                let (x, y) = ((ix - 1) as usize, (iy - 1) as usize);
                flagbits |= frame.flags[[y, x]];
                let mut v = f64::from(frame.image[[y, x]]);
                if subtract_zodi {
                    let z = frame.zodi[[y, x]];
                    if z.is_finite() {
                        v -= f64::from(z);
                    }
                }
                if v.is_finite() {
                    sum += v;
                    n += 1;
                }
                let var = frame.variance[[y, x]];
                if var.is_finite() && var > 0.0 {
                    varsum += f64::from(var);
                    nvar += 1;
                }
            }
        }
        if n == 0 {
            continue;
        }
        let wavelength_um = frame.wavelength.wavelength_at(px, py);
        if !wavelength_um.is_finite() {
            continue;
        }
        points.push(SpectrumPoint {
            wavelength_um,
            bandwidth_um: frame.wavelength.bandwidth_at(px, py),
            value: sum / n as f64,
            sigma: if nvar > 0 {
                varsum.sqrt() / n as f64
            } else {
                f64::NAN
            },
            obs_mjd: frame.header.mjd_avg.or(frame.header.mjd_obs),
            detector: frame.header.detector,
            flags: flags_to_names(flagbits, &frame.header.flag_legend),
        });
    }
    points.sort_by(|a, b| a.wavelength_um.total_cmp(&b.wavelength_um));
    Ok(points)
}

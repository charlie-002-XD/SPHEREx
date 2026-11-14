//! Output pixel grids: a linear TAN WCS over an `nx * ny` raster.
//!
//! `SkyWcs` cannot be constructed outside `crate::fits` (its SIP fields
//! are private), so the science engine uses [`LinearTanWcs`] — the same
//! gnomonic projection without distortion — for the idealized output
//! rasters requested by the API.
//!
//! Pixel coordinates are 1-based FITS coordinates, matching
//! `crate::fits`: pixel `(1, 1)` is the centre of the first array
//! element; `Array2` images are indexed `[y, x]`.

use crate::fits::SkyWcs;

use super::error::ScienceError;

const DEG: f64 = std::f64::consts::PI / 180.0;

/// Linear celestial WCS (TAN projection, no SIP) for an output raster.
///
/// Field layout matches the `Wcs` schema of
/// `shared/openapi-contract.json`, so the API layer can serialise it
/// directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearTanWcs {
    /// 1-based reference pixel `[x, y]` (`CRPIX1`, `CRPIX2`).
    pub crpix: [f64; 2],
    /// `[ra_deg, dec_deg]` at `crpix` (`CRVAL1`, `CRVAL2`).
    pub crval: [f64; 2],
    /// Degrees-per-pixel matrix `[[cd11, cd12], [cd21, cd22]]`.
    pub cd: [[f64; 2]; 2],
}

impl LinearTanWcs {
    /// `(ra_deg, dec_deg)` of 1-based pixel `(x, y)` under a TAN
    /// (gnomonic) projection.
    pub fn pixel_to_radec(&self, x: f64, y: f64) -> (f64, f64) {
        let u = x - self.crpix[0];
        let v = y - self.crpix[1];
        let xi = (self.cd[0][0] * u + self.cd[0][1] * v) * DEG;
        let eta = (self.cd[1][0] * u + self.cd[1][1] * v) * DEG;
        let (ra0, dec0) = (self.crval[0] * DEG, self.crval[1] * DEG);
        let n0 = [dec0.cos() * ra0.cos(), dec0.cos() * ra0.sin(), dec0.sin()];
        let e_ra = [-ra0.sin(), ra0.cos(), 0.0];
        let e_dec = [-dec0.sin() * ra0.cos(), -dec0.sin() * ra0.sin(), dec0.cos()];
        let d = [
            n0[0] + xi * e_ra[0] + eta * e_dec[0],
            n0[1] + xi * e_ra[1] + eta * e_dec[1],
            n0[2] + xi * e_ra[2] + eta * e_dec[2],
        ];
        let ra = d[1].atan2(d[0]).to_degrees().rem_euclid(360.0);
        let dec = (d[2] / norm(d)).asin().to_degrees();
        (ra, dec)
    }

    /// Copy the linear part (CRPIX/CRVAL/CD) of a parsed frame WCS;
    /// SIP distortion terms are dropped.
    pub fn from_sky_wcs(wcs: &SkyWcs) -> Self {
        Self {
            crpix: wcs.crpix,
            crval: wcs.crval,
            cd: wcs.cd,
        }
    }
}

/// Target raster for reprojection: `nx * ny` pixels plus a linear TAN WCS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputGrid {
    /// Number of columns (x size).
    pub nx: usize,
    /// Number of rows (y size).
    pub ny: usize,
    /// WCS of the raster.
    pub wcs: LinearTanWcs,
}

impl OutputGrid {
    /// Square `npix` x `npix` grid centred on `(ra_deg, dec_deg)`,
    /// spanning `fov_deg` on a side: north up, east left
    /// (`cd = diag(-fov/npix, +fov/npix)`), TAN projection, reference
    /// pixel at the grid centre.
    pub fn centered(
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
        npix: usize,
    ) -> Result<Self, ScienceError> {
        if npix == 0 {
            return Err(ScienceError::InvalidGrid("npix must be >= 1"));
        }
        if !(fov_deg.is_finite() && fov_deg > 0.0 && fov_deg <= 180.0) {
            return Err(ScienceError::InvalidGrid("fov_deg must be in (0, 180]"));
        }
        if !(dec_deg.is_finite() && (-90.0..=90.0).contains(&dec_deg)) {
            return Err(ScienceError::InvalidGrid("dec_deg must be in [-90, 90]"));
        }
        if !(ra_deg.is_finite() && (0.0..360.0).contains(&ra_deg)) {
            return Err(ScienceError::InvalidGrid("ra_deg must be in [0, 360)"));
        }
        let scale = fov_deg / npix as f64;
        let c = (npix as f64 + 1.0) / 2.0;
        Ok(Self {
            nx: npix,
            ny: npix,
            wcs: LinearTanWcs {
                crpix: [c, c],
                crval: [ra_deg, dec_deg],
                cd: [[-scale, 0.0], [0.0, scale]],
            },
        })
    }

    /// Grid copying the linear WCS of a parsed frame over `nx * ny`
    /// pixels (e.g. "same grid as the reference epoch").
    pub fn from_frame(nx: usize, ny: usize, wcs: &SkyWcs) -> Self {
        Self {
            nx,
            ny,
            wcs: LinearTanWcs::from_sky_wcs(wcs),
        }
    }

    /// Unit grid: identity CD, `crval = [0, 0]`, `crpix = [1, 1]`.
    /// For tests and synthetic data, not for sky work.
    pub fn identity(nx: usize, ny: usize) -> Self {
        Self {
            nx,
            ny,
            wcs: LinearTanWcs {
                crpix: [1.0, 1.0],
                crval: [0.0, 0.0],
                cd: [[1.0, 0.0], [0.0, 1.0]],
            },
        }
    }
}

fn norm(d: [f64; 3]) -> f64 {
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

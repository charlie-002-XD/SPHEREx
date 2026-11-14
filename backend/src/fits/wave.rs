//! Per-pixel wavelength/bandwidth map from the `WCS-WAVE` binary table
//! (FITS WCS Paper III `WAVE-TAB` lookup).
//!
//! `wavelength_at(x, y)` takes 1-based FITS pixel coordinates of the cutout;
//! the optional linear `CRPIXnW/CRVALnW/CDELTnW` transform maps them into
//! the table's control-point coordinate space.

use crate::fits::error::FitsError;
use crate::fits::hdu::Hdu;
use crate::fits::header::Header;
use crate::fits::table::BinTable;

/// Bilinear wavelength/bandwidth lookup built from `WCS-WAVE`.
#[derive(Debug, Clone)]
pub struct WaveMap {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Wavelength grid, `iy * nx + ix` (um).
    wave: Vec<f64>,
    /// Bandwidth grid, `iy * nx + ix` (um).
    band: Vec<f64>,
    /// Linear pixel -> table-coordinate transform (axis order: x, y).
    xform: [LinTransform; 2],
}

/// `coord = crval + (pix - crpix) * cdelt`.
#[derive(Debug, Clone, Copy)]
struct LinTransform {
    crpix: f64,
    crval: f64,
    cdelt: f64,
}

impl LinTransform {
    fn apply(&self, pix: f64) -> f64 {
        self.crval + (pix - self.crpix) * self.cdelt
    }
}

impl WaveMap {
    /// Build from the `WCS-WAVE` HDU plus the image header that carries the
    /// `-TAB` keywords (`CRPIXnW`, `PVi_3W`, ...).
    ///
    /// Expected table layout (as delivered by IRSA): one row with columns
    /// `X`, `Y` (control point coordinates) and `VALUES` (pairs of
    /// wavelength, bandwidth at each `nx*ny` grid point, x fastest).
    /// Per-row point tables `(x, y, value)` are also accepted.
    pub fn new(hdu: &Hdu, bytes: &[u8], image_header: &Header) -> Result<WaveMap, FitsError> {
        let table = BinTable::new(hdu, bytes)?;
        let col_x = table
            .column("X")
            .ok_or(FitsError::MissingKeyword("TTYPE 'X'".into()))?;
        let col_y = table
            .column("Y")
            .ok_or(FitsError::MissingKeyword("TTYPE 'Y'".into()))?;
        let col_v = table
            .column("VALUES")
            .ok_or(FitsError::MissingKeyword("TTYPE 'VALUES'".into()))?;
        if table.nrows == 0 {
            return Err(FitsError::Corrupt("WCS-WAVE table is empty".into()));
        }

        // Which element of each VALUES vector is wavelength / bandwidth.
        let wave_idx = image_header
            .opt_i64("PV1_3W")
            .map(|v| (v - 1).max(0) as usize)
            .unwrap_or(0);
        let band_idx = image_header
            .opt_i64("PV2_3W")
            .map(|v| (v - 1).max(0) as usize)
            .unwrap_or(1);

        let xs;
        let ys;
        let wave;
        let band;

        let first_x = table.cell_f64s(0, col_x)?;
        let first_y = table.cell_f64s(0, col_y)?;
        let first_v = table.cell_f64s(0, col_v)?;
        if table.nrows == 1 && first_x.len() > 1 && first_y.len() > 1 {
            // Grid layout: X = nx control x's, Y = ny control y's,
            // VALUES = nx*ny*npair values (x fastest).
            let nx = first_x.len();
            let ny = first_y.len();
            if nx * ny == 0 || first_v.len() % (nx * ny) != 0 {
                return Err(FitsError::Corrupt("WCS-WAVE VALUES length".into()));
            }
            let npair = first_v.len() / (nx * ny);
            // Sort control coordinates and permute the value grid to match.
            let xperm = sort_perm(&first_x);
            let yperm = sort_perm(&first_y);
            xs = xperm.iter().map(|&i| first_x[i]).collect();
            ys = yperm.iter().map(|&i| first_y[i]).collect();
            let mut w = vec![0.0; nx * ny];
            let mut b = vec![0.0; nx * ny];
            for j in 0..ny {
                for i in 0..nx {
                    let base = (yperm[j] * nx + xperm[i]) * npair;
                    w[j * nx + i] = first_v[base + wave_idx.min(npair - 1)];
                    b[j * nx + i] = first_v[base + band_idx.min(npair - 1)];
                }
            }
            wave = w;
            band = b;
        } else {
            // One point per row: (x_i, y_i, values_i).
            let n = table.nrows;
            if n < 4 {
                return Err(FitsError::Corrupt(
                    "WCS-WAVE table has too few points".into(),
                ));
            }
            let mut pts = Vec::with_capacity(n);
            for r in 0..n {
                let px = table.cell_f64s(r, col_x)?.first().copied().unwrap_or(0.0);
                let py = table.cell_f64s(r, col_y)?.first().copied().unwrap_or(0.0);
                let v = table.cell_f64s(r, col_v)?;
                pts.push((px, py, v));
            }
            let mut ux: Vec<f64> = pts.iter().map(|p| p.0).collect();
            let mut uy: Vec<f64> = pts.iter().map(|p| p.1).collect();
            ux = sorted(ux);
            uy = sorted(uy);
            let nx = ux.len();
            let ny = uy.len();
            if nx * ny != n {
                return Err(FitsError::Corrupt(
                    "WCS-WAVE points do not form a grid".into(),
                ));
            }
            let mut w = vec![f64::NAN; nx * ny];
            let mut b = vec![f64::NAN; nx * ny];
            for (px, py, v) in pts {
                let ix = ux.partition_point(|c| *c < px);
                let iy = uy.partition_point(|c| *c < py);
                let npair = v.len().max(1);
                w[iy * nx + ix] = v[wave_idx.min(npair - 1)];
                b[iy * nx + ix] = v[band_idx.min(npair - 1)];
            }
            xs = ux;
            ys = uy;
            wave = w;
            band = b;
        }

        if xs.len() < 2 || ys.len() < 2 {
            return Err(FitsError::Corrupt("WCS-WAVE grid smaller than 2x2".into()));
        }
        let xform = [
            LinTransform {
                crpix: image_header.opt_f64("CRPIX1W").unwrap_or(0.0),
                crval: image_header.opt_f64("CRVAL1W").unwrap_or(0.0),
                cdelt: image_header.opt_f64("CDELT1W").unwrap_or(1.0),
            },
            LinTransform {
                crpix: image_header.opt_f64("CRPIX2W").unwrap_or(0.0),
                crval: image_header.opt_f64("CRVAL2W").unwrap_or(0.0),
                cdelt: image_header.opt_f64("CDELT2W").unwrap_or(1.0),
            },
        ];
        Ok(WaveMap {
            xs,
            ys,
            wave,
            band,
            xform,
        })
    }

    /// Central wavelength (um) at 1-based pixel `(x, y)`; bilinear,
    /// clamped to the grid edges.
    pub fn wavelength_at(&self, x: f64, y: f64) -> f64 {
        self.interp(&self.wave, x, y)
    }

    /// Bandwidth (um) at 1-based pixel `(x, y)`; bilinear, clamped.
    pub fn bandwidth_at(&self, x: f64, y: f64) -> f64 {
        self.interp(&self.band, x, y)
    }

    fn interp(&self, grid: &[f64], x: f64, y: f64) -> f64 {
        let xt = self.xform[0].apply(x);
        let yt = self.xform[1].apply(y);
        let nx = self.xs.len();
        let ny = self.ys.len();
        if nx < 2 || ny < 2 {
            return f64::NAN;
        }
        let (ix, fx) = bracket(&self.xs, xt);
        let (iy, fy) = bracket(&self.ys, yt);
        let ix1 = (ix + 1).min(nx - 1);
        let iy1 = (iy + 1).min(ny - 1);
        let v00 = grid[iy * nx + ix];
        let v10 = grid[iy * nx + ix1];
        let v01 = grid[iy1 * nx + ix];
        let v11 = grid[iy1 * nx + ix1];
        bilinear(v00, v10, v01, v11, fx, fy)
    }
}

/// Sorted, de-duplicated copy of a control point vector.
fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v.dedup();
    v
}

/// Indices that sort `v` ascending (stable, keeps duplicates in order).
fn sort_perm(v: &[f64]) -> Vec<usize> {
    let mut p: Vec<usize> = (0..v.len()).collect();
    p.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    p
}

/// Lower index `i` such that `xs[i] <= x <= xs[i+1]`, plus fraction.
/// Clamps to the grid ends.
fn bracket(xs: &[f64], x: f64) -> (usize, f64) {
    let last = xs.len() - 1;
    if x <= xs[0] {
        return (0, 0.0);
    }
    if x >= xs[last] {
        return (last - 1, 1.0);
    }
    let i = xs.partition_point(|c| *c <= x) - 1;
    let i = i.min(last - 1);
    let f = ((x - xs[i]) / (xs[i + 1] - xs[i])).clamp(0.0, 1.0);
    (i, f)
}

fn bilinear(v00: f64, v10: f64, v01: f64, v11: f64, fx: f64, fy: f64) -> f64 {
    v00 * (1.0 - fx) * (1.0 - fy) + v10 * fx * (1.0 - fy) + v01 * (1.0 - fx) * fy + v11 * fx * fy
}

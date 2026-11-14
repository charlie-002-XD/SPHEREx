//! Celestial WCS: TAN (gnomonic) projection with optional SIP distortion.
//!
//! Pixel coordinates are 1-based FITS coordinates (pixel (1,1) is the
//! centre of the first array element), matching the FITS/WCS convention and
//! the SPHEREx headers verbatim.

use crate::fits::error::FitsError;
use crate::fits::header::Header;

const DEG: f64 = std::f64::consts::PI / 180.0;

/// A 2-D SIP-style polynomial `f(u,v) = sum coeff[p][q] u^p v^q` over
/// `p+q <= order`.
#[derive(Debug, Clone, Default)]
pub struct Poly2 {
    order: usize,
    /// Coefficients indexed by `(p * (order + 1)) + q`... stored as flat
    /// vec with index `p * stride + q`, `stride = order + 1`.
    coeff: Vec<f64>,
}

impl Poly2 {
    /// Read `A_p_q`-style coefficients: `prefix` is `"A"` (gives `A_ORDER`,
    /// `A_0_0`, ...). Returns `None` if no coefficients present.
    fn read(h: &Header, prefix: &str) -> Option<Poly2> {
        let order = h.opt_i64(&format!("{prefix}_ORDER"))? as usize;
        if order > 9 {
            return None;
        }
        let stride = order + 1;
        let mut coeff = vec![0.0; stride * stride];
        let mut any = false;
        for (key, val) in h.iter() {
            if let Some(rest) = key.strip_prefix(&format!("{prefix}_")) {
                let mut parts = rest.split('_');
                if let (Some(Ok(p)), Some(Ok(q))) = (
                    parts.next().map(str::parse::<usize>),
                    parts.next().map(str::parse::<usize>),
                ) {
                    if p + q <= order {
                        if let Some(v) = val.and_then(|v| v.as_f64()) {
                            coeff[p * stride + q] = v;
                            any = true;
                        }
                    }
                }
            }
        }
        any.then_some(Poly2 { order, coeff })
    }

    /// Evaluate `sum a[p][q] u^p v^q`.
    pub fn eval(&self, u: f64, v: f64) -> f64 {
        let stride = self.order + 1;
        let mut sum = 0.0;
        let mut up = vec![1.0; stride];
        let mut vq = vec![1.0; stride];
        for i in 1..stride {
            up[i] = up[i - 1] * u;
            vq[i] = vq[i - 1] * v;
        }
        for (p, &up_p) in up.iter().enumerate() {
            for (q, &vq_q) in vq.iter().take(stride - p).enumerate() {
                sum += self.coeff[p * stride + q] * up_p * vq_q;
            }
        }
        sum
    }
}

/// SIP distortion pair: `a` corrects u, `b` corrects v.
#[derive(Debug, Clone, Default)]
pub struct Sip {
    /// Polynomial for the u (x) correction.
    pub a: Poly2,
    /// Polynomial for the v (y) correction.
    pub b: Poly2,
}

/// Linear celestial WCS (TAN) with optional SIP distortion.
#[derive(Debug, Clone)]
pub struct SkyWcs {
    /// Reference pixel, 1-based `[x, y]` (CRPIX1/CRPIX2).
    pub crpix: [f64; 2],
    /// `[ra_deg, dec_deg]` at the reference pixel (CRVAL1/CRVAL2).
    pub crval: [f64; 2],
    /// Degrees-per-pixel matrix `[[cd11, cd12], [cd21, cd22]]`
    /// (built from `CDi_j`, or `CDELT_i * PC_i_j`).
    pub cd: [[f64; 2]; 2],
    /// Forward (pixel -> corrected pixel) SIP, if present.
    sip_fwd: Option<Sip>,
    /// Inverse SIP (AP_/BP_ keywords), if present.
    sip_inv: Option<Sip>,
}

impl SkyWcs {
    /// Build from an image HDU header. `CTYPE1/2` must be `RA---TAN` or
    /// `RA---TAN-SIP` (the `-SIP` suffix is tolerated but not required for
    /// the coefficients to be applied).
    pub fn from_header(h: &Header) -> Result<SkyWcs, FitsError> {
        let ctype1 = h.opt_str("CTYPE1").unwrap_or_default();
        let ctype2 = h.opt_str("CTYPE2").unwrap_or_default();
        let check = |c: &str| c.trim_end().ends_with("TAN") || c.trim_end().ends_with("TAN-SIP");
        if !check(&ctype1) || !check(&ctype2) {
            return Err(FitsError::UnsupportedProjection(format!(
                "{ctype1} / {ctype2}"
            )));
        }
        let crpix = [h.req_f64("CRPIX1")?, h.req_f64("CRPIX2")?];
        let crval = [h.req_f64("CRVAL1")?, h.req_f64("CRVAL2")?];

        // Linear matrix: prefer CDi_j; else CDELT_i * PC_i_j; else diag CDELT.
        let cd = if h.opt_f64("CD1_1").is_some() || h.opt_f64("CD2_2").is_some() {
            [
                [
                    h.opt_f64("CD1_1").unwrap_or(0.0),
                    h.opt_f64("CD1_2").unwrap_or(0.0),
                ],
                [
                    h.opt_f64("CD2_1").unwrap_or(0.0),
                    h.opt_f64("CD2_2").unwrap_or(0.0),
                ],
            ]
        } else {
            let cdelt = [
                h.opt_f64("CDELT1").unwrap_or(1.0),
                h.opt_f64("CDELT2").unwrap_or(1.0),
            ];
            let pc = [
                [
                    h.opt_f64("PC1_1").unwrap_or(1.0),
                    h.opt_f64("PC1_2").unwrap_or(0.0),
                ],
                [
                    h.opt_f64("PC2_1").unwrap_or(0.0),
                    h.opt_f64("PC2_2").unwrap_or(1.0),
                ],
            ];
            [
                [cdelt[0] * pc[0][0], cdelt[0] * pc[0][1]],
                [cdelt[1] * pc[1][0], cdelt[1] * pc[1][1]],
            ]
        };
        let det = cd[0][0] * cd[1][1] - cd[0][1] * cd[1][0];
        if !det.is_finite() || det.abs() < 1e-30 {
            return Err(FitsError::Wcs("singular CD/PC matrix"));
        }

        let sip_fwd = match (Poly2::read(h, "A"), Poly2::read(h, "B")) {
            (Some(a), Some(b)) => Some(Sip { a, b }),
            _ => None,
        };
        let sip_inv = match (Poly2::read(h, "AP"), Poly2::read(h, "BP")) {
            (Some(a), Some(b)) => Some(Sip { a, b }),
            _ => None,
        };

        Ok(SkyWcs {
            crpix,
            crval,
            cd,
            sip_fwd,
            sip_inv,
        })
    }

    /// `(ra_deg, dec_deg)` of 1-based FITS pixel `(x, y)`.
    pub fn pixel_to_radec(&self, x: f64, y: f64) -> (f64, f64) {
        let mut u = x - self.crpix[0];
        let mut v = y - self.crpix[1];
        if let Some(sip) = &self.sip_fwd {
            // Both corrections are evaluated on the *original* offsets.
            let du = sip.a.eval(u, v);
            let dv = sip.b.eval(u, v);
            u += du;
            v += dv;
        }
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

    /// 1-based FITS pixel `(x, y)` of `(ra_deg, dec_deg)`.
    ///
    /// Errors when the point is behind the tangent plane or the inverse
    /// SIP does not converge.
    pub fn radec_to_pixel(&self, ra: f64, dec: f64) -> Result<(f64, f64), FitsError> {
        let (ra_r, dec_r) = (ra * DEG, dec * DEG);
        let d = [
            dec_r.cos() * ra_r.cos(),
            dec_r.cos() * ra_r.sin(),
            dec_r.sin(),
        ];
        let (ra0, dec0) = (self.crval[0] * DEG, self.crval[1] * DEG);
        let n0 = [dec0.cos() * ra0.cos(), dec0.cos() * ra0.sin(), dec0.sin()];
        let e_ra = [-ra0.sin(), ra0.cos(), 0.0];
        let e_dec = [-dec0.sin() * ra0.cos(), -dec0.sin() * ra0.sin(), dec0.cos()];
        let denom = dot(d, n0);
        if denom <= 0.0 {
            return Err(FitsError::Wcs("point behind the tangent plane"));
        }
        let xi = (dot(d, e_ra) / denom).to_degrees();
        let eta = (dot(d, e_dec) / denom).to_degrees();

        let det = self.cd[0][0] * self.cd[1][1] - self.cd[0][1] * self.cd[1][0];
        let up = (self.cd[1][1] * xi - self.cd[0][1] * eta) / det;
        let vp = (-self.cd[1][0] * xi + self.cd[0][0] * eta) / det;

        // Undo the SIP distortion on the corrected offsets (u', v').
        let (u, v) = if let Some(sip) = &self.sip_fwd {
            // Fixed-point inversion of u' = u + f(u,v), seeded with the
            // AP_/BP_ inverse polynomial when present (its fit residuals
            // are then corrected by iteration).
            let (mut u, mut v) = match &self.sip_inv {
                Some(inv) => (up + inv.a.eval(up, vp), vp + inv.b.eval(up, vp)),
                None => (up, vp),
            };
            let mut converged = false;
            for _ in 0..40 {
                let nu = up - sip.a.eval(u, v);
                let nv = vp - sip.b.eval(u, v);
                if (nu - u).abs() < 1e-10 && (nv - v).abs() < 1e-10 {
                    u = nu;
                    v = nv;
                    converged = true;
                    break;
                }
                if !nu.is_finite() || !nv.is_finite() {
                    break;
                }
                u = nu;
                v = nv;
            }
            if !converged {
                return Err(FitsError::Wcs("SIP inversion did not converge"));
            }
            (u, v)
        } else if let Some(inv) = &self.sip_inv {
            (up + inv.a.eval(up, vp), vp + inv.b.eval(up, vp))
        } else {
            (up, vp)
        };
        Ok((u + self.crpix[0], v + self.crpix[1]))
    }
}

fn norm(d: [f64; 3]) -> f64 {
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

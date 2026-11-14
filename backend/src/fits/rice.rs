//! RICE_1 tile decompression (FITS tiled-image convention), a port of the
//! cfitsio `fits_rdecomp` family for 8/16/32-bit integer pixels.
//!
//! Reference: CFITSIO `ricecomp.c`, and the "Tiled Image Compression
//! Convention" registered document (fits.gsfc.nasa.gov).

use crate::fits::error::FitsError;

/// MSB-first bit reader over a byte slice.
struct BitReader<'a> {
    buf: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, bit: 0 }
    }

    /// Read `n` bits (n <= 32) as a big-endian-aligned u32.
    fn read_bits(&mut self, n: u32) -> Result<u32, FitsError> {
        debug_assert!(n <= 32);
        let mut v: u32 = 0;
        for _ in 0..n {
            let byte = *self
                .buf
                .get(self.bit / 8)
                .ok_or(FitsError::Truncated("rice bitstream"))?;
            v = (v << 1) | u32::from(byte >> (7 - self.bit % 8) & 1);
            self.bit += 1;
        }
        Ok(v)
    }

    /// Count 0-bits until the next 1-bit (unary quotient); consumes the 1.
    /// Terminates on end-of-input (Truncated), so it is bounded by the
    /// stream length.
    fn read_unary(&mut self) -> Result<u32, FitsError> {
        let mut n = 0u32;
        loop {
            let byte = *self
                .buf
                .get(self.bit / 8)
                .ok_or(FitsError::Truncated("rice bitstream"))?;
            let b = byte >> (7 - self.bit % 8) & 1;
            self.bit += 1;
            if b == 1 {
                return Ok(n);
            }
            n = n
                .checked_add(1)
                .ok_or(FitsError::Corrupt("rice unary overflow".into()))?;
        }
    }
}

/// Parameters of the Rice split for each integer width.
struct RiceParams {
    fsbits: u32,
    fsmax: u32,
    /// bits per pixel for the "direct" (high entropy) path.
    bbits: u32,
    /// raw bytes of the uncompressed first pixel.
    first_bytes: usize,
}

fn params_for(elem_bytes: usize) -> Result<RiceParams, FitsError> {
    Ok(match elem_bytes {
        1 => RiceParams {
            fsbits: 3,
            fsmax: 6,
            bbits: 8,
            first_bytes: 1,
        },
        2 => RiceParams {
            fsbits: 4,
            fsmax: 14,
            bbits: 16,
            first_bytes: 2,
        },
        4 => RiceParams {
            fsbits: 5,
            fsmax: 25,
            bbits: 32,
            first_bytes: 4,
        },
        other => {
            return Err(FitsError::UnsupportedCompression(format!(
                "RICE with {other}-byte pixels"
            )))
        }
    })
}

/// Undo the zigzag mapping applied to pixel differences.
fn zigzag_undo(diff: u32) -> u32 {
    if diff & 1 == 0 {
        diff >> 1
    } else {
        !(diff >> 1)
    }
}

/// Decompress one RICE_1 stream into `npix` integers of `elem_bytes` width.
///
/// `nblock` is the coding block size (FITS default 32). Returns the decoded
/// values sign-extended to `i64`.
pub fn decode(
    bytes: &[u8],
    npix: usize,
    elem_bytes: usize,
    nblock: usize,
) -> Result<Vec<i64>, FitsError> {
    let p = params_for(elem_bytes)?;
    if npix == 0 {
        return Ok(Vec::new());
    }
    if nblock == 0 {
        return Err(FitsError::Corrupt("RICE nblock 0".into()));
    }
    let first = bytes
        .get(..p.first_bytes)
        .ok_or(FitsError::Truncated("rice first pixel"))?;
    let mut lastpix: u32 = match p.first_bytes {
        1 => u32::from(first[0]),
        2 => u32::from(u16::from_be_bytes([first[0], first[1]])),
        _ => u32::from_be_bytes([first[0], first[1], first[2], first[3]]),
    };
    let mut out = Vec::with_capacity(npix);
    // `lastpix` seeds the running value; the stream itself encodes npix
    // differences (the first one is always zero).
    let mut r = BitReader::new(&bytes[p.first_bytes..]);
    let mut i = 0usize;
    while i < npix {
        let fs = r.read_bits(p.fsbits)? as i64 - 1;
        let imax = (i + nblock).min(npix);
        if fs < 0 {
            // Low-entropy block: all differences are zero.
            while i < imax {
                out.push(sign_extend(lastpix, p.bbits));
                i += 1;
            }
        } else if fs as u32 == p.fsmax {
            // High-entropy block: raw differences, bbits each.
            while i < imax {
                let diff = zigzag_undo(r.read_bits(p.bbits)?);
                lastpix = lastpix.wrapping_add(diff);
                out.push(sign_extend(lastpix, p.bbits));
                i += 1;
            }
        } else {
            let fs = fs as u32;
            while i < imax {
                let nzero = r.read_unary()?;
                if fs >= 32 || nzero > (u32::MAX >> fs) {
                    return Err(FitsError::Corrupt("rice quotient too large".into()));
                }
                let rem = if fs > 0 { r.read_bits(fs)? } else { 0 };
                let diff = zigzag_undo((nzero << fs) | rem);
                lastpix = lastpix.wrapping_add(diff);
                out.push(sign_extend(lastpix, p.bbits));
                i += 1;
            }
        }
    }
    Ok(out)
}

/// Sign-extend the low `bits` of `v` into i64.
fn sign_extend(v: u32, bits: u32) -> i64 {
    match bits {
        32 => v as i32 as i64,
        16 => v as u16 as i16 as i64,
        8 => v as u8 as i8 as i64,
        _ => v as i64,
    }
}

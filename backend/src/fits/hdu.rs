//! Iteration over HDUs of an in-memory FITS file.
//!
//! Every read is bounds-checked; truncated or corrupt input returns an error.

use crate::fits::error::FitsError;
use crate::fits::header::Header;

const BLOCK: usize = 2880;

/// One Header-Data Unit: parsed header plus the byte range of its data area
/// (excluding padding).
#[derive(Debug)]
pub struct Hdu {
    /// Parsed header cards.
    pub header: Header,
    /// Byte range of the (unpadded) data area inside the input buffer.
    pub data: std::ops::Range<usize>,
    /// True for the primary HDU.
    pub is_primary: bool,
}

impl Hdu {
    /// `EXTNAME` value, trimmed (primary HDU has none).
    pub fn extname(&self) -> Option<String> {
        self.header.opt_str("EXTNAME")
    }

    /// `XTENSION` value ('IMAGE', 'BINTABLE', ...); `None` for the primary HDU.
    pub fn xtension(&self) -> Option<String> {
        self.header.opt_str("XTENSION")
    }

    /// True if this HDU is a tile-compressed image (ZIMAGE convention).
    pub fn is_compressed_image(&self) -> bool {
        self.header.opt_bool("ZIMAGE") == Some(true)
            || (self.header.opt_i64("ZBITPIX").is_some() && self.header.opt_i64("ZNAXIS").is_some())
    }
}

/// Parse all HDUs of `bytes`.
pub fn parse_hdus(bytes: &[u8]) -> Result<Vec<Hdu>, FitsError> {
    if bytes.len() < BLOCK {
        return Err(FitsError::NotFits("input shorter than one FITS block"));
    }
    // The primary HDU must begin with `SIMPLE  =` (or `XTENSION` for random
    // groups, which we do not need).
    if !matches!(first_keyword(bytes), Some(k) if k == "SIMPLE") {
        return Err(FitsError::NotFits("first keyword is not SIMPLE"));
    }

    let mut hdus = Vec::new();
    let mut pos = 0usize;
    loop {
        if pos >= bytes.len() {
            break;
        }
        // Trailing padding (zeros/spaces) shorter than one block is legal.
        let remaining = &bytes[pos..];
        if remaining.iter().all(|b| *b == 0 || *b == b' ') {
            break;
        }
        if remaining.len() < BLOCK {
            return Err(FitsError::Truncated("partial FITS block after last HDU"));
        }

        // Read 2880-byte blocks until a card whose keyword is END appears.
        let mut header = Header::default();
        let mut found_end = false;
        while pos + BLOCK <= bytes.len() {
            for i in 0..36 {
                let card = &bytes[pos + i * 80..pos + i * 80 + 80];
                let (key, value) = Header::parse_card(card);
                if key == "END" {
                    found_end = true;
                    break;
                }
                header.cards.push((key, value));
            }
            pos += BLOCK;
            if found_end {
                break;
            }
        }
        if !found_end {
            return Err(FitsError::Truncated("header without END card"));
        }

        let data_len = declared_data_len(&header)?;
        let data_end = pos
            .checked_add(data_len)
            .ok_or(FitsError::Corrupt("declared data size overflows".into()))?;
        if data_end > bytes.len() {
            return Err(FitsError::Truncated("HDU data area"));
        }
        let is_primary = hdus.is_empty();
        hdus.push(Hdu {
            header,
            data: pos..data_end,
            is_primary,
        });
        pos = data_end.div_ceil(BLOCK) * BLOCK;
    }

    if hdus.is_empty() {
        return Err(FitsError::NotFits("no HDUs"));
    }
    Ok(hdus)
}

/// Keyword of the very first card of the file.
fn first_keyword(bytes: &[u8]) -> Option<String> {
    let card = bytes.get(..80)?;
    Some(std::str::from_utf8(&card[..8]).ok()?.trim_end().to_string())
}

/// Declared data size of an HDU in bytes:
/// `GCOUNT * (PCOUNT + |BITPIX|/8 * prod(NAXISi))`.
fn declared_data_len(header: &Header) -> Result<usize, FitsError> {
    let bitpix = header.req_i64("BITPIX")?;
    let elem_bytes: u64 = (bitpix.unsigned_abs()) / 8;
    if elem_bytes == 0 || bitpix.unsigned_abs() % 8 != 0 {
        return Err(FitsError::Corrupt(format!("invalid BITPIX {bitpix}")));
    }
    let naxis = header.req_i64("NAXIS")?;
    if !(0..=999).contains(&naxis) {
        return Err(FitsError::Corrupt(format!("invalid NAXIS {naxis}")));
    }
    // NAXIS=0 means no data array (the empty product would wrongly give 1).
    let mut npix: u64 = if naxis == 0 { 0 } else { 1 };
    for i in 1..=naxis {
        let n = header.req_i64(&format!("NAXIS{i}"))?;
        if n < 0 {
            return Err(FitsError::Corrupt(format!("negative NAXIS{i}")));
        }
        npix = npix
            .checked_mul(n as u64)
            .ok_or(FitsError::Corrupt("NAXIS product overflows".into()))?;
    }
    let pcount = header.opt_i64("PCOUNT").unwrap_or(0);
    let gcount = header.opt_i64("GCOUNT").unwrap_or(1);
    if pcount < 0 || gcount < 0 {
        return Err(FitsError::Corrupt("negative PCOUNT/GCOUNT".into()));
    }
    (gcount as u64)
        .checked_mul(
            npix.checked_mul(elem_bytes)
                .and_then(|v| v.checked_add(pcount as u64))
                .ok_or(FitsError::Corrupt("data size overflows".into()))?,
        )
        .map(|v| v as usize)
        .ok_or(FitsError::Corrupt("data size overflows".into()))
}

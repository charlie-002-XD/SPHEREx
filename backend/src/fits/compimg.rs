//! Tile-compressed image decoding (ZIMAGE convention).
//!
//! Only the compression schemes a pure-Rust reader can support are decoded:
//! `RICE_1` and uncompressed tiles (`UNCOMPRESSED_DATA` column or
//! `ZCMPTYPE='NOCOMPRESS'`). `GZIP_*`, `HCOMPRESS` and `PLIO` return
//! [`FitsError::UnsupportedCompression`].

use crate::fits::error::FitsError;
use crate::fits::hdu::Hdu;
use crate::fits::rice;
use crate::fits::table::{decode_numeric, BinTable, Column, ElemKind};

/// Decoded pixel data of a compressed image.
#[derive(Debug)]
pub enum Pixels {
    /// Integer pixels (ZBITPIX > 0).
    Int(Vec<i64>),
    /// Float pixels (ZBITPIX < 0 or with ZSCALE/ZZERO applied).
    Float(Vec<f64>),
}

/// Decompressed image: `dims` is `[NAXIS1, NAXIS2, ...]` (first axis
/// fastest), `pixels` has `prod(dims)` elements in FITS order.
#[derive(Debug)]
pub struct DecompressedImage {
    /// Original image dimensions (ZNAXISn).
    pub dims: Vec<usize>,
    /// Decoded pixels.
    pub pixels: Pixels,
}

fn elem_kind_for_bitpix(bitpix: i64) -> Result<ElemKind, FitsError> {
    Ok(match bitpix {
        8 => ElemKind::Byte,
        16 => ElemKind::Short,
        32 => ElemKind::Int,
        64 => ElemKind::Long,
        -32 => ElemKind::Float,
        -64 => ElemKind::Double,
        other => return Err(FitsError::Corrupt(format!("invalid ZBITPIX {other}"))),
    })
}

/// Decode a tile-compressed image HDU.
pub fn decompress(hdu: &Hdu, bytes: &[u8]) -> Result<DecompressedImage, FitsError> {
    let h = &hdu.header;
    let zbitpix = h.req_i64("ZBITPIX")?;
    let znaxis = h.req_i64("ZNAXIS")?;
    if !(1..=6).contains(&znaxis) {
        return Err(FitsError::Corrupt(format!("bad ZNAXIS {znaxis}")));
    }
    let mut dims = Vec::with_capacity(znaxis as usize);
    for i in 1..=znaxis {
        let n = h.req_i64(&format!("ZNAXIS{i}"))?;
        if n <= 0 {
            return Err(FitsError::Corrupt(format!("bad ZNAXIS{i}")));
        }
        dims.push(n as usize);
    }
    // Tile dimensions: default = whole image.
    let mut tile = dims.clone();
    for (i, t) in tile.iter_mut().enumerate() {
        if let Some(v) = h.opt_i64(&format!("ZTILE{}", i + 1)) {
            if v <= 0 {
                return Err(FitsError::Corrupt(format!("bad ZTILE{}", i + 1)));
            }
            *t = v as usize;
        }
    }
    let ntiles_per_axis: Vec<usize> = dims
        .iter()
        .zip(&tile)
        .map(|(d, t)| d.div_ceil(*t))
        .collect();
    let ntiles: usize = ntiles_per_axis.iter().product();
    if ntiles == 0 {
        return Err(FitsError::Corrupt("zero tiles".into()));
    }
    // (origin, extent) of tile index t; tile order is first-axis-fastest.
    let tile_geometry = |t: usize| -> (Vec<usize>, Vec<usize>) {
        let mut rem = t;
        let mut origin = vec![0usize; dims.len()];
        for (axis, n) in ntiles_per_axis.iter().enumerate() {
            origin[axis] = rem % n;
            rem /= n;
        }
        for (axis, o) in origin.iter_mut().enumerate() {
            *o *= tile[axis];
        }
        let extent: Vec<usize> = dims
            .iter()
            .zip(&origin)
            .zip(&tile)
            .map(|((d, o), t)| (*t).min(d - o))
            .collect();
        (origin, extent)
    };

    // Compression parameters from ZNAMEn/ZVALn pairs.
    let mut nblock: usize = 32;
    for i in 1..=8 {
        if h.opt_str(&format!("ZNAME{i}"))
            .is_some_and(|n| n.eq_ignore_ascii_case("BLOCKSIZE"))
        {
            if let Some(v) = h.opt_i64(&format!("ZVAL{i}")) {
                nblock = v.max(1) as usize;
            }
        }
    }
    let zcmptype = h.opt_str("ZCMPTYPE").unwrap_or_else(|| "RICE_1".into());

    let table = BinTable::new(hdu, bytes)?;
    if table.nrows < ntiles {
        return Err(FitsError::Corrupt(format!(
            "compressed image has {} rows but {} tiles are required",
            table.nrows, ntiles
        )));
    }
    let data_col = table
        .column("COMPRESSED_DATA")
        .ok_or(FitsError::Corrupt("no COMPRESSED_DATA column".into()))?;
    let raw_col = table.column("UNCOMPRESSED_DATA");
    let scale_col = table.column("ZSCALE");
    let zero_col = table.column("ZZERO");

    let npix_total: usize = dims.iter().product();
    let mut out: Vec<i64> = vec![0; npix_total];
    let elem_bytes = (zbitpix.unsigned_abs() / 8) as usize;
    let raw_kind = elem_kind_for_bitpix(zbitpix)?;

    for t in 0..ntiles {
        let (origin, extent) = tile_geometry(t);
        let tile_npix: usize = extent.iter().product();

        // Bytes of this tile's compressed stream (or raw tile). A
        // variable-length cell with 0 elements means "store raw in
        // UNCOMPRESSED_DATA".
        let stream: Option<&[u8]> = match data_col.store {
            crate::fits::table::ColStore::Fixed { .. } => Some(table.cell_raw(t, data_col)?),
            _ => {
                let b = table.cell_var(t, data_col)?;
                if b.is_empty() {
                    None
                } else {
                    Some(b)
                }
            }
        };
        let vals: Vec<i64> = match stream {
            None => decode_raw_tile(&table, t, raw_col, raw_kind, tile_npix)?,
            Some(s) if zcmptype.eq_ignore_ascii_case("RICE_1") => {
                rice::decode(s, tile_npix, elem_bytes, nblock)?
            }
            Some(s) if zcmptype.eq_ignore_ascii_case("NOCOMPRESS") => {
                decode_numeric_i64(s, raw_kind)?
            }
            Some(_) => return Err(FitsError::UnsupportedCompression(zcmptype)),
        };
        if vals.len() < tile_npix {
            return Err(FitsError::Corrupt("tile shorter than expected".into()));
        }

        scatter(&mut out, &dims, &origin, &extent, &vals);
    }

    // Per-tile ZSCALE/ZZERO (or float ZBITPIX) -> float output.
    let want_float = scale_col.is_some() || zero_col.is_some() || zbitpix < 0;
    if want_float {
        let mut fvals = vec![0f64; npix_total];
        for t in 0..ntiles {
            let scale = match scale_col {
                Some(c) => table.cell_f64s(t, c)?.first().copied().unwrap_or(1.0),
                None => 1.0,
            };
            let zero = match zero_col {
                Some(c) => table.cell_f64s(t, c)?.first().copied().unwrap_or(0.0),
                None => 0.0,
            };
            let (origin, extent) = tile_geometry(t);
            apply_scale(&mut fvals, &out, &dims, &origin, &extent, scale, zero);
        }
        return Ok(DecompressedImage {
            dims,
            pixels: Pixels::Float(fvals),
        });
    }

    Ok(DecompressedImage {
        dims,
        pixels: Pixels::Int(out),
    })
}

/// Scatter tile pixels (first axis fastest) into the full image buffer.
/// At most `prod(extent)` values are written, even if `vals` is longer.
fn scatter(out: &mut [i64], dims: &[usize], origin: &[usize], extent: &[usize], vals: &[i64]) {
    let npix: usize = extent.iter().product();
    for (lin, v) in vals.iter().take(npix).enumerate() {
        let mut rem = lin;
        let mut idx = 0usize;
        let mut stride = 1usize;
        for (axis, d) in dims.iter().enumerate() {
            let coord = rem % extent[axis];
            rem /= extent[axis];
            idx += (origin[axis] + coord) * stride;
            stride *= d;
        }
        out[idx] = *v;
    }
}

/// Scale a tile's raw integers into the float output.
fn apply_scale(
    dst: &mut [f64],
    src: &[i64],
    dims: &[usize],
    origin: &[usize],
    extent: &[usize],
    scale: f64,
    zero: f64,
) {
    let npix: usize = extent.iter().product();
    for lin in 0..npix {
        let mut rem = lin;
        let mut idx = 0usize;
        let mut stride = 1usize;
        for (axis, d) in dims.iter().enumerate() {
            let coord = rem % extent[axis];
            rem /= extent[axis];
            idx += (origin[axis] + coord) * stride;
            stride *= d;
        }
        dst[idx] = src[idx] as f64 * scale + zero;
    }
}

/// Decode a raw (uncompressed) tile from the UNCOMPRESSED_DATA column.
fn decode_raw_tile(
    table: &BinTable,
    row: usize,
    raw_col: Option<&Column>,
    kind: ElemKind,
    npix: usize,
) -> Result<Vec<i64>, FitsError> {
    let col = raw_col.ok_or(FitsError::Corrupt(
        "empty COMPRESSED_DATA cell without UNCOMPRESSED_DATA".into(),
    ))?;
    let raw = table.cell_raw(row, col)?;
    let vals = decode_numeric_i64(raw, kind)?;
    if vals.len() < npix {
        return Err(FitsError::Truncated("uncompressed tile"));
    }
    Ok(vals[..npix].to_vec())
}

/// Decode big-endian integers into i64.
fn decode_numeric_i64(bytes: &[u8], kind: ElemKind) -> Result<Vec<i64>, FitsError> {
    Ok(match kind {
        ElemKind::Byte => bytes.iter().map(|b| i64::from(*b)).collect(),
        ElemKind::Short => bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i64::from(i16::from_be_bytes(*c)))
            .collect(),
        ElemKind::Int => bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i64::from(i32::from_be_bytes(*c)))
            .collect(),
        ElemKind::Long => bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| i64::from_be_bytes(*c))
            .collect(),
        ElemKind::Float => decode_numeric(bytes, ElemKind::Float)?
            .into_iter()
            .map(|v| v as i64)
            .collect(),
        ElemKind::Double => decode_numeric(bytes, ElemKind::Double)?
            .into_iter()
            .map(|v| v as i64)
            .collect(),
        _ => {
            return Err(FitsError::Corrupt(
                "unsupported element kind for image tile".into(),
            ))
        }
    })
}

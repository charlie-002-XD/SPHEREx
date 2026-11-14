//! Synthetic FITS writers used by the fits parser tests.
//!
//! Builds small multi-extension FITS files in memory, including a
//! tile-compressed (ZIMAGE/RICE_1) FLAGS extension via a RICE_1 encoder
//! mirroring cfitsio `fits_rcomp` for 32-bit integer pixels.

#![allow(dead_code)]

const BLOCK: usize = 2880;

/// Format one 80-byte card.
pub fn card(text: &str) -> String {
    let mut s = text.to_string();
    while s.len() < 80 {
        s.push(' ');
    }
    s.truncate(80);
    s
}

pub fn card_str(key: &str, val: &str) -> String {
    card(&format!("{key:<8}= '{val:<8}'"))
}

pub fn card_int(key: &str, val: i64) -> String {
    card(&format!("{key:<8}= {val:>20}"))
}

pub fn card_flt(key: &str, val: f64) -> String {
    card(&format!("{key:<8}= {val:>20.12}"))
}

pub fn card_bool(key: &str, val: bool) -> String {
    card(&format!("{key:<8}= {:>20}", if val { "T" } else { "F" }))
}

/// Whole 2880-byte blocks of cards ending with END.
pub fn header_block(cards: &[String]) -> Vec<u8> {
    let mut all = cards.to_vec();
    all.push(card("END"));
    let mut bytes = Vec::new();
    for c in &all {
        bytes.extend_from_slice(c.as_bytes());
    }
    while bytes.len() % BLOCK != 0 {
        bytes.push(b' ');
    }
    bytes
}

/// Pad a data section to a 2880 boundary with zero bytes.
fn pad_data(mut d: Vec<u8>) -> Vec<u8> {
    while !d.len().is_multiple_of(BLOCK) {
        d.push(0);
    }
    d
}

/// Primary HDU (SIMPLE, no data).
pub fn primary_hdu() -> Vec<u8> {
    header_block(&[
        card("SIMPLE  =                    T"),
        card_int("BITPIX", 8),
        card_int("NAXIS", 0),
        card("EXTEND  =                    T"),
    ])
}

/// A 2-D image extension. `data` must be `nx*ny` elements of `bitpix`
/// already encoded big-endian.
pub fn image_hdu(
    extname: &str,
    bitpix: i64,
    nx: usize,
    ny: usize,
    data: &[u8],
    extra: &[String],
) -> Vec<u8> {
    let mut cards = vec![
        card_str("XTENSION", "IMAGE"),
        card_int("BITPIX", bitpix),
        card_int("NAXIS", 2),
        card_int("NAXIS1", nx as i64),
        card_int("NAXIS2", ny as i64),
        card_int("PCOUNT", 0),
        card_int("GCOUNT", 1),
        card_str("EXTNAME", extname),
    ];
    cards.extend_from_slice(extra);
    let mut out = header_block(&cards);
    out.extend(pad_data(data.to_vec()));
    out
}

/// One column of a binary table.
pub struct TableCol {
    pub ttype: String,
    pub tform: String,
    /// Byte width of the field in each row.
    pub width: usize,
}

/// A BINTABLE extension. `rows` is row_len * nrows bytes; `heap` the
/// variable-length area (PCOUNT).
pub fn bintable_hdu(
    extname: &str,
    cols: &[TableCol],
    nrows: usize,
    rows: &[u8],
    heap: &[u8],
    extra: &[String],
) -> Vec<u8> {
    let row_len: usize = cols.iter().map(|c| c.width).sum();
    assert_eq!(rows.len(), row_len * nrows);
    let mut cards = vec![
        card_str("XTENSION", "BINTABLE"),
        card_int("BITPIX", 8),
        card_int("NAXIS", 2),
        card_int("NAXIS1", row_len as i64),
        card_int("NAXIS2", nrows as i64),
        card_int("PCOUNT", heap.len() as i64),
        card_int("GCOUNT", 1),
        card_int("TFIELDS", cols.len() as i64),
    ];
    for (i, c) in cols.iter().enumerate() {
        cards.push(card_str(&format!("TTYPE{}", i + 1), &c.ttype));
        cards.push(card_str(&format!("TFORM{}", i + 1), &c.tform));
    }
    cards.push(card_str("EXTNAME", extname));
    cards.extend_from_slice(extra);
    let mut out = header_block(&cards);
    let mut data = rows.to_vec();
    data.extend_from_slice(heap);
    out.extend(pad_data(data));
    out
}

/// Encode f32 image data big-endian.
pub fn f32_data(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_be_bytes()).collect()
}

/// Encode i32 image data big-endian.
pub fn i32_data(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_be_bytes()).collect()
}

// ---------------------------------------------------------------------
// RICE_1 encoder (cfitsio fits_rcomp port, 32-bit pixels).
// ---------------------------------------------------------------------

struct BitWriter {
    buf: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            buf: Vec::new(),
            acc: 0,
            nbits: 0,
        }
    }

    /// Emit the low `n` bits of `v`, MSB-first.
    fn write(&mut self, v: u64, n: u32) {
        for i in (0..n).rev() {
            let b = (v >> i) & 1;
            self.acc = (self.acc << 1) | b;
            self.nbits += 1;
            if self.nbits == 8 {
                self.buf.push(self.acc as u8);
                self.acc = 0;
                self.nbits = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.buf.push((self.acc << (8 - self.nbits)) as u8);
        }
        self.buf
    }
}

/// Zigzag mapping of signed differences to unsigned.
fn zigzag(d: i32) -> u32 {
    if d < 0 {
        !(d << 1) as u32
    } else {
        (d << 1) as u32
    }
}

/// RICE_1-encode i32 pixels (nblock=32 as in the FITS convention).
pub fn rice_encode_i32(a: &[i32], nblock: usize) -> Vec<u8> {
    assert!(!a.is_empty());
    const FSBITS: u32 = 5;
    const FSMAX: u32 = 25;
    const BBITS: u32 = 32;
    let mut w = BitWriter::new();
    w.write(a[0] as u32 as u64, 32);
    let mut lastpix = a[0];
    let mut i = 0usize;
    while i < a.len() {
        let this = nblock.min(a.len() - i);
        let mut diffs = vec![0u32; this];
        let mut pixelsum: u64 = 0;
        for (j, d) in diffs.iter_mut().enumerate() {
            let nextpix = a[i + j];
            let pdiff = nextpix.wrapping_sub(lastpix);
            *d = zigzag(pdiff);
            pixelsum += u64::from(*d);
            lastpix = nextpix;
        }
        let dpsum = ((pixelsum as f64) - (this as f64) / 2.0 - 1.0) / this as f64;
        let mut psum = (dpsum.max(0.0) as u64) >> 1;
        let mut fs = 0u32;
        while psum > 0 {
            psum >>= 1;
            fs += 1;
        }
        if fs >= FSMAX {
            w.write(u64::from(FSMAX + 1), FSBITS);
            for d in &diffs {
                w.write(u64::from(*d), BBITS);
            }
        } else if fs == 0 && pixelsum == 0 {
            w.write(0, FSBITS);
        } else {
            w.write(u64::from(fs + 1), FSBITS);
            let fsmask = (1u32 << fs) - 1;
            for d in &diffs {
                let top = d >> fs;
                // `top` zero bits then a one bit.
                w.write(0, top);
                w.write(1, 1);
                if fs > 0 {
                    w.write(u64::from(d & fsmask), fs);
                }
            }
        }
        i += this;
    }
    w.finish()
}

/// A tile-compressed image extension (ZIMAGE), one tile per row of the
/// compressed-data table.
///
/// `dims` = [nx, ny], `tile` = [tx, ty] nominal tile size, `pixels` the
/// full image as i32 (first axis fastest). Each tile is RICE_1 encoded;
/// tiles listed in `raw_tiles` are instead stored (raw big-endian) in the
/// UNCOMPRESSED_DATA column with an empty COMPRESSED_DATA descriptor.
pub fn compressed_image_hdu(
    extname: &str,
    dims: [usize; 2],
    tile: [usize; 2],
    pixels: &[i32],
    raw_tiles: &[usize],
    extra: &[String],
) -> Vec<u8> {
    let (nx, ny) = (dims[0], dims[1]);
    let (tx, ty) = (tile[0], tile[1]);
    assert_eq!(pixels.len(), nx * ny);
    let nt1 = nx.div_ceil(tx);
    let nt2 = ny.div_ceil(ty);
    let ntiles = nt1 * nt2;

    // Extract tile pixels (first axis fastest within the tile).
    let tile_pixels = |t: usize| -> Vec<i32> {
        let ox = (t % nt1) * tx;
        let oy = (t / nt1) * ty;
        let ex = tx.min(nx - ox);
        let ey = ty.min(ny - oy);
        let mut v = Vec::with_capacity(ex * ey);
        for j in 0..ey {
            for i in 0..ex {
                v.push(pixels[(oy + j) * nx + (ox + i)]);
            }
        }
        v
    };

    // Build per-tile streams and the table row data.
    let mut heap: Vec<u8> = Vec::new();
    let has_raw = !raw_tiles.is_empty();
    let mut rows: Vec<u8> = Vec::new();
    for t in 0..ntiles {
        let tp = tile_pixels(t);
        if raw_tiles.contains(&t) {
            // Empty variable-length cell -> tile kept in UNCOMPRESSED_DATA.
            rows.extend_from_slice(&0u32.to_be_bytes());
            rows.extend_from_slice(&0u32.to_be_bytes());
            rows.extend_from_slice(&i32_data(&tp));
        } else {
            let stream = rice_encode_i32(&tp, 32);
            let off = heap.len() as u32;
            heap.extend_from_slice(&stream);
            rows.extend_from_slice(&(stream.len() as u32).to_be_bytes());
            rows.extend_from_slice(&off.to_be_bytes());
            if has_raw {
                rows.extend_from_slice(&vec![0u8; tp.len() * 4]);
            }
        }
    }

    let mut cols = vec![TableCol {
        ttype: "COMPRESSED_DATA".into(),
        tform: "1PB".into(),
        width: 8,
    }];
    if has_raw {
        cols.push(TableCol {
            ttype: "UNCOMPRESSED_DATA".into(),
            tform: format!("{}J", tx * ty),
            width: tx * ty * 4,
        });
    }

    let mut extra_cards = vec![
        card_bool("ZIMAGE", true),
        card_int("ZTILE1", tx as i64),
        card_int("ZTILE2", ty as i64),
        card_str("ZCMPTYPE", "RICE_1"),
        card_str("ZNAME1", "BLOCKSIZE"),
        card_int("ZVAL1", 32),
        card_int("ZBITPIX", 32),
        card_int("ZNAXIS", 2),
        card_int("ZNAXIS1", nx as i64),
        card_int("ZNAXIS2", ny as i64),
    ];
    extra_cards.extend_from_slice(extra);
    bintable_hdu(extname, &cols, ntiles, &rows, &heap, &extra_cards)
}

/// A `WCS-WAVE` table with one row: X (nJ), Y (nJ), VALUES (nE).
pub fn wcs_wave_hdu(xs: &[i32], ys: &[i32], values: &[f32], extra: &[String]) -> Vec<u8> {
    let cols = [
        TableCol {
            ttype: "X".into(),
            tform: format!("{}J", xs.len()),
            width: xs.len() * 4,
        },
        TableCol {
            ttype: "Y".into(),
            tform: format!("{}J", ys.len()),
            width: ys.len() * 4,
        },
        TableCol {
            ttype: "VALUES".into(),
            tform: format!("{}E", values.len()),
            width: values.len() * 4,
        },
    ];
    let mut rows = i32_data(xs);
    rows.extend(i32_data(ys));
    rows.extend(f32_data(values));
    bintable_hdu("WCS-WAVE", &cols, 1, &rows, &[], extra)
}

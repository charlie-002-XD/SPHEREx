//! Minimal FITS binary table (BINTABLE) reader.
//!
//! Supports the fixed-width formats we need plus `P`/`Q` variable-length
//! array descriptors into the heap (required for tile-compressed images).

use crate::fits::error::FitsError;
use crate::fits::hdu::Hdu;

/// Element type of a table column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ElemKind {
    /// 'L' logical, 1 byte.
    Logical,
    /// 'X' bit array, repeat bits packed into bytes.
    Bits,
    /// 'B' unsigned byte.
    Byte,
    /// 'I' 16-bit signed int.
    Short,
    /// 'J' 32-bit signed int.
    Int,
    /// 'K' 64-bit signed int.
    Long,
    /// 'A' ASCII char.
    Char,
    /// 'E' 32-bit float.
    Float,
    /// 'D' 64-bit float.
    Double,
    /// 'C' complex (2 floats).
    Complex,
    /// 'M' double complex (2 doubles).
    DComplex,
}

impl ElemKind {
    fn size(self) -> usize {
        match self {
            ElemKind::Logical | ElemKind::Byte | ElemKind::Char => 1,
            ElemKind::Short => 2,
            ElemKind::Int | ElemKind::Float | ElemKind::Complex => {
                if self == ElemKind::Complex {
                    8
                } else {
                    4
                }
            }
            ElemKind::Long | ElemKind::Double | ElemKind::DComplex => {
                if self == ElemKind::DComplex {
                    16
                } else {
                    8
                }
            }
            ElemKind::Bits => 0, // size computed from repeat count
        }
    }
}

/// How a column's bytes are stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColStore {
    /// Fixed-width array of `repeat` elements of `elem`.
    Fixed { elem: ElemKind, repeat: usize },
    /// Variable-length array, 32-bit `(count, offset)` descriptors.
    VarP { elem: ElemKind, repeat: usize },
    /// Variable-length array, 64-bit descriptors.
    VarQ { elem: ElemKind, repeat: usize },
}

/// One column definition.
#[derive(Debug, Clone)]
pub struct Column {
    /// `TTYPEn` value, trimmed (may be empty).
    pub name: String,
    /// Byte offset of the field within a row.
    pub offset: usize,
    /// Field storage.
    pub store: ColStore,
    /// Total width of the field in the row, in bytes.
    pub width: usize,
}

/// Parsed binary table over an HDU's data area.
pub struct BinTable<'a> {
    /// Columns, in file order.
    pub columns: Vec<Column>,
    /// Bytes per row (NAXIS1).
    pub row_len: usize,
    /// Number of rows (NAXIS2).
    pub nrows: usize,
    data: &'a [u8],
    heap: &'a [u8],
}

/// Parse a TFORMn value like `162E`, `9J`, `1PB(257)`, `E`, `1QB`.
fn parse_tform(tform: &str) -> Result<ColStore, FitsError> {
    let t = tform.trim().to_uppercase();
    // Strip a trailing "(...)" (max size hint for P/Q columns).
    let t = match t.find('(') {
        Some(p) => t[..p].to_string(),
        None => t,
    };
    let t = t.as_str();
    let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let (count_str, rest) = t.split_at(digits);
    let repeat: usize = if count_str.is_empty() {
        1
    } else {
        count_str
            .parse()
            .map_err(|_| FitsError::Corrupt(format!("bad TFORM '{tform}'")))?
    };
    let (variable, type_str) = if let Some(r) = rest.strip_prefix('P') {
        (Some(4usize), r)
    } else if let Some(r) = rest.strip_prefix('Q') {
        (Some(8usize), r)
    } else {
        (None, rest)
    };
    let elem = match type_str {
        "L" => ElemKind::Logical,
        "X" => ElemKind::Bits,
        "B" => ElemKind::Byte,
        "I" => ElemKind::Short,
        "J" => ElemKind::Int,
        "K" => ElemKind::Long,
        "A" => ElemKind::Char,
        "E" => ElemKind::Float,
        "D" => ElemKind::Double,
        "C" => ElemKind::Complex,
        "M" => ElemKind::DComplex,
        // `rP`/`rQ` with no element type is invalid but tolerated as bytes.
        "" if variable.is_some() => ElemKind::Byte,
        _ => return Err(FitsError::Corrupt(format!("bad TFORM '{tform}'"))),
    };
    Ok(match variable {
        Some(4) => ColStore::VarP { elem, repeat },
        Some(_) => ColStore::VarQ { elem, repeat },
        None => ColStore::Fixed { elem, repeat },
    })
}

impl<'a> BinTable<'a> {
    /// Build a table view over an HDU whose `data` slice is `bytes[hdu.data]`.
    pub fn new(hdu: &'a Hdu, bytes: &'a [u8]) -> Result<Self, FitsError> {
        let h = &hdu.header;
        let row_len = h.req_i64("NAXIS1")? as usize;
        let nrows = h.req_i64("NAXIS2")? as usize;
        let tfields = h.req_i64("TFIELDS")?;
        if !(0..=999).contains(&tfields) {
            return Err(FitsError::Corrupt(format!("bad TFIELDS {tfields}")));
        }
        let mut columns = Vec::with_capacity(tfields as usize);
        let mut offset = 0usize;
        for i in 1..=tfields {
            let name = h.opt_str(&format!("TTYPE{i}")).unwrap_or_default();
            let tform = h.req_str(&format!("TFORM{i}"))?;
            let store = parse_tform(&tform)?;
            let width = match store {
                ColStore::Fixed { elem, repeat } => {
                    if elem == ElemKind::Bits {
                        repeat.div_ceil(8)
                    } else {
                        repeat
                            .checked_mul(elem.size())
                            .ok_or(FitsError::Corrupt("column width overflow".into()))?
                    }
                }
                ColStore::VarP { repeat, .. } => repeat * 8,
                ColStore::VarQ { repeat, .. } => repeat * 16,
            };
            columns.push(Column {
                name,
                offset,
                store,
                width,
            });
            offset = offset
                .checked_add(width)
                .ok_or(FitsError::Corrupt("row layout overflow".into()))?;
        }
        if row_len < offset {
            return Err(FitsError::Corrupt(format!(
                "NAXIS1 {row_len} smaller than declared fields {offset}"
            )));
        }
        let data = &bytes[hdu.data.clone()];
        let table_len = row_len
            .checked_mul(nrows)
            .ok_or(FitsError::Corrupt("table size overflow".into()))?;
        if table_len > data.len() {
            return Err(FitsError::Truncated("binary table rows"));
        }
        // Heap starts after the fixed table (+ optional THEAP keyword).
        let theap = h.opt_i64("THEAP").unwrap_or(0).max(0) as usize;
        let heap_start = table_len
            .checked_add(theap)
            .ok_or(FitsError::Corrupt("THEAP overflow".into()))?;
        let heap: &[u8] = data.get(heap_start..).unwrap_or(&[]);
        Ok(BinTable {
            columns,
            row_len,
            nrows,
            data: &data[..table_len],
            heap,
        })
    }

    /// Column by `TTYPE` name (case-insensitive, trimmed).
    pub fn column(&self, name: &str) -> Option<&Column> {
        let want = name.trim();
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(want))
    }

    /// Fixed-format cell bytes (raw, big-endian on the wire).
    pub fn cell_raw(&self, row: usize, col: &Column) -> Result<&'a [u8], FitsError> {
        if row >= self.nrows {
            return Err(FitsError::Corrupt(format!("row {row} out of range")));
        }
        let start = row * self.row_len + col.offset;
        self.data
            .get(start..start + col.width)
            .ok_or(FitsError::Truncated("table cell"))
    }

    /// For a variable-length column: bytes of the element array in the heap.
    /// `elem_size` is the byte size of one element (1 for 'B', 4 for 'J'...).
    pub fn cell_var(&self, row: usize, col: &Column) -> Result<&'a [u8], FitsError> {
        let (elem, desc_len) = match col.store {
            ColStore::VarP { elem, .. } => (elem, 8usize),
            ColStore::VarQ { elem, .. } => (elem, 16),
            ColStore::Fixed { .. } => {
                return Err(FitsError::Corrupt("cell_var on fixed column".into()))
            }
        };
        let raw = self.cell_raw(row, col)?;
        let count = be_usize(&raw[0..desc_len / 2]);
        let off = be_usize(&raw[desc_len / 2..desc_len]);
        let n = count
            .checked_mul(elem.size())
            .and_then(|n| off.checked_add(n))
            .ok_or(FitsError::Corrupt("variable array size overflow".into()))?;
        self.heap
            .get(off..n)
            .ok_or(FitsError::Truncated("variable-length array in heap"))
    }

    /// Decode a numeric cell (fixed or variable) to `Vec<f64>`.
    /// Handles B, I, J, K, E, D element kinds.
    pub fn cell_f64s(&self, row: usize, col: &Column) -> Result<Vec<f64>, FitsError> {
        let (bytes, elem) = match col.store {
            ColStore::Fixed { elem, .. } => (self.cell_raw(row, col)?, elem),
            ColStore::VarP { elem, .. } | ColStore::VarQ { elem, .. } => {
                (self.cell_var(row, col)?, elem)
            }
        };
        decode_numeric(bytes, elem)
    }
}

/// Big-endian unsigned integer from a 4- or 8-byte slice.
fn be_usize(raw: &[u8]) -> usize {
    match raw.len() {
        4 => u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize,
        8 => u64::from_be_bytes([
            raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
        ]) as usize,
        _ => 0,
    }
}

/// Decode a byte buffer of `elem` items to f64 values.
pub fn decode_numeric(bytes: &[u8], elem: ElemKind) -> Result<Vec<f64>, FitsError> {
    macro_rules! chunks {
        ($n:expr, $f:expr) => {{
            if bytes.len() % $n != 0 {
                return Err(FitsError::Corrupt(
                    "numeric buffer is not a whole number of elements".into(),
                ));
            }
            bytes
                .chunks_exact($n)
                .map(|c: &[u8]| $f(c))
                .collect::<Vec<f64>>()
        }};
    }
    Ok(match elem {
        ElemKind::Byte | ElemKind::Char | ElemKind::Logical => {
            bytes.iter().map(|b| f64::from(*b)).collect()
        }
        ElemKind::Short => chunks!(2, |c: &[u8]| f64::from(i16::from_be_bytes([c[0], c[1]]))),
        ElemKind::Int => {
            chunks!(4, |c: &[u8]| f64::from(i32::from_be_bytes([
                c[0], c[1], c[2], c[3]
            ])))
        }
        ElemKind::Long => chunks!(8, |c: &[u8]| {
            i64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f64
        }),
        ElemKind::Float => {
            chunks!(4, |c: &[u8]| f64::from(f32::from_be_bytes([
                c[0], c[1], c[2], c[3]
            ])))
        }
        ElemKind::Double => chunks!(8, |c: &[u8]| f64::from_be_bytes([
            c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]
        ])),
        ElemKind::Bits | ElemKind::Complex | ElemKind::DComplex => {
            return Err(FitsError::Corrupt("unsupported numeric column kind".into()))
        }
    })
}

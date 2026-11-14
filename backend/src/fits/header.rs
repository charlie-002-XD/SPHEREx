//! FITS header card parsing (80-byte card images, classic and HIERARCH keys).

/// Parsed value of a header card.
#[derive(Debug, Clone, PartialEq)]
pub enum CardValue {
    /// `'string'` value (quotes stripped, doubled quotes unescaped, trimmed).
    Str(String),
    /// Logical `T`/`F`.
    Bool(bool),
    /// Integer number.
    Int(i64),
    /// Floating point number (`D` exponents normalised).
    Real(f64),
}

impl CardValue {
    /// Integer access (accepts ints and integral reals).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            CardValue::Int(v) => Some(*v),
            CardValue::Real(v) if v.fract() == 0.0 && v.is_finite() => Some(*v as i64),
            _ => None,
        }
    }

    /// Float access.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CardValue::Int(v) => Some(*v as f64),
            CardValue::Real(v) => Some(*v),
            _ => None,
        }
    }

    /// String access (string cards; numbers/bools are not converted).
    pub fn as_str(&self) -> Option<&str> {
        match self {
            CardValue::Str(v) => Some(v.as_str()),
            _ => None,
        }
    }

    /// Bool access.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            CardValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// Ordered set of header cards of one HDU. Keys preserve first occurrence.
#[derive(Debug, Clone, Default)]
pub struct Header {
    pub(crate) cards: Vec<(String, Option<CardValue>)>,
}

impl Header {
    /// Parse a single 80-byte card. Returns `(key, value)`; `key` is the
    /// raw first-8-bytes keyword, or the long form for `HIERARCH` cards.
    pub(crate) fn parse_card(raw: &[u8]) -> (String, Option<CardValue>) {
        debug_assert_eq!(raw.len(), 80);
        let field = |a: usize, b: usize| {
            std::str::from_utf8(&raw[a..b])
                .unwrap_or("")
                .trim_end_matches('\0')
        };
        let key8 = field(0, 8);
        if key8.starts_with("HIERARCH ") {
            // HIERARCH convention: `HIERARCH KEY WORD = value`.
            let text = field(9, 80);
            if let Some(eq) = text.find('=') {
                let key = text[..eq].trim().to_string();
                return (key, Some(parse_value(&text[eq + 1..])));
            }
        }
        if raw.len() >= 10 && &raw[8..10] == b"= " {
            (key8.trim().to_string(), Some(parse_value(field(10, 80))))
        } else {
            // COMMENT / HISTORY / blank / END and other value-less cards.
            (key8.trim_end().to_string(), None)
        }
    }

    /// Look up a keyword value. Keyword comparison is exact but tolerates
    /// trailing spaces on the caller side.
    pub fn get(&self, key: &str) -> Option<&CardValue> {
        let key = key.trim();
        self.cards
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.as_ref())
    }

    /// All `(key, value)` pairs, in file order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Option<&CardValue>)> {
        self.cards.iter().map(|(k, v)| (k.as_str(), v.as_ref()))
    }

    /// Integer keyword or `MissingKeyword`.
    pub fn req_i64(&self, key: &str) -> Result<i64, crate::fits::error::FitsError> {
        self.get(key)
            .and_then(CardValue::as_i64)
            .ok_or_else(|| crate::fits::error::FitsError::MissingKeyword(key.to_string()))
    }

    /// Float keyword or `MissingKeyword`.
    pub fn req_f64(&self, key: &str) -> Result<f64, crate::fits::error::FitsError> {
        self.get(key)
            .and_then(CardValue::as_f64)
            .ok_or_else(|| crate::fits::error::FitsError::MissingKeyword(key.to_string()))
    }

    /// String keyword or `MissingKeyword`.
    pub fn req_str(&self, key: &str) -> Result<String, crate::fits::error::FitsError> {
        self.get(key)
            .and_then(CardValue::as_str)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| crate::fits::error::FitsError::MissingKeyword(key.to_string()))
    }

    /// Optional helpers.
    pub fn opt_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(CardValue::as_i64)
    }

    /// Optional float.
    pub fn opt_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(CardValue::as_f64)
    }

    /// Optional string (trimmed).
    pub fn opt_str(&self, key: &str) -> Option<String> {
        self.get(key)
            .and_then(CardValue::as_str)
            .map(|s| s.trim().to_string())
    }

    /// Optional bool.
    pub fn opt_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(CardValue::as_bool)
    }

    /// `DETECTOR` may be stored as an integer or as a string (`'1'`).
    pub fn opt_detector(&self) -> Option<u8> {
        if let Some(v) = self.opt_i64("DETECTOR") {
            return u8::try_from(v).ok();
        }
        self.opt_str("DETECTOR")
            .and_then(|s| s.trim().parse::<u8>().ok())
    }

    /// Iterate over keyword names only (used to collect MSKN/MSKD keys).
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.cards.iter().map(|(k, _)| k.as_str())
    }
}

/// Parse the value field of a card (bytes 11..80 for classic cards, or the
/// text after `=` for HIERARCH). Comments are ignored outside strings.
fn parse_value(text: &str) -> CardValue {
    let text = text.trim_start();
    let mut chars = text.char_indices().peekable();
    match chars.peek().map(|(_, c)| *c) {
        Some('\'') => {
            // Quoted string: '' escapes a literal quote.
            let mut out = String::new();
            let mut iter = text[1..].chars().peekable();
            while let Some(c) = iter.next() {
                match c {
                    '\'' if iter.peek() == Some(&'\'') => {
                        iter.next();
                        out.push('\'');
                    }
                    '\'' => break,
                    _ => out.push(c),
                }
            }
            CardValue::Str(out.trim_end().to_string())
        }
        Some('T') | Some('F')
            if text[1..].trim_start().starts_with('/') || text[1..].trim_start().is_empty() =>
        {
            CardValue::Bool(text.starts_with('T'))
        }
        _ => {
            // Numeric token: up to whitespace or '/'.
            let end = text
                .find(|c: char| c.is_whitespace() || c == '/')
                .unwrap_or(text.len());
            let tok = &text[..end];
            let norm = tok.replace(['D', 'd'], "e");
            if let Ok(v) = norm.parse::<i64>() {
                CardValue::Int(v)
            } else if let Ok(v) = norm.parse::<f64>() {
                CardValue::Real(v)
            } else if tok.is_empty() {
                CardValue::Str(String::new())
            } else {
                CardValue::Str(tok.to_string())
            }
        }
    }
}

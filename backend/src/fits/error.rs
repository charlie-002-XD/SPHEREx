//! Typed errors for the FITS parser.
//!
//! `crate::error::AppError` is owned by task b1 and does not exist on this
//! branch yet. The API task (b5) is expected to map `FitsError` into
//! `AppError::Fits` at merge time (see the b3 handoff).

use thiserror::Error;

/// Errors returned by the FITS parser. Never panics on malformed input.
#[derive(Debug, Error)]
pub enum FitsError {
    /// The input does not start with a valid primary HDU.
    #[error("not a FITS file: {0}")]
    NotFits(&'static str),

    /// The input ends before the declared structure (truncated download etc).
    #[error("truncated FITS input: {0}")]
    Truncated(&'static str),

    /// Structurally valid FITS start, but inconsistent/invalid content.
    #[error("corrupt FITS content: {0}")]
    Corrupt(String),

    /// A required header keyword is absent or has an unexpected type.
    #[error("missing or invalid header keyword '{0}'")]
    MissingKeyword(String),

    /// A required extension (looked up by `EXTNAME`) is absent.
    #[error("missing required FITS extension '{name}'; extensions found: {found}")]
    MissingExtension {
        /// `EXTNAME` that was required (e.g. `"IMAGE"`).
        name: &'static str,
        /// Sorted list of the `EXTNAME`s that were present.
        found: String,
    },

    /// An extension exists but its data layout is not what we can consume.
    #[error("bad data in extension '{ext}': {what}")]
    BadData {
        /// `EXTNAME` of the offending HDU.
        ext: String,
        /// Human readable explanation (never raw user input).
        what: String,
    },

    /// Tile-compressed image uses an algorithm we do not implement.
    #[error("unsupported tile compression '{0}'")]
    UnsupportedCompression(String),

    /// Celestial WCS projection other than TAN (with or without SIP).
    #[error("unsupported WCS projection '{0}'")]
    UnsupportedProjection(String),

    /// World coordinate conversion failed (singular matrix, point behind
    /// the tangent plane, ...).
    #[error("WCS conversion failed: {0}")]
    Wcs(&'static str),
}

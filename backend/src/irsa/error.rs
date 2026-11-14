//! Typed errors for the IRSA client.
//!
//! `crate::error::AppError` is owned by task b1; the `From` impl below
//! encodes the mapping agreed for b5 (IRSA unavailable -> 502, empty
//! result -> 404) without editing `error.rs`.

use std::time::Duration;

use thiserror::Error;

use crate::error::AppError;
use crate::fits::FitsError;

/// Errors returned by the IRSA SIA2/cutout client.
#[derive(Debug, Error)]
pub enum IrsaError {
    /// A query coordinate failed validation. Mapped to HTTP 400; the
    /// message never echoes raw user input.
    #[error("{0}")]
    InvalidQuery(String),

    /// Upstream is unreachable, timed out, or kept answering 503/429
    /// after all retries. Mapped to HTTP 502.
    #[error("IRSA unavailable after {attempts} attempt(s): {detail}")]
    Unavailable {
        /// Number of attempts made.
        attempts: usize,
        /// Last failure seen (status or transport error), sanitized.
        detail: String,
    },

    /// Upstream answered a non-retryable HTTP status.
    #[error("IRSA returned HTTP {status}")]
    Http {
        /// HTTP status code.
        status: u16,
    },

    /// The VOTable (or URL structure) could not be understood.
    #[error("could not parse IRSA response: {0}")]
    Parse(String),

    /// No observation covers the requested position. Mapped to HTTP 404.
    #[error("no SPHEREx observations at this position")]
    NoObservations,

    /// A response body exceeded the in-memory cap (default 64 MiB).
    #[error("IRSA response body exceeds the size cap ({limit} bytes)")]
    BodyTooLarge {
        /// Configured cap, in bytes.
        limit: usize,
    },

    /// The downloaded cutout is not a valid SPHEREx MEF file.
    #[error("invalid FITS cutout: {0}")]
    Fits(#[from] FitsError),

    /// Bad configuration (env var value, empty collection list, ...).
    #[error("invalid IRSA client configuration: {0}")]
    Config(String),

    /// Internal invariant failure (e.g. closed semaphore).
    #[error("internal IRSA client error: {0}")]
    Internal(String),
}

/// A single HTTP attempt outcome, classified for the retry loop.
#[derive(Debug)]
pub(crate) enum Attempt {
    /// Body received and within the size cap.
    Ok(Vec<u8>),
    /// Failure worth retrying (503, 429, timeout, transport).
    Retryable(String),
    /// Failure that must abort immediately.
    Fatal(IrsaError),
}

/// Exponential backoff with jitter for attempt `n` (1-based).
///
/// `base * 2^(n-1)` capped at 5 s, plus up to `base` of jitter derived
/// from the wall clock (no `rand` dependency in scope).
pub(crate) fn backoff(base: Duration, attempt: usize) -> Duration {
    let shift = (attempt.saturating_sub(1)).min(4) as u32;
    let expo = base
        .saturating_mul(1u32 << shift)
        .min(Duration::from_secs(5));
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let jitter = base.mul_f64(f64::from(nanos % 1000) / 1000.0);
    expo + jitter
}

impl From<IrsaError> for AppError {
    /// Mapping agreed for b5: `NoObservations` -> 404, retry-exhausted or
    /// 5xx upstream -> 502, `InvalidQuery` -> 400, everything else -> 500.
    fn from(e: IrsaError) -> Self {
        match e {
            IrsaError::InvalidQuery(m) => AppError::invalid_parameter(m),
            IrsaError::NoObservations => AppError::NotFound,
            IrsaError::Unavailable { .. } => AppError::UpstreamUnavailable,
            IrsaError::Http { status: 404 } => AppError::NotFound,
            IrsaError::Http { status } if status >= 500 => AppError::UpstreamUnavailable,
            IrsaError::Http { .. }
            | IrsaError::Parse(_)
            | IrsaError::BodyTooLarge { .. }
            | IrsaError::Fits(_)
            | IrsaError::Config(_)
            | IrsaError::Internal(_) => AppError::Internal,
        }
    }
}

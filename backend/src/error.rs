//! Typed application errors mapped to HTTP responses.
//!
//! Error codes and body shape match `ErrorBody` in
//! `shared/openapi-contract.json`. Messages must describe the violated
//! constraint and must never echo raw user input.

use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

use crate::fits::FitsError;
use crate::science::ScienceError;

/// Errors surfaced through the HTTP API.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A request parameter failed validation (HTTP 400).
    #[error("{0}")]
    InvalidParameter(String),

    /// No SPHEREx observations cover the requested position (HTTP 404).
    #[error("no observations at this position")]
    NotFound,

    /// The client exceeded the request rate limit (HTTP 429).
    #[error("rate limited")]
    RateLimited,

    /// An upstream service (e.g. IRSA) is unavailable (HTTP 502).
    #[error("upstream service unavailable")]
    UpstreamUnavailable,

    /// Unexpected server error (HTTP 500).
    #[error("internal server error")]
    Internal,
}

impl AppError {
    /// Build a 400 error. `message` describes the constraint that failed;
    /// callers must not interpolate raw user input into it.
    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        Self::InvalidParameter(message.into())
    }

    /// HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidParameter(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::UpstreamUnavailable => StatusCode::BAD_GATEWAY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine-readable code, per the API contract.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidParameter(_) => "invalid_parameter",
            Self::NotFound => "not_found",
            Self::RateLimited => "rate_limited",
            Self::UpstreamUnavailable => "upstream_unavailable",
            Self::Internal => "internal",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(json!({
            "error": {
                "code": self.code(),
                "message": self.to_string(),
            }
        }));
        (status, body).into_response()
    }
}

impl From<FitsError> for AppError {
    /// A cutout that fails to parse is malformed upstream data: HTTP 502
    /// (`upstream_unavailable`); parser internals never reach the client.
    fn from(_: FitsError) -> Self {
        Self::UpstreamUnavailable
    }
}

impl From<ScienceError> for AppError {
    /// Caller-fixable conditions map to 400 with a constraint message
    /// (never echoing raw input); empty extraction maps to 404; upstream
    /// WCS trouble maps to 502; grid/shape invariants are internal bugs.
    fn from(e: ScienceError) -> Self {
        match e {
            ScienceError::InvalidAperture { .. } => {
                Self::invalid_parameter("aperture_px: expected one of 1, 3, 5")
            }
            ScienceError::InvalidDetector { .. } => {
                Self::invalid_parameter("detector: out of range [1, 6]")
            }
            ScienceError::WavelengthMismatch { .. } | ScienceError::UnknownWavelength => {
                Self::invalid_parameter("epochs are not compatible for this mode")
            }
            ScienceError::EmptyInput => Self::NotFound,
            ScienceError::Wcs(_) => Self::UpstreamUnavailable,
            ScienceError::ShapeMismatch { .. }
            | ScienceError::GridMismatch
            | ScienceError::InvalidGrid(_) => Self::Internal,
        }
    }
}

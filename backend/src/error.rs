//! Typed application errors mapped to HTTP responses.
//!
//! Error codes and body shape match `ErrorBody` in
//! `shared/openapi-contract.json`. Messages must describe the violated
//! constraint and must never echo raw user input.

use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

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

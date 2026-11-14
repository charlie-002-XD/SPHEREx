//! HTTP API layer (task b5).
//!
//! Routes match `shared/openapi-contract.json` exactly:
//! `GET /api/v1/{health,targets,frames,spectrum}`. Unknown `/api/v1`
//! paths get a JSON `ErrorBody` 404; everything else falls through to
//! `STATIC_DIR` with an SPA fallback to `index.html`.
//!
//! Middleware stack (outermost first):
//! - `Content-Security-Policy: frame-ancestors *` (embedding is a
//!   feature; `X-Frame-Options` is deliberately NOT set so it cannot
//!   deny framing);
//! - request tracing;
//! - strict CORS from `ALLOWED_ORIGINS` (GET only);
//! - gzip compression (`tower-http`);
//! - a request-timeout backstop;
//! - per-IP rate limiting on the API routes (`tower-governor`), which
//!   answers 429 with a contract `ErrorBody`.

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{header, HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::{GovernorError, GovernorLayer};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::warn;

use crate::config::Config;
use crate::error::AppError;
use crate::fits::Frame;
use crate::irsa::{IrsaClient, IrsaError, Observation};

pub mod validation;

mod dto;
mod fetch;
mod frames;
mod params;
mod spectrum;
pub mod targets;

use dto::HealthBody;

/// Upstream frame provider: the slice of
/// [`crate::irsa::IrsaClient`] the API pipeline needs, abstracted so
/// tests can substitute a stub with synthetic fixtures (no network).
pub trait FrameSource: Send + Sync + 'static {
    /// SIA2 cone search merged over the configured collections,
    /// deduplicated by `access_url`.
    fn search(
        &self,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> impl Future<Output = Result<Arc<Vec<Observation>>, IrsaError>> + Send + '_;

    /// Download and decode one cutout for `observation`.
    fn fetch_cutout(
        &self,
        observation: Observation,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> impl Future<Output = Result<Arc<Frame>, IrsaError>> + Send + '_;
}

impl FrameSource for IrsaClient {
    fn search(
        &self,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> impl Future<Output = Result<Arc<Vec<Observation>>, IrsaError>> + Send + '_ {
        IrsaClient::search(self, ra_deg, dec_deg, fov_deg)
    }

    async fn fetch_cutout(
        &self,
        observation: Observation,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> Result<Arc<Frame>, IrsaError> {
        IrsaClient::fetch_cutout(self, &observation, ra_deg, dec_deg, fov_deg).await
    }
}

/// State shared by every handler; cheap to clone (`Arc`/`Copy` fields).
///
/// `Clone` is manual so `S` itself need not be `Clone` (the `Arc`
/// already shares it).
pub struct AppState<S> {
    /// Upstream frame provider.
    pub source: Arc<S>,
    /// SIA2 collections in use (reported by `/health`).
    pub collections: Arc<Vec<String>>,
    /// Directory with the built frontend (`STATIC_DIR`).
    pub static_dir: PathBuf,
    /// Strict CORS allow-list (`ALLOWED_ORIGINS`; empty = deny all
    /// cross-origin requests).
    pub allowed_origins: Arc<Vec<String>>,
    /// Side of the square output raster (`OUTPUT_NPIX`, <= 256).
    pub output_npix: usize,
    /// Hard cap on cutouts fetched per request (`MAX_FRAMES`).
    pub max_frames_cap: usize,
    /// Total upstream-fetch budget per request (`REQUEST_DEADLINE_SECS`).
    pub request_deadline: Duration,
    /// Per-IP rate limit, requests/second (`RATE_LIMIT_RPS`).
    pub rate_limit_rps: u32,
}

impl<S> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            collections: self.collections.clone(),
            static_dir: self.static_dir.clone(),
            allowed_origins: self.allowed_origins.clone(),
            output_npix: self.output_npix,
            max_frames_cap: self.max_frames_cap,
            request_deadline: self.request_deadline,
            rate_limit_rps: self.rate_limit_rps,
        }
    }
}

impl<S> AppState<S> {
    /// Build state from a source and the process [`Config`].
    pub fn from_config(source: S, cfg: &Config) -> Self {
        Self {
            source: Arc::new(source),
            collections: Arc::new(cfg.irsa.collections.clone()),
            static_dir: cfg.static_dir.clone(),
            allowed_origins: Arc::new(cfg.allowed_origins.clone()),
            output_npix: cfg.output_npix,
            max_frames_cap: cfg.irsa.max_frames,
            request_deadline: cfg.request_deadline,
            rate_limit_rps: cfg.rate_limit_rps,
        }
    }
}

/// Assemble the complete router for `state`.
pub fn build_router<S: FrameSource>(state: AppState<S>) -> Router {
    // Per-IP rate limiting on the API only (static assets stay free).
    // SmartIpKeyExtractor honours the forwarded headers a trusted
    // reverse proxy sets and falls back to the peer IP (main always
    // serves with `ConnectInfo`).
    let mut governor = GovernorConfigBuilder::default().key_extractor(SmartIpKeyExtractor);
    governor
        .period(Duration::from_secs_f64(
            1.0 / f64::from(state.rate_limit_rps.max(1)),
        ))
        .burst_size(state.rate_limit_rps.max(1))
        .methods(vec![Method::GET])
        .error_handler(|e: GovernorError| match e {
            GovernorError::TooManyRequests { .. } => AppError::RateLimited.into_response(),
            _ => AppError::Internal.into_response(),
        });
    let governor_config = match governor.finish() {
        Some(c) => c,
        // `finish` fails only on a zero period or burst; both are
        // forced to >= 1 above.
        None => unreachable!("rate limiter period/burst are non-zero"),
    };

    let api = Router::new()
        .route("/health", get(health::<S>))
        .route("/targets", get(targets::get_targets))
        .route("/frames", get(frames::get_frames::<S>))
        .route("/spectrum", get(spectrum::get_spectrum::<S>))
        .fallback(api_fallback)
        .layer(GovernorLayer {
            config: Arc::new(governor_config),
        })
        .with_state(state.clone());

    let origins: Vec<HeaderValue> = state
        .allowed_origins
        .iter()
        .filter_map(|o| match HeaderValue::from_str(o) {
            Ok(v) => Some(v),
            Err(_) => {
                warn!("ignoring invalid ALLOWED_ORIGINS entry");
                None
            }
        })
        .collect();
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_headers(Any)
        .allow_origin(AllowOrigin::list(origins));

    let spa = ServeDir::new(&state.static_dir)
        .fallback(ServeFile::new(state.static_dir.join("index.html")));

    Router::new()
        .nest("/api/v1", api)
        .fallback_service(spa)
        // Backstop timeout: the in-handler deadline replies first.
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            state.request_deadline + Duration::from_secs(15),
        ))
        .layer(CompressionLayer::new().gzip(true))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("frame-ancestors *"),
        ))
}

/// `GET /api/v1/health`.
async fn health<S: FrameSource>(State(state): State<AppState<S>>) -> Json<HealthBody> {
    Json(HealthBody {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        collections: (*state.collections).clone(),
    })
}

/// JSON 404 for anything under `/api/v1` that matched no route.
async fn api_fallback() -> AppError {
    AppError::NotFound
}

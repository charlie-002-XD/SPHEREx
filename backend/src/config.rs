//! Process configuration (task b5): one [`Config`] struct, loaded from
//! environment variables with documented defaults.
//!
//! | Env var                 | Default                 | Meaning |
//! |-------------------------|-------------------------|---------|
//! | `PORT`                  | `8080`                  | Listen port |
//! | `IRSA_BASE_URL`         | see [`IrsaConfig`]      | IRSA origin |
//! | `IRSA_COLLECTIONS`      | see [`IrsaConfig`]      | SIA2 collections |
//! | `IRSA_MAX_CONCURRENCY`  | `3`                     | Upstream parallelism cap |
//! | `MAX_FRAMES`            | `60`                    | Selection cap per query |
//! | `MAX_CACHE_MB`          | `200`                   | Decoded-result cache budget |
//! | `ALLOWED_ORIGINS`       | (empty = strictest)     | Comma-separated CORS origins |
//! | `RATE_LIMIT_RPS`        | `4`                     | Requests/second per client IP |
//! | `STATIC_DIR`            | `frontend/dist`         | Built frontend directory |
//! | `OUTPUT_NPIX`           | `192`                   | Output raster side, `<= 256` |
//! | `REQUEST_DEADLINE_SECS` | `40`                    | Total fetch budget per API request |
//! | `WARM_TARGETS`          | unset (`1` enables)     | Pre-fetch presets at startup |

use std::path::PathBuf;
use std::time::Duration;

use crate::irsa::{IrsaConfig, IrsaError};

/// Default listen port.
pub const DEFAULT_PORT: u16 = 8080;
/// Default per-IP rate limit (requests/second, also the burst size).
pub const DEFAULT_RATE_LIMIT_RPS: u32 = 4;
/// Default directory holding the built frontend.
pub const DEFAULT_STATIC_DIR: &str = "frontend/dist";
/// Default output raster side; payloads stay small for the time-lapse.
pub const DEFAULT_OUTPUT_NPIX: usize = 192;
/// Hard ceiling for `OUTPUT_NPIX` (contract: images at most 256x256).
pub const MAX_OUTPUT_NPIX: usize = 256;
/// Default total fetch budget per API request (D-014).
pub const DEFAULT_DEADLINE_SECS: u64 = 40;

/// Errors while loading [`Config`] from the environment.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// An environment variable had an unusable value.
    #[error("invalid configuration: {0}")]
    Bad(String),
    /// The IRSA sub-configuration failed to load.
    #[error(transparent)]
    Irsa(#[from] IrsaError),
}

/// Server configuration. Every knob has a documented default so the
/// binary runs with no environment set.
#[derive(Debug, Clone)]
pub struct Config {
    /// TCP port the HTTP server binds (`PORT`, default 8080).
    pub port: u16,
    /// IRSA client tuning (`IRSA_*`, `MAX_FRAMES`, `MAX_CACHE_MB`).
    pub irsa: IrsaConfig,
    /// CORS allow-list (`ALLOWED_ORIGINS`, comma-separated). An empty
    /// list is strictest: no cross-origin requests are allowed at all.
    pub allowed_origins: Vec<String>,
    /// Per-IP request rate limit in requests/second (`RATE_LIMIT_RPS`,
    /// default 4). One token is replenished every `1/rps` seconds and the
    /// burst size equals `rps`.
    pub rate_limit_rps: u32,
    /// Directory with the built frontend (`STATIC_DIR`,
    /// default `frontend/dist`). Served with SPA fallback to
    /// `index.html`.
    pub static_dir: PathBuf,
    /// Side of the square output raster (`OUTPUT_NPIX`, default 192,
    /// must be `<= 256`).
    pub output_npix: usize,
    /// Total upstream-fetch budget per request
    /// (`REQUEST_DEADLINE_SECS`, default 40). On expiry the API returns
    /// whatever is already decoded with `truncated = true`.
    pub request_deadline: Duration,
    /// When `WARM_TARGETS=1`, pre-fetch the five preset targets through
    /// the cache at startup.
    pub warm_targets: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            irsa: IrsaConfig::default(),
            allowed_origins: Vec::new(),
            rate_limit_rps: DEFAULT_RATE_LIMIT_RPS,
            static_dir: PathBuf::from(DEFAULT_STATIC_DIR),
            output_npix: DEFAULT_OUTPUT_NPIX,
            request_deadline: Duration::from_secs(DEFAULT_DEADLINE_SECS),
            warm_targets: false,
        }
    }
}

impl Config {
    /// Build from process environment; unset variables keep the
    /// [`Self::default`] values. Returns [`ConfigError`] on unparseable
    /// or inconsistent values.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut cfg = Self::default();
        cfg.irsa = IrsaConfig::from_env()?;
        cfg.port = env_u64("PORT", u64::from(cfg.port))?
            .try_into()
            .map_err(|_| ConfigError::Bad("PORT is out of range (0-65535)".into()))?;
        if let Ok(v) = std::env::var("ALLOWED_ORIGINS") {
            cfg.allowed_origins = v
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        for origin in &cfg.allowed_origins {
            axum::http::HeaderValue::from_str(origin).map_err(|_| {
                ConfigError::Bad("ALLOWED_ORIGINS contains an invalid origin".into())
            })?;
        }
        cfg.rate_limit_rps =
            u32::try_from(env_u64("RATE_LIMIT_RPS", u64::from(cfg.rate_limit_rps))?)
                .map_err(|_| ConfigError::Bad("RATE_LIMIT_RPS is too large".into()))?;
        if let Ok(v) = std::env::var("STATIC_DIR") {
            cfg.static_dir = PathBuf::from(v);
        }
        cfg.output_npix = env_usize("OUTPUT_NPIX", cfg.output_npix)?;
        cfg.request_deadline = Duration::from_secs(env_u64(
            "REQUEST_DEADLINE_SECS",
            cfg.request_deadline.as_secs(),
        )?);
        cfg.warm_targets = std::env::var("WARM_TARGETS")
            .map(|v| {
                matches!(
                    v.trim(),
                    "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
                )
            })
            .unwrap_or(false);
        cfg.validate()?;
        Ok(cfg)
    }

    /// Enforce invariants; called by `from_env`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.rate_limit_rps == 0 {
            return Err(ConfigError::Bad("RATE_LIMIT_RPS must be >= 1".into()));
        }
        if !(1..=MAX_OUTPUT_NPIX).contains(&self.output_npix) {
            return Err(ConfigError::Bad("OUTPUT_NPIX must be in 1..=256".into()));
        }
        if self.request_deadline.is_zero() {
            return Err(ConfigError::Bad(
                "REQUEST_DEADLINE_SECS must be >= 1".into(),
            ));
        }
        if self.static_dir.as_os_str().is_empty() {
            return Err(ConfigError::Bad("STATIC_DIR must not be empty".into()));
        }
        Ok(())
    }
}

fn env_usize(name: &str, default: usize) -> Result<usize, ConfigError> {
    match std::env::var(name) {
        Ok(v) => v
            .parse::<usize>()
            .map_err(|_| ConfigError::Bad(format!("{name} is not a non-negative integer"))),
        Err(_) => Ok(default),
    }
}

fn env_u64(name: &str, default: u64) -> Result<u64, ConfigError> {
    match std::env::var(name) {
        Ok(v) => v
            .parse::<u64>()
            .map_err(|_| ConfigError::Bad(format!("{name} is not a non-negative integer"))),
        Err(_) => Ok(default),
    }
}

//! Configuration for the IRSA client, loadable from environment.
//!
//! | Env var                | Default                              |
//! |------------------------|--------------------------------------|
//! | `IRSA_BASE_URL`        | `https://irsa.ipac.caltech.edu`      |
//! | `IRSA_COLLECTIONS`     | `spherex_dr1,spherex_qr3`            |
//! | `IRSA_MAX_CONCURRENCY` | `3` (IRSA reportedly 503s at 4)      |
//! | `MAX_FRAMES`           | `60`                                 |
//! | `MAX_CACHE_MB`         | `200`                                |
//! | `IRSA_TIMEOUT_SECS`    | `15`                                 |
//! | `IRSA_MAX_ATTEMPTS`    | `3`                                  |
//! | `IRSA_RETRY_BASE_MS`   | `400`                                |
//! | `IRSA_MAX_BODY_MB`     | `64`                                 |

use std::time::Duration;

use super::error::IrsaError;

/// Default IRSA origin.
pub const DEFAULT_BASE_URL: &str = "https://irsa.ipac.caltech.edu";
/// Default SIA2 collections (`_deep` variants are opt-in via env).
pub const DEFAULT_COLLECTIONS: &[&str] = &["spherex_dr1", "spherex_qr3"];
/// Absolute cap on any IRSA response body kept in memory.
const DEFAULT_MAX_BODY: usize = 64 * 1024 * 1024;

/// Tuning knobs for [`crate::irsa::IrsaClient`].
#[derive(Debug, Clone)]
pub struct IrsaConfig {
    /// IRSA origin, e.g. `https://irsa.ipac.caltech.edu` (no trailing
    /// slash required; the SIA2 path `/SIA` is appended).
    pub base_url: String,
    /// SIA2 collections queried for every position, in priority order.
    /// Dedup across collections keeps the first hit per `access_url`.
    pub collections: Vec<String>,
    /// Bound on simultaneous outbound HTTP requests (semaphore).
    pub max_concurrency: usize,
    /// Maximum frames fetched per `fetch_frames` call.
    pub max_frames: usize,
    /// Total decoded-result cache budget, MiB.
    pub cache_max_mb: u64,
    /// Per-request HTTP timeout.
    pub request_timeout: Duration,
    /// Total tries per request including the first (retries = n-1).
    pub max_attempts: usize,
    /// Base of the exponential backoff between retries.
    pub retry_base_delay: Duration,
    /// Reject response bodies larger than this many bytes.
    pub max_body_bytes: usize,
}

impl Default for IrsaConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            collections: DEFAULT_COLLECTIONS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            max_concurrency: 3,
            max_frames: 60,
            cache_max_mb: 200,
            request_timeout: Duration::from_secs(15),
            max_attempts: 3,
            retry_base_delay: Duration::from_millis(400),
            max_body_bytes: DEFAULT_MAX_BODY,
        }
    }
}

impl IrsaConfig {
    /// Build from process environment, falling back to [`Self::default`]
    /// for unset variables. Returns [`IrsaError::Config`] on unparseable
    /// or inconsistent values.
    pub fn from_env() -> Result<Self, IrsaError> {
        let mut cfg = Self::default();
        if let Ok(v) = std::env::var("IRSA_BASE_URL") {
            cfg.base_url = v;
        }
        if let Ok(v) = std::env::var("IRSA_COLLECTIONS") {
            cfg.collections = v
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        cfg.max_concurrency = env_usize("IRSA_MAX_CONCURRENCY", cfg.max_concurrency)?;
        cfg.max_frames = env_usize("MAX_FRAMES", cfg.max_frames)?;
        cfg.cache_max_mb = env_u64("MAX_CACHE_MB", cfg.cache_max_mb)?;
        cfg.request_timeout =
            Duration::from_secs(env_u64("IRSA_TIMEOUT_SECS", cfg.request_timeout.as_secs())?);
        cfg.max_attempts = env_usize("IRSA_MAX_ATTEMPTS", cfg.max_attempts)?;
        cfg.retry_base_delay = Duration::from_millis(env_u64(
            "IRSA_RETRY_BASE_MS",
            cfg.retry_base_delay.as_millis() as u64,
        )?);
        let body_mb = env_u64(
            "IRSA_MAX_BODY_MB",
            (cfg.max_body_bytes / 1024 / 1024) as u64,
        )?;
        cfg.max_body_bytes =
            usize::try_from(body_mb.saturating_mul(1024 * 1024)).unwrap_or(usize::MAX);
        cfg.validate()?;
        Ok(cfg)
    }

    /// Enforce invariants; called by `from_env` and `IrsaClient::new`.
    pub fn validate(&self) -> Result<(), IrsaError> {
        if self.base_url.trim().is_empty() {
            return Err(IrsaError::Config("base URL is empty".into()));
        }
        if self.collections.is_empty() {
            return Err(IrsaError::Config(
                "IRSA_COLLECTIONS produced an empty list".into(),
            ));
        }
        if self.max_concurrency == 0 {
            return Err(IrsaError::Config(
                "IRSA_MAX_CONCURRENCY must be >= 1".into(),
            ));
        }
        if self.max_frames == 0 {
            return Err(IrsaError::Config("MAX_FRAMES must be >= 1".into()));
        }
        if self.max_attempts == 0 {
            return Err(IrsaError::Config("IRSA_MAX_ATTEMPTS must be >= 1".into()));
        }
        if self.max_body_bytes == 0 {
            return Err(IrsaError::Config("IRSA_MAX_BODY_MB must be >= 1".into()));
        }
        Ok(())
    }
}

fn env_usize(name: &str, default: usize) -> Result<usize, IrsaError> {
    match std::env::var(name) {
        Ok(v) => v
            .parse::<usize>()
            .map_err(|_| IrsaError::Config(format!("{name} is not a non-negative integer"))),
        Err(_) => Ok(default),
    }
}

fn env_u64(name: &str, default: u64) -> Result<u64, IrsaError> {
    match std::env::var(name) {
        Ok(v) => v
            .parse::<u64>()
            .map_err(|_| IrsaError::Config(format!("{name} is not a non-negative integer"))),
        Err(_) => Ok(default),
    }
}

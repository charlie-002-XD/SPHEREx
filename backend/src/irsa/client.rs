//! Async IRSA client: bounded concurrency, retries, size-capped
//! streaming downloads and a byte-weighted decoded-result cache.
//!
//! - Concurrency is bounded by a [`tokio::sync::Semaphore`] sized by
//!   `IRSA_MAX_CONCURRENCY` (default 3; IRSA reportedly answers 503 at
//!   4 concurrent cutout requests — see `docs/vault/decisions.md` D-005).
//! - Every request is retried on HTTP 503/429, timeouts and transport
//!   errors with exponential backoff + jitter, up to
//!   `IRSA_MAX_ATTEMPTS` (default 3) tries and a 15 s timeout.
//! - Bodies are streamed into memory only; anything beyond
//!   `IRSA_MAX_BODY_MB` (default 64 MiB) is rejected. No temp files.
//! - Decoded results (SIA observation lists and parsed FITS frames)
//!   live in a single `moka::future::Cache` weighted by bytes and
//!   capped at `MAX_CACHE_MB` (default 200 MiB).

use std::sync::Arc;

use moka::future::Cache;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, warn};

use crate::api::validation::{
    DEC_MAX_DEG, DEC_MIN_DEG, FOV_MAX_DEG, FOV_MIN_DEG, RA_MAX_DEG, RA_MIN_DEG,
};
use crate::fits::{parse_frame, Frame};

use super::config::IrsaConfig;
use super::error::{backoff, Attempt, IrsaError};
use super::sia::{
    cutout_url, dedup_by_url, observations_from_votable, pos_parameter, search_radius_deg,
    select_frames, Observation,
};

/// A preset target to pre-warm the cache with (e.g. the five official
/// challenge targets at startup).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WarmTarget {
    /// Right ascension, degrees.
    pub ra_deg: f64,
    /// Declination, degrees.
    pub dec_deg: f64,
    /// Field of view (square side), degrees.
    pub fov_deg: f64,
}

/// Outcome of [`IrsaClient::warm`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WarmReport {
    /// Targets whose frames were fetched and cached.
    pub succeeded: usize,
    /// Targets that failed (error logged, warming continues).
    pub failed: usize,
    /// Total frames cached across successful targets.
    pub frames: usize,
}

/// A parsed cutout together with the SIA row that produced it.
#[derive(Debug, Clone)]
pub struct CutoutFrame {
    /// The SIA2 row (times, spectral band, source collection).
    pub observation: Observation,
    /// Decoded MEF cutout (IMAGE/FLAGS/VARIANCE/ZODI/WCS-WAVE, WCS).
    /// Shared with the cache; cheap to clone.
    pub frame: Arc<Frame>,
}

/// Result of [`IrsaClient::fetch_frames`].
#[derive(Debug)]
pub struct FrameSet {
    /// Decoded frames, newest first.
    pub frames: Vec<CutoutFrame>,
    /// Observations covering the position before the `MAX_FRAMES` cap.
    pub total_observations: usize,
    /// True when `total_observations` exceeded the cap.
    pub truncated: bool,
}

/// Discriminates cache entries: SIA result lists vs decoded cutouts.
#[derive(Debug, Clone)]
enum Entry {
    /// Deduped observation list for one (position, radius).
    Observations(Arc<Vec<Observation>>),
    /// Decoded cutout for one (position, size, observation).
    Frame(Arc<Frame>),
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum KeyKind {
    /// SIA cone search: (ra, dec, radius) as f64 bits.
    Search { ra: u64, dec: u64, radius: u64 },
    /// Cutout fetch: (ra, dec, size) as f64 bits plus observation id.
    Frame {
        ra: u64,
        dec: u64,
        size: u64,
        obs: u64,
    },
}

/// Cache key: (collection set, kind-specific tuple), per the task spec.
#[derive(Hash, PartialEq, Eq, Clone)]
struct CacheKey {
    /// Sorted, comma-joined collection set this client was built with.
    collections: Arc<str>,
    kind: KeyKind,
}

/// Byte weight of a cached entry (moka weigher). Frames dominate:
/// 4 arrays * 4 B/px plus a flat allowance for WCS-WAVE/WCS/strings.
fn entry_weight(e: &Entry) -> u32 {
    match e {
        Entry::Observations(obs) => {
            let bytes: usize = obs
                .iter()
                .map(|o| {
                    std::mem::size_of::<Observation>()
                        + o.band_name.len()
                        + o.access_url.len()
                        + o.collection.len()
                })
                .sum();
            u32::try_from(bytes).unwrap_or(u32::MAX)
        }
        Entry::Frame(f) => {
            let px = f.image.len() + f.variance.len() + f.zodi.len();
            let bytes = px.saturating_mul(4) + f.flags.len().saturating_mul(4) + 8192;
            u32::try_from(bytes).unwrap_or(u32::MAX)
        }
    }
}

/// Shared, cheaply cloneable client. All clones share the semaphore
/// and the cache.
#[derive(Clone)]
pub struct IrsaClient {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: IrsaConfig,
    http: reqwest::Client,
    sem: Semaphore,
    collections_key: Arc<str>,
    cache: Cache<CacheKey, Entry>,
}

impl IrsaClient {
    /// Build a client from an explicit configuration.
    pub fn new(cfg: IrsaConfig) -> Result<Self, IrsaError> {
        cfg.validate()?;
        let http = reqwest::Client::builder()
            .timeout(cfg.request_timeout)
            .user_agent("spherex-sky-explorer/0.1 (NASA Space Apps 2026)")
            .build()
            .map_err(|e| IrsaError::Config(format!("cannot build HTTP client: {e}")))?;

        let mut collections = cfg.collections.clone();
        collections.sort();
        let collections_key: Arc<str> = Arc::from(collections.join(",").as_str());

        let cache = Cache::builder()
            .max_capacity(cfg.cache_max_mb.saturating_mul(1024 * 1024))
            .weigher(|_k: &CacheKey, v: &Entry| entry_weight(v))
            .build();

        Ok(Self {
            inner: Arc::new(Inner {
                sem: Semaphore::new(cfg.max_concurrency),
                http,
                collections_key,
                cache,
                cfg,
            }),
        })
    }

    /// Build a client from `IRSA_*`/`MAX_*` environment variables.
    pub fn from_env() -> Result<Self, IrsaError> {
        Self::new(IrsaConfig::from_env()?)
    }

    /// The configuration this client was built with.
    pub fn config(&self) -> &IrsaConfig {
        &self.inner.cfg
    }

    /// Defensive validation (the API layer validates earlier; this
    /// guards `warm` and direct use). Bounds come from `api::validation`.
    fn check_query(&self, ra_deg: f64, dec_deg: f64, fov_deg: f64) -> Result<(), IrsaError> {
        for (name, v) in [("ra", ra_deg), ("dec", dec_deg), ("fov", fov_deg)] {
            if !v.is_finite() {
                return Err(IrsaError::InvalidQuery(format!(
                    "{name}: must be a finite number"
                )));
            }
        }
        if !(RA_MIN_DEG..RA_MAX_DEG).contains(&ra_deg) {
            return Err(IrsaError::InvalidQuery(
                "ra: out of range [0, 360) deg".into(),
            ));
        }
        if !(DEC_MIN_DEG..=DEC_MAX_DEG).contains(&dec_deg) {
            return Err(IrsaError::InvalidQuery(
                "dec: out of range [-90, 90] deg".into(),
            ));
        }
        if !(FOV_MIN_DEG..=FOV_MAX_DEG).contains(&fov_deg) {
            return Err(IrsaError::InvalidQuery(
                "fov: out of range [0.01, 0.5] deg".into(),
            ));
        }
        Ok(())
    }

    /// SIA2 cone search over every configured collection, deduped by
    /// `access_url`. Result is cached; an empty answer maps to
    /// [`IrsaError::NoObservations`] and is not cached.
    pub async fn search(
        &self,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> Result<Arc<Vec<Observation>>, IrsaError> {
        self.check_query(ra_deg, dec_deg, fov_deg)?;
        let radius = search_radius_deg(fov_deg);
        let key = CacheKey {
            collections: self.inner.collections_key.clone(),
            kind: KeyKind::Search {
                ra: ra_deg.to_bits(),
                dec: dec_deg.to_bits(),
                radius: radius.to_bits(),
            },
        };
        if let Some(Entry::Observations(hit)) = self.inner.cache.get(&key).await {
            debug!("irsa search cache hit");
            return Ok(hit);
        }

        let sia = format!("{}/SIA", self.inner.cfg.base_url.trim_end_matches('/'));
        let pos = pos_parameter(ra_deg, dec_deg, radius);
        let mut all = Vec::new();
        for collection in &self.inner.cfg.collections {
            let body = self
                .get_with_retry(&sia, &[("COLLECTION", collection.as_str()), ("POS", &pos)])
                .await?;
            let text = String::from_utf8(body)
                .map_err(|_| IrsaError::Parse("SIA response is not UTF-8".into()))?;
            let mut obs = observations_from_votable(&text, collection)?;
            all.append(&mut obs);
        }
        let obs = Arc::new(dedup_by_url(all));
        if obs.is_empty() {
            return Err(IrsaError::NoObservations);
        }
        self.inner
            .cache
            .insert(key, Entry::Observations(obs.clone()))
            .await;
        Ok(obs)
    }

    /// Download and decode one cutout, with cache lookup keyed by
    /// (collection set, ra, dec, size, observation id).
    pub async fn fetch_cutout(
        &self,
        obs: &Observation,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> Result<Arc<Frame>, IrsaError> {
        self.check_query(ra_deg, dec_deg, fov_deg)?;
        let key = CacheKey {
            collections: self.inner.collections_key.clone(),
            kind: KeyKind::Frame {
                ra: ra_deg.to_bits(),
                dec: dec_deg.to_bits(),
                size: fov_deg.to_bits(),
                obs: obs.id_hash(),
            },
        };
        if let Some(Entry::Frame(hit)) = self.inner.cache.get(&key).await {
            return Ok(hit);
        }
        let url = cutout_url(&obs.access_url, ra_deg, dec_deg, fov_deg);
        let body = self.get_with_retry(&url, &[]).await?;
        let frame = Arc::new(parse_frame(&body)?);
        self.inner
            .cache
            .insert(key, Entry::Frame(frame.clone()))
            .await;
        Ok(frame)
    }

    /// Full pipeline: SIA search -> cap at `MAX_FRAMES` (newest first,
    /// evenly spread over time) -> fetch each cutout in parallel under
    /// the semaphore.
    ///
    /// A cutout that fails (stale `access_url`, cutout-service 500 for
    /// an off-edge position, ...) is logged and skipped; the call only
    /// errors when *no* frame could be decoded, surfacing the first
    /// failure.
    pub async fn fetch_frames(
        &self,
        ra_deg: f64,
        dec_deg: f64,
        fov_deg: f64,
    ) -> Result<FrameSet, IrsaError> {
        let observations = self.search(ra_deg, dec_deg, fov_deg).await?;
        let total = observations.len();
        let selected = select_frames(&observations, self.inner.cfg.max_frames);
        let truncated = selected.len() < total;

        let mut set: JoinSet<(usize, Result<Arc<Frame>, IrsaError>)> = JoinSet::new();
        for (i, obs) in selected.iter().enumerate() {
            let inner = self.inner.clone();
            let obs = obs.clone();
            set.spawn(async move {
                let client = IrsaClient { inner };
                let res = client.fetch_cutout(&obs, ra_deg, dec_deg, fov_deg).await;
                (i, res)
            });
        }
        let mut slots: Vec<Option<Arc<Frame>>> = Vec::new();
        slots.resize_with(selected.len(), || None);
        let mut first_err = None;
        while let Some(joined) = set.join_next().await {
            let (i, res) = joined
                .map_err(|e| IrsaError::Internal(format!("cutout task failed to join: {e}")))?;
            match res {
                Ok(frame) => slots[i] = Some(frame),
                Err(e) => {
                    warn!("cutout for {} skipped: {e}", selected[i].access_url);
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                }
            }
        }

        let mut frames = Vec::new();
        for (obs, slot) in selected.into_iter().zip(slots) {
            if let Some(frame) = slot {
                frames.push(CutoutFrame {
                    observation: obs,
                    frame,
                });
            }
        }
        if frames.is_empty() {
            return Err(first_err.unwrap_or(IrsaError::NoObservations));
        }
        Ok(FrameSet {
            frames,
            total_observations: total,
            truncated,
        })
    }

    /// Pre-warm the cache for preset targets. Targets are processed
    /// sequentially (each `fetch_frames` already parallelizes its
    /// cutouts under the semaphore); failures are logged, not fatal.
    pub async fn warm(&self, targets: &[WarmTarget]) -> WarmReport {
        let mut report = WarmReport::default();
        for t in targets {
            match self.fetch_frames(t.ra_deg, t.dec_deg, t.fov_deg).await {
                Ok(set) => {
                    report.succeeded += 1;
                    report.frames += set.frames.len();
                }
                Err(e) => {
                    warn!("cache warm failed for target {t:?}: {e}");
                    report.failed += 1;
                }
            }
        }
        report
    }

    /// One GET with the retry policy: bounded by the semaphore, retried
    /// on 503/429/timeouts/transport errors with exponential backoff +
    /// jitter, at most `cfg.max_attempts` tries.
    async fn get_with_retry(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<u8>, IrsaError> {
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let outcome =
                {
                    let _permit =
                        self.inner.sem.acquire().await.map_err(|_| {
                            IrsaError::Internal("concurrency semaphore closed".into())
                        })?;
                    self.try_get(url, query).await
                };
            match outcome {
                Attempt::Ok(body) => return Ok(body),
                Attempt::Fatal(e) => return Err(e),
                Attempt::Retryable(detail) => {
                    if attempt >= self.inner.cfg.max_attempts {
                        return Err(IrsaError::Unavailable {
                            attempts: attempt,
                            detail,
                        });
                    }
                    let delay = backoff(self.inner.cfg.retry_base_delay, attempt);
                    debug!("irsa attempt {attempt} retryable ({detail}); sleeping {delay:?}");
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Single attempt: send, classify the status, stream the body into
    /// memory under the size cap.
    async fn try_get(&self, url: &str, query: &[(&str, &str)]) -> Attempt {
        let mut resp = match self.inner.http.get(url).query(query).send().await {
            Ok(r) => r,
            Err(e) => {
                let kind = if e.is_timeout() {
                    "timeout"
                } else if e.is_connect() {
                    "connect error"
                } else {
                    "request error"
                };
                return Attempt::Retryable(format!("{kind}: {e}"));
            }
        };
        let status = resp.status();
        if status.as_u16() == 503 || status.as_u16() == 429 {
            return Attempt::Retryable(format!("HTTP {status}"));
        }
        if !status.is_success() {
            return Attempt::Fatal(IrsaError::Http {
                status: status.as_u16(),
            });
        }
        let cap = self.inner.cfg.max_body_bytes;
        if resp.content_length().is_some_and(|len| len > cap as u64) {
            return Attempt::Fatal(IrsaError::BodyTooLarge { limit: cap });
        }
        let mut body = Vec::new();
        loop {
            match resp.chunk().await {
                Ok(Some(chunk)) => {
                    if body.len().saturating_add(chunk.len()) > cap {
                        return Attempt::Fatal(IrsaError::BodyTooLarge { limit: cap });
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(None) => break,
                Err(e) => return Attempt::Retryable(format!("body read error: {e}")),
            }
        }
        Attempt::Ok(body)
    }
}

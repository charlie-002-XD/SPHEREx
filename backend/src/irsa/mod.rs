//! IRSA SIA2 search and cutout client (task b2).
//!
//! Pipeline: [`IrsaClient::search`] issues one SIA2 cone query per
//! configured collection (`GET {IRSA_BASE_URL}/SIA?COLLECTION=<c>&POS=
//! circle <ra> <dec> <radius>`) and merges the VOTable rows into
//! [`Observation`]s deduplicated by `access_url`.
//! [`IrsaClient::fetch_frames`] caps the selection at `MAX_FRAMES`
//! (newest first, evenly spread over time) and downloads each cutout
//! (`{access_url}?center=<ra>,<dec>&size=<fov>deg`, see
//! [`fov_to_cutout_size`]) decoded by [`crate::fits::parse_frame`].
//!
//! Guard rails: a semaphore bounds concurrency (`IRSA_MAX_CONCURRENCY`,
//! default 3); 503/429/timeout/transport failures retry with
//! exponential backoff + jitter (`IRSA_MAX_ATTEMPTS`, default 3 tries;
//! 15 s timeout); bodies stream into memory only and are rejected past
//! 64 MiB; decoded results sit in a byte-weighted moka cache capped at
//! `MAX_CACHE_MB` (default 200 MiB), keyed by (collection set, ra, dec,
//! size, observation id).
//!
//! Errors use [`IrsaError`]; `From<IrsaError> for AppError` encodes the
//! agreed mapping (empty result -> 404, IRSA down -> 502). Detector is
//! not a SIA column: read `frame.header.detector` after parsing.

mod client;
mod config;
mod error;
mod sia;
mod votable;

pub use client::{CutoutFrame, FrameSet, IrsaClient, WarmReport, WarmTarget};
pub use config::{IrsaConfig, DEFAULT_BASE_URL, DEFAULT_COLLECTIONS};
pub use error::IrsaError;
pub use sia::{
    cutout_url, dedup_by_url, em_to_um, fov_to_cutout_size, observations_from_votable,
    pos_parameter, search_radius_deg, select_frames, time_to_mjd, Observation,
};
pub use votable::{parse_votable, Field, Votable};

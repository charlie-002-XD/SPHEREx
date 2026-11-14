//! Deadline-bounded parallel cutout fetching, shared by `/frames` and
//! `/spectrum` (task b5).
//!
//! Mirrors `IrsaClient::fetch_frames`: one task per selected observation,
//! while the client's semaphore still bounds actual upstream
//! concurrency — no extra parallelism is added here. The difference is
//! the total deadline: in-flight requests at expiry are aborted and
//! whatever decoded is returned so the API can answer with
//! `truncated = true` instead of failing outright (D-014).

use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::{sleep_until, Instant};
use tracing::warn;

use crate::fits::Frame;
use crate::irsa::{CutoutFrame, IrsaError, Observation};

use super::FrameSource;

/// Outcome of [`fetch_cutouts`].
pub(crate) struct FetchOutcome {
    /// Successfully decoded cutouts, in selection order.
    pub frames: Vec<CutoutFrame>,
    /// True when the deadline expired with requests still in flight.
    pub deadline_hit: bool,
    /// First per-cutout failure (surfaced when nothing decoded).
    pub first_error: Option<IrsaError>,
}

/// Fetch the cutout of every `selected` observation in parallel until
/// `deadline`. Individual failures are logged and skipped (off-edge
/// positions get HTTP 422 upstream — common); requests still pending at
/// the deadline are aborted.
pub(crate) async fn fetch_cutouts<S: FrameSource>(
    source: &Arc<S>,
    selected: Vec<Observation>,
    ra_deg: f64,
    dec_deg: f64,
    fov_deg: f64,
    deadline: Duration,
) -> FetchOutcome {
    let until = Instant::now() + deadline;
    let mut set: JoinSet<(usize, Result<Arc<Frame>, IrsaError>)> = JoinSet::new();
    for (i, obs) in selected.iter().cloned().enumerate() {
        let src = Arc::clone(source);
        set.spawn(async move { (i, src.fetch_cutout(obs, ra_deg, dec_deg, fov_deg).await) });
    }

    let mut slots: Vec<Option<Arc<Frame>>> = Vec::new();
    slots.resize_with(selected.len(), || None);
    let mut first_error = None;
    let mut deadline_hit = false;
    while !set.is_empty() {
        tokio::select! {
            joined = set.join_next() => {
                match joined {
                    Some(Ok((i, Ok(frame)))) => slots[i] = Some(frame),
                    Some(Ok((_, Err(e)))) => {
                        warn!("cutout skipped: {e}");
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                    Some(Err(e)) => warn!("cutout task failed to join: {e}"),
                    None => break,
                }
            }
            () = sleep_until(until) => {
                deadline_hit = true;
                set.abort_all();
                break;
            }
        }
    }

    let frames = selected
        .into_iter()
        .zip(slots)
        .filter_map(|(observation, slot)| slot.map(|frame| CutoutFrame { observation, frame }))
        .collect();
    FetchOutcome {
        frames,
        deadline_hit,
        first_error,
    }
}

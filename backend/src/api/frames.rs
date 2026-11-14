//! `GET /api/v1/frames` (task b5).
//!
//! Pipeline: validate -> SIA search -> cap at `max_frames` ->
//! deadline-bounded cutout fetch -> optional `detector` filter ->
//! reproject every frame onto `OutputGrid::centered(ra, dec, fov, npix)`
//! -> compose per `mode` (`raw` = reprojected frames, `diff` = each
//! frame minus the first of its detector x wavelength group via
//! [`subtract_epochs`], `coadd` = one inverse-variance co-add per group,
//! `stack` = [`stack_detectors`] per detector array) -> `Frame` DTOs with
//! float32-LE base64 rasters sorted by observation time.

use std::collections::{BTreeMap, HashMap};

use axum::{extract::State, http::Uri, Json};
use tracing::warn;

use crate::error::AppError;
use crate::fits::{flags_to_names, FlagLegend, Frame};
use crate::irsa::{select_frames, CutoutFrame, Observation};
use crate::science::{
    central_wavelength, group_frames, image_to_b64, reproject, stack_detectors, stack_images,
    subtract_epochs, Interp, OutputGrid, ReprojectOptions, ScienceImage, StackOptions,
    DEFAULT_WAVELENGTH_TOLERANCE_UM,
};

use super::dto::{FrameDto, FramesResponseDto, Mode, QueryEcho, WcsDto};
use super::fetch::fetch_cutouts;
use super::params;
use super::{AppState, FrameSource};

/// FNV-1a 64-bit constants (same scheme as `Observation::id`).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `GET /api/v1/frames` handler.
pub(crate) async fn get_frames<S: FrameSource>(
    State(state): State<AppState<S>>,
    uri: Uri,
) -> Result<Json<FramesResponseDto>, AppError> {
    let p = params::frames_params(&uri)?;

    let observations = state
        .source
        .search(p.ra_deg, p.dec_deg, p.fov_deg)
        .await
        .map_err(AppError::from)?;
    let total_observations = observations.len();

    let cap = p.max_frames.min(state.max_frames_cap);
    let selected = select_frames(&observations, cap);
    let fetched = fetch_cutouts(
        &state.source,
        selected,
        p.ra_deg,
        p.dec_deg,
        p.fov_deg,
        state.request_deadline,
    )
    .await;
    let n_decoded = fetched.frames.len();
    if n_decoded == 0 {
        // Contract: a deadline with nothing decoded is an upstream
        // failure (502), not an empty 200.
        return Err(if fetched.deadline_hit {
            AppError::UpstreamUnavailable
        } else {
            fetched
                .first_error
                .map_or(AppError::NotFound, AppError::from)
        });
    }
    let mut cutouts = fetched.frames;
    if let Some(d) = p.detector {
        cutouts.retain(|c| c.frame.header.detector == Some(d));
    }
    let truncated = total_observations > n_decoded;

    // Reproject every decoded frame onto the output grid; zodi
    // subtraction follows the query flag (contract default false).
    let grid = OutputGrid::centered(p.ra_deg, p.dec_deg, p.fov_deg, state.output_npix)
        .map_err(AppError::from)?;
    let ropts = ReprojectOptions {
        interp: Interp::Bilinear,
        subtract_zodi: p.subtract_zodi,
    };
    let mut images = Vec::with_capacity(cutouts.len());
    for c in &cutouts {
        images.push(reproject(&c.frame, &grid, &ropts).map_err(AppError::from)?);
    }

    let outs = compose(p.mode, &cutouts, images)?;
    let mut frames: Vec<FrameDto> = outs
        .iter()
        .filter_map(|o| frame_dto(o, &cutouts, &grid, p.mode))
        .collect();
    frames.sort_by(|a, b| {
        a.obs_mjd_start
            .total_cmp(&b.obs_mjd_start)
            .then(a.detector.cmp(&b.detector))
            .then(a.wavelength_um.total_cmp(&b.wavelength_um))
    });

    let unit = cutouts
        .first()
        .and_then(|c| c.frame.header.unit.clone())
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| "MJy/sr".to_string());

    Ok(Json(FramesResponseDto {
        query: QueryEcho {
            ra_deg: p.ra_deg,
            dec_deg: p.dec_deg,
            fov_deg: p.fov_deg,
            mode: p.mode,
        },
        width: grid.nx,
        height: grid.ny,
        unit,
        frames,
        flag_legend: response_legend(&cutouts),
        truncated,
        total_observations,
    }))
}

/// One output raster plus provenance back into `cutouts`.
struct OutImg {
    /// The raster on the shared output grid.
    img: ScienceImage,
    /// Indices of the contributing cutouts (non-empty).
    members: Vec<usize>,
    /// Member whose observation times stamp the frame when
    /// `span_times` is false (raw/diff: the "later" epoch).
    epoch_member: usize,
    /// True -> `obs_mjd_start/end` span all members (coadd/stack);
    /// false -> the `epoch_member`'s times (raw/diff).
    span_times: bool,
}

/// Compose the reprojected images into output rasters per `mode`.
fn compose(
    mode: Mode,
    cutouts: &[CutoutFrame],
    images: Vec<ScienceImage>,
) -> Result<Vec<OutImg>, AppError> {
    match mode {
        Mode::Raw => Ok(images
            .into_iter()
            .enumerate()
            .map(|(i, img)| OutImg {
                img,
                members: vec![i],
                epoch_member: i,
                span_times: false,
            })
            .collect()),
        Mode::Diff => {
            let mut outs = Vec::new();
            for (_, idx) in group_indices(cutouts)? {
                let reference = idx[0];
                // Members are epoch-sorted: each one is differenced
                // against the first of its group (contract).
                for &k in &idx[1..] {
                    match subtract_epochs(
                        &images[k],
                        &images[reference],
                        DEFAULT_WAVELENGTH_TOLERANCE_UM,
                    ) {
                        Ok(img) => outs.push(OutImg {
                            img,
                            members: vec![k, reference],
                            epoch_member: k,
                            span_times: false,
                        }),
                        Err(e) => warn!("diff epoch skipped: {e}"),
                    }
                }
            }
            Ok(outs)
        }
        Mode::Coadd => {
            let mut outs = Vec::new();
            for (_, idx) in group_indices(cutouts)? {
                let ims: Vec<&ScienceImage> = idx.iter().map(|&i| &images[i]).collect();
                match stack_images(&ims, &StackOptions::default()) {
                    Ok(img) => {
                        let Some(&epoch_member) = idx.last() else {
                            continue;
                        };
                        outs.push(OutImg {
                            img,
                            members: idx.clone(),
                            epoch_member,
                            span_times: true,
                        });
                    }
                    Err(e) => warn!("coadd group skipped: {e}"),
                }
            }
            Ok(outs)
        }
        Mode::Stack => {
            let mut by_det: BTreeMap<u8, Vec<usize>> = BTreeMap::new();
            for (i, c) in cutouts.iter().enumerate() {
                match c.frame.header.detector {
                    Some(d) if (1..=6).contains(&d) => by_det.entry(d).or_default().push(i),
                    _ => {}
                }
            }
            let refs: BTreeMap<u8, Vec<&ScienceImage>> = by_det
                .iter()
                .map(|(&d, idx)| (d, idx.iter().map(|&i| &images[i]).collect()))
                .collect();
            let stacked =
                stack_detectors(&refs, &StackOptions::default()).map_err(AppError::from)?;
            // `combined` is a mixed-wavelength raster with no single
            // detector; the contract requires `detector` in 1..=6, so
            // only the per-detector stacks are emitted.
            let mut outs = Vec::new();
            for (d, img) in stacked.per_detector {
                let Some(idx) = by_det.get(&d) else { continue };
                let Some(&epoch_member) = idx
                    .iter()
                    .max_by(|&&a, &&b| obs_epoch(&cutouts[a]).total_cmp(&obs_epoch(&cutouts[b])))
                else {
                    continue;
                };
                outs.push(OutImg {
                    img,
                    members: idx.clone(),
                    epoch_member,
                    span_times: true,
                });
            }
            Ok(outs)
        }
    }
}

/// Detector x wavelength groups (b4 `group_frames`) mapped back to
/// indices into `cutouts`/`images`.
fn group_indices(cutouts: &[CutoutFrame]) -> Result<Vec<(u8, Vec<usize>)>, AppError> {
    let refs: Vec<&Frame> = cutouts.iter().map(|c| c.frame.as_ref()).collect();
    let pos: HashMap<*const Frame, usize> = cutouts
        .iter()
        .enumerate()
        .map(|(i, c)| (c.frame.as_ref() as *const Frame, i))
        .collect();
    let groups = group_frames(&refs, DEFAULT_WAVELENGTH_TOLERANCE_UM).map_err(AppError::from)?;
    let mut out = Vec::with_capacity(groups.len());
    for g in groups {
        let idx: Vec<usize> = g
            .frames
            .iter()
            .filter_map(|f| pos.get(&(*f as *const Frame)).copied())
            .collect();
        if !idx.is_empty() {
            out.push((g.detector, idx));
        }
    }
    Ok(out)
}

/// Representative epoch of a cutout (mid-exposure MJD).
fn obs_epoch(c: &CutoutFrame) -> f64 {
    let o = &c.observation;
    match (o.t_min_mjd.is_finite(), o.t_max_mjd.is_finite()) {
        (true, true) => f64::midpoint(o.t_min_mjd, o.t_max_mjd),
        (true, false) => o.t_min_mjd,
        _ => o.t_max_mjd,
    }
}

/// Stable id for a computed raster: FNV-1a over the mode byte plus the
/// member access URLs (same scheme as `Observation::id`).
fn derived_id(mode: Mode, members: &[&Observation]) -> String {
    let mut h = FNV_OFFSET;
    h ^= mode as u64;
    h = h.wrapping_mul(FNV_PRIME);
    for o in members {
        for b in o.access_url.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(FNV_PRIME);
        }
    }
    format!("{h:016x}")
}

/// Union of the members' FLAGS legends (bit -> info), first wins.
fn member_legend(cutouts: &[CutoutFrame], members: &[usize]) -> FlagLegend {
    let mut legend = FlagLegend::new();
    for &i in members {
        for (bit, info) in &cutouts[i].frame.header.flag_legend {
            legend.entry(*bit).or_insert_with(|| info.clone());
        }
    }
    legend
}

/// Response-level `flag_legend`: bit number (as string) -> description
/// (or name when the FITS header has no `MSKDnnnn`).
fn response_legend(cutouts: &[CutoutFrame]) -> BTreeMap<String, String> {
    let all: Vec<usize> = (0..cutouts.len()).collect();
    member_legend(cutouts, &all)
        .iter()
        .map(|(bit, fb)| {
            (
                bit.to_string(),
                fb.description.clone().unwrap_or_else(|| fb.name.clone()),
            )
        })
        .collect()
}

/// Bitwise OR of the central 3x3 flag pixels of a raster.
fn central_flag_bits(img: &ScienceImage) -> i32 {
    let (ny, nx) = img.flags.dim();
    let (cx, cy) = (nx / 2, ny / 2);
    let mut bits = 0i32;
    for y in cy.saturating_sub(1)..=(cy + 1).min(ny - 1) {
        for x in cx.saturating_sub(1)..=(cx + 1).min(nx - 1) {
            bits |= img.flags[[y, x]];
        }
    }
    bits
}

/// Serialise one output raster to a contract `Frame`; `None` when the
/// raster cannot satisfy required fields (missing detector or
/// wavelength).
fn frame_dto(
    o: &OutImg,
    cutouts: &[CutoutFrame],
    grid: &OutputGrid,
    mode: Mode,
) -> Option<FrameDto> {
    let detector = o.img.detector.filter(|d| (1..=6).contains(d))?;

    let members: Vec<&Observation> = o.members.iter().map(|&i| &cutouts[i].observation).collect();
    let epoch_obs = &cutouts[o.epoch_member].observation;
    let (obs_mjd_start, obs_mjd_end) = if o.span_times {
        (
            members
                .iter()
                .map(|o| o.t_min_mjd)
                .fold(f64::INFINITY, f64::min),
            members
                .iter()
                .map(|o| o.t_max_mjd)
                .fold(f64::NEG_INFINITY, f64::max),
        )
    } else {
        (epoch_obs.t_min_mjd, epoch_obs.t_max_mjd)
    };

    let wavelength_um = if o.img.wavelength_um.is_finite() {
        o.img.wavelength_um
    } else if o.members.len() == 1 {
        central_wavelength(&cutouts[o.members[0]].frame)
    } else {
        f64::NAN
    };
    if !wavelength_um.is_finite() {
        warn!("frame dropped: wavelength unknown at the raster centre");
        return None;
    }

    let bandwidth_um = if o.members.len() == 1 {
        let f = &cutouts[o.members[0]].frame;
        let (ny, nx) = f.image.dim();
        let b = f
            .wavelength
            .bandwidth_at((nx as f64 + 1.0) / 2.0, (ny as f64 + 1.0) / 2.0);
        b.is_finite().then_some(b)
    } else {
        let lo = members
            .iter()
            .map(|o| o.em_min_um)
            .fold(f64::INFINITY, f64::min);
        let hi = members
            .iter()
            .map(|o| o.em_max_um)
            .fold(f64::NEG_INFINITY, f64::max);
        (lo.is_finite() && hi.is_finite() && hi > lo).then_some(hi - lo)
    };

    let mut cols: Vec<&str> = members.iter().map(|o| o.collection.as_str()).collect();
    cols.sort_unstable();
    cols.dedup();

    let legend = member_legend(cutouts, &o.members);
    // Single-member outputs in raw mode keep the source observation id;
    // every computed raster gets a derived hash instead.
    let id = if o.members.len() == 1 && mode == Mode::Raw {
        epoch_obs.id()
    } else {
        derived_id(mode, &members)
    };

    Some(FrameDto {
        id,
        obs_mjd_start,
        obs_mjd_end,
        detector,
        wavelength_um,
        bandwidth_um,
        collection: cols.join("+"),
        zodi_subtracted: o.img.zodi_subtracted,
        flags: flags_to_names(central_flag_bits(&o.img), &legend),
        wcs: WcsDto::from(grid.wcs),
        data_b64: image_to_b64(&o.img.image),
    })
}

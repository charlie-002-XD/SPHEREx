//! `GET /api/v1/targets`: the five official preset targets (task b5).
//!
//! Values are kept in sync with `frontend/src/lib/presets.ts`. The
//! sexagesimal strings are parsed through the b1 grammar, so a typo in
//! this table fails loudly at first request (and in the unit test that
//! asserts the resolved degrees) instead of silently serving bad data.

use axum::Json;

use crate::api::validation::{parse_coordinates, validate_query_params};
use crate::error::AppError;
use crate::irsa::WarmTarget;

use super::dto::TargetDto;

/// One preset row (sexagesimal coordinates as in `presets.ts`).
struct Preset {
    /// Stable machine id.
    id: &'static str,
    /// Display name (`label` in the frontend table).
    name: &'static str,
    /// RA, sexagesimal hours.
    ra: &'static str,
    /// Dec, sexagesimal degrees.
    dec: &'static str,
    /// Suggested field of view, degrees.
    fov_deg: f64,
    /// Optional human note.
    note: Option<&'static str>,
}

/// The five official challenge targets; same values as
/// `frontend/src/lib/presets.ts`.
const PRESETS: &[Preset] = &[
    Preset {
        id: "barnards-star",
        name: "Barnard's Star",
        ra: "17h57m48.5s",
        dec: "+04d41m36s",
        fov_deg: 0.1,
        note: Some("Highest proper-motion star known"),
    },
    Preset {
        id: "epsilon-indi-b",
        name: "Epsilon Indi B",
        ra: "22h04m10.5s",
        dec: "-56d46m58s",
        fov_deg: 0.05,
        note: None,
    },
    Preset {
        id: "wise-0855",
        name: "WISE 0855",
        ra: "08h55m10.831s",
        dec: "-07d14m42.53s",
        fov_deg: 0.05,
        note: Some("Official challenge coordinates"),
    },
    Preset {
        id: "rw-aurigae",
        name: "RW Aurigae",
        ra: "05h07m49.5s",
        dec: "+30d24m05s",
        fov_deg: 0.05,
        note: None,
    },
    Preset {
        id: "r-draconis",
        name: "R Draconis",
        ra: "16h32m40.2s",
        dec: "+66d45m17s",
        fov_deg: 0.05,
        note: None,
    },
];

/// Parsed presets as contract `Target`s. `Err` is unreachable (the
/// table is constants through the validated grammar) and maps to 500.
pub fn preset_targets() -> Result<Vec<TargetDto>, AppError> {
    PRESETS
        .iter()
        .map(|p| {
            let (ra_deg, dec_deg) = parse_coordinates(p.ra, p.dec)?;
            validate_query_params(ra_deg, dec_deg, p.fov_deg)?;
            Ok(TargetDto {
                id: p.id.to_string(),
                name: p.name.to_string(),
                ra_deg,
                dec_deg,
                fov_deg: p.fov_deg,
                note: p.note.map(str::to_string),
            })
        })
        .collect()
}

/// Presets as [`WarmTarget`]s for `IrsaClient::warm` (startup cache
/// pre-fetch, `WARM_TARGETS=1`).
pub fn warm_targets() -> Result<Vec<WarmTarget>, AppError> {
    Ok(preset_targets()?
        .iter()
        .map(|t| WarmTarget {
            ra_deg: t.ra_deg,
            dec_deg: t.dec_deg,
            fov_deg: t.fov_deg,
        })
        .collect())
}

/// `GET /api/v1/targets` handler.
pub(crate) async fn get_targets() -> Result<Json<Vec<TargetDto>>, AppError> {
    preset_targets().map(Json)
}

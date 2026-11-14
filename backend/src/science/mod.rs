//! Science engine (task b4): detector/wavelength grouping, WCS
//! reprojection onto a common grid, inverse-variance stacking, epoch
//! subtraction/co-addition, aperture spectra and temporal statistics for
//! moving-object (blink/difference) modes.
//!
//! Typical pipeline for the API layer (b5):
//! 1. [`group_frames`] raw [`Frame`](crate::fits::Frame)s by detector and
//!    central wavelength (`DEFAULT_WAVELENGTH_TOLERANCE_UM` = 0.05 um);
//! 2. [`reproject`] each `Frame` onto a shared [`OutputGrid`]
//!    (`OutputGrid::centered` on the query, or `OutputGrid::from_frame`);
//! 3. combine with [`stack_images`]/[`stack_detectors`],
//!    [`subtract_epochs`]/[`coadd_epochs`], [`extract_spectrum`] or
//!    [`temporal_stats`];
//! 4. serialise rasters with [`image_to_b64`] (float32 LE, row-major).
//!
//! Conventions inherited from `crate::fits`: 1-based FITS pixel
//! coordinates in public APIs; `Array2[y][x]`; flag-bit meanings come
//! from the FITS `FLAGS` legend, never hardcoded (see
//! [`mask_from_names`]). `IMAGE` is not zodi-subtracted; `subtract_zodi`
//! options default to on for difference/stack modes.

mod encode;
mod epoch;
mod error;
mod grid;
mod image;
mod reproject;
mod spectrum;

pub use encode::{image_to_b64, image_to_le_bytes};
pub use epoch::{coadd_epochs, subtract_epochs, temporal_stats, TemporalStats};
pub use error::ScienceError;
pub use grid::{LinearTanWcs, OutputGrid};
pub use image::{
    central_wavelength, frame_epoch, group_frames, mask_from_names, stack_detectors, stack_images,
    FrameGroup, ScienceImage, StackOptions, StackResult, DEFAULT_WAVELENGTH_TOLERANCE_UM,
};
pub use reproject::{reproject, Interp, ReprojectOptions};
pub use spectrum::{extract_spectrum, SpectrumPoint};

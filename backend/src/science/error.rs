//! Typed errors for the science engine.
//!
//! Task b5 maps `ScienceError` into `AppError` (e.g. `ShapeMismatch` /
//! `GridMismatch` / `WavelengthMismatch` -> `invalid_parameter`,
//! `Wcs` -> `internal`). Messages never echo raw user input.

use thiserror::Error;

use crate::fits::FitsError;

/// Errors returned by `crate::science`.
#[derive(Debug, Error)]
pub enum ScienceError {
    /// No usable inputs (empty frame/image lists).
    #[error("no input images")]
    EmptyInput,

    /// Inputs have different array shapes.
    #[error("array shape mismatch: {a:?} vs {b:?}")]
    ShapeMismatch {
        /// `[nx, ny]` of the first input.
        a: [usize; 2],
        /// `[nx, ny]` of the second input.
        b: [usize; 2],
    },

    /// Same shape but different sky grids (WCS): callers must reproject
    /// onto a common `OutputGrid` before epoch arithmetic.
    #[error("epochs are not on the same pixel grid; reproject onto a common OutputGrid first")]
    GridMismatch,

    /// Epoch central wavelengths differ by more than the tolerance.
    #[error(
        "wavelength mismatch: {wave_a_um:.4} um vs {wave_b_um:.4} um exceeds tolerance {tolerance_um:.4} um"
    )]
    WavelengthMismatch {
        /// Central wavelength of the first epoch (um).
        wave_a_um: f64,
        /// Central wavelength of the second epoch (um).
        wave_b_um: f64,
        /// Tolerance that was exceeded (um).
        tolerance_um: f64,
    },

    /// Wavelength is unknown (NaN) on at least one epoch, so the
    /// wavelength-tolerance check cannot be performed.
    #[error("cannot compare epochs: wavelength is unknown")]
    UnknownWavelength,

    /// Detector key outside the `1..=6` range.
    #[error("invalid detector number {detector}: expected 1..=6")]
    InvalidDetector {
        /// Offending detector number.
        detector: u8,
    },

    /// Aperture size must be a positive odd integer (contract: 1, 3, 5).
    #[error("invalid aperture size {aperture_px} px: expected a positive odd integer")]
    InvalidAperture {
        /// Offending value.
        aperture_px: u32,
    },

    /// Bad `OutputGrid` construction parameters.
    #[error("invalid output grid: {0}")]
    InvalidGrid(&'static str),

    /// Sky-WCS conversion failed while reprojecting or locating a source.
    #[error(transparent)]
    Wcs(#[from] FitsError),
}

//! Raster serialisation for the JSON API: row-major float32
//! little-endian, base64 — the `data_b64` field of `Frame` in
//! `shared/openapi-contract.json`. NaN is allowed in the payload.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use ndarray::Array2;

/// Row-major little-endian f32 bytes of `image` (`width*height` items).
pub fn image_to_le_bytes(image: &Array2<f32>) -> Vec<u8> {
    let mut out = Vec::with_capacity(image.len() * 4);
    for v in image.iter() {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Base64 of [`image_to_le_bytes`].
pub fn image_to_b64(image: &Array2<f32>) -> String {
    STANDARD.encode(image_to_le_bytes(image))
}

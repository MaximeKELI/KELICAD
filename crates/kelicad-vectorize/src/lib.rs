//! Deterministic SVG/PNG → CAD geometries (ADR 0010).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod error;
mod png;
mod svg;

pub use error::VectorizeError;
pub use png::{vectorize_png, PngOptions};
pub use svg::vectorize_svg;

use kelicad_doc::Geometry;

/// Detect format from bytes / filename hint and vectorize.
pub fn vectorize_bytes(
    bytes: &[u8],
    filename_hint: Option<&str>,
) -> Result<Vec<Geometry>, VectorizeError> {
    let hint = filename_hint.unwrap_or("").to_ascii_lowercase();
    if hint.ends_with(".svg") || looks_like_svg(bytes) {
        return vectorize_svg(bytes);
    }
    if hint.ends_with(".png") || looks_like_png(bytes) {
        return vectorize_png(bytes, PngOptions::default());
    }
    Err(VectorizeError::UnsupportedFormat)
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = std::str::from_utf8(&bytes[..bytes.len().min(256)]).unwrap_or("");
    let t = head.trim_start();
    t.starts_with("<?xml") || t.starts_with("<svg") || t.contains("<svg")
}

fn looks_like_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
}

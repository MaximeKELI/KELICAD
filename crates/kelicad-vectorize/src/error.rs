//! Vectorize errors.

/// Errors from SVG/PNG vectorization.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum VectorizeError {
    /// Input is not valid UTF-8 SVG text.
    #[error("invalid SVG text")]
    InvalidSvgText,
    /// Unsupported or unrecognized format.
    #[error("unsupported image format")]
    UnsupportedFormat,
    /// PNG decode failure.
    #[error("PNG decode failed: {0}")]
    PngDecode(String),
    /// No geometry extracted.
    #[error("no geometry extracted")]
    Empty,
}

//! I/O errors.

use thiserror::Error;

/// Errors reading or writing `.kelicad` files.
#[derive(Debug, Error)]
pub enum IoError {
    /// SQLite error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// JSON geometry / meta.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid or unsupported file.
    #[error("{0}")]
    Format(String),
    /// UUID parse.
    #[error("uuid: {0}")]
    Uuid(#[from] uuid::Error),
    /// I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

//! DWG bridge errors.

/// Errors from DWG import/export.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DwgError {
    /// Bytes are not a supported DWG/proxy payload.
    #[error("unsupported DWG format: {0}")]
    Unsupported(String),
    /// Underlying DXF I/O failed.
    #[error("DXF: {0}")]
    Dxf(String),
    /// External worker failed.
    #[error("DWG worker: {0}")]
    Worker(String),
    /// I/O.
    #[error("I/O: {0}")]
    Io(String),
}

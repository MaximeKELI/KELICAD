//! Solid kernel errors.

/// Errors from solid construction / boolean / section.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SolidError {
    /// Profile is open or degenerate.
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    /// Parameter out of range.
    #[error("invalid argument: {0}")]
    InvalidArg(String),
    /// Boolean / CSG failed.
    #[error("boolean failed: {0}")]
    Boolean(String),
}

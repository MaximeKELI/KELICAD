//! BIM errors.

/// BIM / IFC errors.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BimError {
    /// Invalid construction parameters.
    #[error("{0}")]
    Invalid(String),
    /// IFC parse / export failure.
    #[error("IFC: {0}")]
    Ifc(String),
    /// Solid kernel error.
    #[error(transparent)]
    Solid(#[from] kelicad_solid::SolidError),
}

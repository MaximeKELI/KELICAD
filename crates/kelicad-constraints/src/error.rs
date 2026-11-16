//! Constraint errors.

use kelicad_doc::EntityId;

/// Errors from the constraint system / solver.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ConstraintError {
    /// Entity is missing or not a line.
    #[error("entity {0} is not a line")]
    NotALine(EntityId),
    /// Unknown constraint id.
    #[error("unknown constraint {0}")]
    UnknownConstraint(uuid::Uuid),
    /// Invalid argument.
    #[error("{0}")]
    Invalid(String),
    /// Formula parse / eval error.
    #[error("formula: {0}")]
    Formula(String),
}

//! Entity identifiers and kinds.

use uuid::Uuid;

/// Stable entity identifier (UUID v7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(Uuid);

impl EntityId {
    /// Create from a UUID (typically v7).
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// High-level entity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    /// Point.
    Point,
    /// Line segment.
    Line,
    /// Circular arc.
    Arc,
    /// Full circle.
    Circle,
    /// Polyline (including rectangles).
    Polyline,
}

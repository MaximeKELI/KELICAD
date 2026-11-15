//! Constraint kinds and identifiers.

use kelicad_doc::EntityId;
use uuid::Uuid;

/// Stable constraint identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ConstraintId(pub Uuid);

impl ConstraintId {
    /// Allocate a new id (v7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for ConstraintId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConstraintId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Which endpoint of a line segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    /// Line start.
    Start,
    /// Line end.
    End,
}

/// A geometric constraint referencing line entities.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Constraint {
    /// Two endpoints share the same position.
    Coincident {
        /// First entity.
        a: EntityId,
        /// First endpoint.
        a_end: Endpoint,
        /// Second entity.
        b: EntityId,
        /// Second endpoint.
        b_end: Endpoint,
    },
    /// Line is horizontal (equal Y).
    Horizontal {
        /// Target line.
        entity: EntityId,
    },
    /// Line is vertical (equal X).
    Vertical {
        /// Target line.
        entity: EntityId,
    },
    /// Two lines are parallel.
    Parallel {
        /// First line.
        a: EntityId,
        /// Second line.
        b: EntityId,
    },
    /// Two lines are perpendicular.
    Perpendicular {
        /// First line.
        a: EntityId,
        /// Second line.
        b: EntityId,
    },
    /// Two lines have equal length.
    EqualLength {
        /// First line.
        a: EntityId,
        /// Second line.
        b: EntityId,
    },
    /// Driving length constraint.
    Distance {
        /// Target line.
        entity: EntityId,
        /// Required length (must be > 0).
        value: f64,
    },
}

impl Constraint {
    /// Entity ids referenced by this constraint.
    pub fn entity_ids(&self) -> Vec<EntityId> {
        match self {
            Self::Coincident { a, b, .. } => vec![*a, *b],
            Self::Horizontal { entity } | Self::Vertical { entity } | Self::Distance { entity, .. } => {
                vec![*entity]
            }
            Self::Parallel { a, b }
            | Self::Perpendicular { a, b }
            | Self::EqualLength { a, b } => vec![*a, *b],
        }
    }
}

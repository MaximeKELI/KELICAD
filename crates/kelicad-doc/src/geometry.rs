//! Geometry component attached to entities.

use kelicad_geom::{Arc, Circle, Line};
use kelicad_math::Vec2;

use crate::entity::EntityKind;

/// Geometry payload for an entity.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// Single point.
    Point(Vec2),
    /// Line segment.
    Line(Line),
    /// Arc.
    Arc(Arc),
    /// Circle.
    Circle(Circle),
}

impl Geometry {
    /// Entity kind for this geometry.
    pub fn kind(&self) -> EntityKind {
        match self {
            Self::Point(_) => EntityKind::Point,
            Self::Line(_) => EntityKind::Line,
            Self::Arc(_) => EntityKind::Arc,
            Self::Circle(_) => EntityKind::Circle,
        }
    }
}

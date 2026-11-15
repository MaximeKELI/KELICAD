//! Geometry component attached to entities.

use kelicad_geom::transform::{
    mirror_arc, mirror_circle, mirror_line, mirror_polyline, rotate_arc, rotate_circle, rotate_line,
    rotate_polyline, scale_arc, scale_circle, scale_line, scale_polyline, translate_arc,
    translate_circle, translate_line, translate_polyline,
};
use kelicad_geom::{Arc, Circle, Line, Polyline};
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
    /// Polyline / rectangle.
    Polyline(Polyline),
}

impl Geometry {
    /// Entity kind for this geometry.
    pub fn kind(&self) -> EntityKind {
        match self {
            Self::Point(_) => EntityKind::Point,
            Self::Line(_) => EntityKind::Line,
            Self::Arc(_) => EntityKind::Arc,
            Self::Circle(_) => EntityKind::Circle,
            Self::Polyline(_) => EntityKind::Polyline,
        }
    }

    /// Translate geometry.
    pub fn translated(&self, delta: Vec2) -> Self {
        match self {
            Self::Point(p) => Self::Point(*p + delta),
            Self::Line(l) => Self::Line(translate_line(*l, delta)),
            Self::Arc(a) => Self::Arc(translate_arc(*a, delta)),
            Self::Circle(c) => Self::Circle(translate_circle(*c, delta)),
            Self::Polyline(p) => Self::Polyline(translate_polyline(p, delta)),
        }
    }

    /// Rotate about origin.
    pub fn rotated(&self, origin: Vec2, radians: f64) -> Self {
        match self {
            Self::Point(p) => {
                Self::Point(origin + (*p - origin).rotate(radians))
            }
            Self::Line(l) => Self::Line(rotate_line(*l, origin, radians)),
            Self::Arc(a) => Self::Arc(rotate_arc(*a, origin, radians)),
            Self::Circle(c) => Self::Circle(rotate_circle(*c, origin, radians)),
            Self::Polyline(p) => Self::Polyline(rotate_polyline(p, origin, radians)),
        }
    }

    /// Scale about origin.
    pub fn scaled(&self, origin: Vec2, factor: f64) -> Self {
        match self {
            Self::Point(p) => Self::Point(origin + (*p - origin) * factor),
            Self::Line(l) => Self::Line(scale_line(*l, origin, factor)),
            Self::Arc(a) => Self::Arc(scale_arc(*a, origin, factor)),
            Self::Circle(c) => Self::Circle(scale_circle(*c, origin, factor)),
            Self::Polyline(p) => Self::Polyline(scale_polyline(p, origin, factor)),
        }
    }

    /// Mirror across line ab.
    pub fn mirrored(&self, a: Vec2, b: Vec2) -> Self {
        use kelicad_geom::transform::mirror_point;
        match self {
            Self::Point(p) => Self::Point(mirror_point(*p, a, b)),
            Self::Line(l) => Self::Line(mirror_line(*l, a, b)),
            Self::Arc(ar) => Self::Arc(mirror_arc(*ar, a, b)),
            Self::Circle(c) => Self::Circle(mirror_circle(*c, a, b)),
            Self::Polyline(p) => Self::Polyline(mirror_polyline(p, a, b)),
        }
    }
}

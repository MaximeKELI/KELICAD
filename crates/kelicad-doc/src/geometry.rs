//! Geometry component attached to entities.

use kelicad_geom::transform::{
    mirror_arc, mirror_circle, mirror_line, mirror_point, mirror_polyline, rotate_arc,
    rotate_circle, rotate_line, rotate_point, rotate_polyline, scale_arc, scale_circle, scale_line,
    scale_point, scale_polyline, translate_arc, translate_circle, translate_line,
    translate_polyline,
};
use kelicad_geom::{Arc, Circle, Line, Polyline};
use kelicad_math::Vec2;

use crate::annotation::{DimensionGeom, HatchGeom, MTextGeom, TextGeom};
use crate::blocks::InsertGeom;
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
    /// Single-line text.
    Text(TextGeom),
    /// Multi-line text.
    MText(MTextGeom),
    /// Dimension.
    Dimension(DimensionGeom),
    /// Hatch.
    Hatch(HatchGeom),
    /// Block insert.
    Insert(InsertGeom),
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
            Self::Text(_) => EntityKind::Text,
            Self::MText(_) => EntityKind::MText,
            Self::Dimension(_) => EntityKind::Dimension,
            Self::Hatch(_) => EntityKind::Hatch,
            Self::Insert(_) => EntityKind::Insert,
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
            Self::Text(t) => {
                let mut n = t.clone();
                n.position += delta;
                Self::Text(n)
            }
            Self::MText(t) => {
                let mut n = t.clone();
                n.position += delta;
                Self::MText(n)
            }
            Self::Dimension(d) => {
                let mut n = d.clone();
                n.def1 += delta;
                n.def2 += delta;
                n.dim_line += delta;
                Self::Dimension(n)
            }
            Self::Hatch(h) => {
                let mut n = h.clone();
                n.boundary = n.boundary.iter().map(|p| *p + delta).collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position += delta;
                Self::Insert(n)
            }
        }
    }

    /// Rotate about origin.
    pub fn rotated(&self, origin: Vec2, radians: f64) -> Self {
        match self {
            Self::Point(p) => Self::Point(rotate_point(*p, origin, radians)),
            Self::Line(l) => Self::Line(rotate_line(*l, origin, radians)),
            Self::Arc(a) => Self::Arc(rotate_arc(*a, origin, radians)),
            Self::Circle(c) => Self::Circle(rotate_circle(*c, origin, radians)),
            Self::Polyline(p) => Self::Polyline(rotate_polyline(p, origin, radians)),
            Self::Text(t) => {
                let mut n = t.clone();
                n.position = rotate_point(n.position, origin, radians);
                n.rotation += radians;
                Self::Text(n)
            }
            Self::MText(t) => {
                let mut n = t.clone();
                n.position = rotate_point(n.position, origin, radians);
                n.rotation += radians;
                Self::MText(n)
            }
            Self::Dimension(d) => {
                let mut n = d.clone();
                n.def1 = rotate_point(n.def1, origin, radians);
                n.def2 = rotate_point(n.def2, origin, radians);
                n.dim_line = rotate_point(n.dim_line, origin, radians);
                Self::Dimension(n)
            }
            Self::Hatch(h) => {
                let mut n = h.clone();
                n.boundary = n
                    .boundary
                    .iter()
                    .map(|p| rotate_point(*p, origin, radians))
                    .collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = rotate_point(n.position, origin, radians);
                n.rotation += radians;
                Self::Insert(n)
            }
        }
    }

    /// Scale about origin.
    pub fn scaled(&self, origin: Vec2, factor: f64) -> Self {
        match self {
            Self::Point(p) => Self::Point(scale_point(*p, origin, factor)),
            Self::Line(l) => Self::Line(scale_line(*l, origin, factor)),
            Self::Arc(a) => Self::Arc(scale_arc(*a, origin, factor)),
            Self::Circle(c) => Self::Circle(scale_circle(*c, origin, factor)),
            Self::Polyline(p) => Self::Polyline(scale_polyline(p, origin, factor)),
            Self::Text(t) => {
                let mut n = t.clone();
                n.position = scale_point(n.position, origin, factor);
                n.height *= factor.abs();
                Self::Text(n)
            }
            Self::MText(t) => {
                let mut n = t.clone();
                n.position = scale_point(n.position, origin, factor);
                n.height *= factor.abs();
                n.width *= factor.abs();
                Self::MText(n)
            }
            Self::Dimension(d) => {
                let mut n = d.clone();
                n.def1 = scale_point(n.def1, origin, factor);
                n.def2 = scale_point(n.def2, origin, factor);
                n.dim_line = scale_point(n.dim_line, origin, factor);
                Self::Dimension(n)
            }
            Self::Hatch(h) => {
                let mut n = h.clone();
                n.boundary = n
                    .boundary
                    .iter()
                    .map(|p| scale_point(*p, origin, factor))
                    .collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = scale_point(n.position, origin, factor);
                n.scale = Vec2::new(n.scale.x * factor, n.scale.y * factor);
                Self::Insert(n)
            }
        }
    }

    /// Mirror across line ab.
    pub fn mirrored(&self, a: Vec2, b: Vec2) -> Self {
        match self {
            Self::Point(p) => Self::Point(mirror_point(*p, a, b)),
            Self::Line(l) => Self::Line(mirror_line(*l, a, b)),
            Self::Arc(ar) => Self::Arc(mirror_arc(*ar, a, b)),
            Self::Circle(c) => Self::Circle(mirror_circle(*c, a, b)),
            Self::Polyline(p) => Self::Polyline(mirror_polyline(p, a, b)),
            Self::Text(t) => {
                let mut n = t.clone();
                n.position = mirror_point(n.position, a, b);
                Self::Text(n)
            }
            Self::MText(t) => {
                let mut n = t.clone();
                n.position = mirror_point(n.position, a, b);
                Self::MText(n)
            }
            Self::Dimension(d) => {
                let mut n = d.clone();
                n.def1 = mirror_point(n.def1, a, b);
                n.def2 = mirror_point(n.def2, a, b);
                n.dim_line = mirror_point(n.dim_line, a, b);
                Self::Dimension(n)
            }
            Self::Hatch(h) => {
                let mut n = h.clone();
                n.boundary = n.boundary.iter().map(|p| mirror_point(*p, a, b)).collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = mirror_point(n.position, a, b);
                Self::Insert(n)
            }
        }
    }
}

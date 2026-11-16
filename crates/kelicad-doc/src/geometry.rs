//! Geometry component attached to entities.

use kelicad_geom::transform::{
    mirror_arc, mirror_circle, mirror_ellipse, mirror_line, mirror_point, mirror_polyline,
    mirror_spline, rotate_arc, rotate_circle, rotate_ellipse, rotate_line, rotate_point,
    rotate_polyline, rotate_spline, scale_arc, scale_circle, scale_ellipse, scale_line, scale_point,
    scale_polyline, scale_spline, translate_arc, translate_circle, translate_ellipse, translate_line,
    translate_polyline, translate_spline,
};
use kelicad_geom::{Arc, Circle, Ellipse, Line, Polyline, Spline};
use kelicad_math::Vec2;

use crate::annotation::{DimensionGeom, HatchGeom, LeaderGeom, MTextGeom, TextGeom};
use crate::blocks::InsertGeom;
use crate::entity::EntityKind;
use crate::ray::{RayGeom, XLineGeom};
use crate::xref::XrefInsertGeom;

/// Geometry payload for an entity.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
    /// Ellipse.
    Ellipse(Ellipse),
    /// Spline.
    Spline(Spline),
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
    /// Leader.
    Leader(LeaderGeom),
    /// Ray.
    Ray(RayGeom),
    /// Infinite xline.
    XLine(XLineGeom),
    /// XREF / underlay insert.
    Xref(XrefInsertGeom),
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
            Self::Ellipse(_) => EntityKind::Ellipse,
            Self::Spline(_) => EntityKind::Spline,
            Self::Text(_) => EntityKind::Text,
            Self::MText(_) => EntityKind::MText,
            Self::Dimension(_) => EntityKind::Dimension,
            Self::Hatch(_) => EntityKind::Hatch,
            Self::Insert(_) => EntityKind::Insert,
            Self::Leader(_) => EntityKind::Leader,
            Self::Ray(_) => EntityKind::Ray,
            Self::XLine(_) => EntityKind::XLine,
            Self::Xref(_) => EntityKind::Xref,
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
            Self::Ellipse(e) => Self::Ellipse(translate_ellipse(*e, delta)),
            Self::Spline(s) => Self::Spline(translate_spline(s, delta)),
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
                n.islands = n
                    .islands
                    .iter()
                    .map(|ring| ring.iter().map(|p| *p + delta).collect())
                    .collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position += delta;
                Self::Insert(n)
            }
            Self::Leader(l) => {
                let mut n = l.clone();
                n.vertices = n.vertices.iter().map(|p| *p + delta).collect();
                Self::Leader(n)
            }
            Self::Ray(r) => Self::Ray(RayGeom {
                origin: r.origin + delta,
                direction: r.direction,
            }),
            Self::XLine(x) => Self::XLine(XLineGeom {
                point: x.point + delta,
                direction: x.direction,
            }),
            Self::Xref(x) => {
                let mut n = x.clone();
                n.position += delta;
                Self::Xref(n)
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
            Self::Ellipse(e) => Self::Ellipse(rotate_ellipse(*e, origin, radians)),
            Self::Spline(s) => Self::Spline(rotate_spline(s, origin, radians)),
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
                n.islands = n
                    .islands
                    .iter()
                    .map(|ring| {
                        ring.iter()
                            .map(|p| rotate_point(*p, origin, radians))
                            .collect()
                    })
                    .collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = rotate_point(n.position, origin, radians);
                n.rotation += radians;
                Self::Insert(n)
            }
            Self::Leader(l) => {
                let mut n = l.clone();
                n.vertices = n
                    .vertices
                    .iter()
                    .map(|p| rotate_point(*p, origin, radians))
                    .collect();
                Self::Leader(n)
            }
            Self::Ray(r) => Self::Ray(RayGeom {
                origin: rotate_point(r.origin, origin, radians),
                direction: rotate_point(r.direction, Vec2::ZERO, radians),
            }),
            Self::XLine(x) => Self::XLine(XLineGeom {
                point: rotate_point(x.point, origin, radians),
                direction: rotate_point(x.direction, Vec2::ZERO, radians),
            }),
            Self::Xref(x) => {
                let mut n = x.clone();
                n.position = rotate_point(n.position, origin, radians);
                n.rotation += radians;
                Self::Xref(n)
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
            Self::Ellipse(e) => Self::Ellipse(scale_ellipse(*e, origin, factor)),
            Self::Spline(s) => Self::Spline(scale_spline(s, origin, factor)),
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
                n.islands = n
                    .islands
                    .iter()
                    .map(|ring| {
                        ring.iter()
                            .map(|p| scale_point(*p, origin, factor))
                            .collect()
                    })
                    .collect();
                n.scale *= factor.abs();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = scale_point(n.position, origin, factor);
                n.scale = Vec2::new(n.scale.x * factor, n.scale.y * factor);
                Self::Insert(n)
            }
            Self::Leader(l) => {
                let mut n = l.clone();
                n.vertices = n
                    .vertices
                    .iter()
                    .map(|p| scale_point(*p, origin, factor))
                    .collect();
                n.text_height *= factor.abs();
                Self::Leader(n)
            }
            Self::Ray(r) => Self::Ray(RayGeom {
                origin: scale_point(r.origin, origin, factor),
                direction: r.direction,
            }),
            Self::XLine(x) => Self::XLine(XLineGeom {
                point: scale_point(x.point, origin, factor),
                direction: x.direction,
            }),
            Self::Xref(x) => {
                let mut n = x.clone();
                n.position = scale_point(n.position, origin, factor);
                n.scale = Vec2::new(n.scale.x * factor, n.scale.y * factor);
                Self::Xref(n)
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
            Self::Ellipse(e) => Self::Ellipse(mirror_ellipse(*e, a, b)),
            Self::Spline(s) => Self::Spline(mirror_spline(s, a, b)),
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
                n.islands = n
                    .islands
                    .iter()
                    .map(|ring| ring.iter().map(|p| mirror_point(*p, a, b)).collect())
                    .collect();
                Self::Hatch(n)
            }
            Self::Insert(i) => {
                let mut n = i.clone();
                n.position = mirror_point(n.position, a, b);
                Self::Insert(n)
            }
            Self::Leader(l) => {
                let mut n = l.clone();
                n.vertices = n.vertices.iter().map(|p| mirror_point(*p, a, b)).collect();
                Self::Leader(n)
            }
            Self::Ray(r) => {
                let o2 = mirror_point(r.origin, a, b);
                let tip = mirror_point(r.origin + r.direction, a, b);
                let d = tip - o2;
                let len = d.length().max(1e-12);
                Self::Ray(RayGeom {
                    origin: o2,
                    direction: d / len,
                })
            }
            Self::XLine(x) => {
                let p2 = mirror_point(x.point, a, b);
                let tip = mirror_point(x.point + x.direction, a, b);
                let d = tip - p2;
                let len = d.length().max(1e-12);
                Self::XLine(XLineGeom {
                    point: p2,
                    direction: d / len,
                })
            }
            Self::Xref(x) => {
                let mut n = x.clone();
                n.position = mirror_point(n.position, a, b);
                Self::Xref(n)
            }
        }
    }
}

//! Axis-aligned bounds helpers for selection windows.

use kelicad_geom::Aabb;
use kelicad_math::Vec2;

use crate::geometry::Geometry;

/// Approximate AABB of a geometry (for window select).
pub fn geometry_aabb(g: &Geometry) -> Aabb {
    match g {
        Geometry::Point(p) => Aabb::from_points(*p, *p),
        Geometry::Line(l) => Aabb::from_points(l.start, l.end),
        Geometry::Circle(c) => Aabb::from_points(
            Vec2::new(c.center.x - c.radius, c.center.y - c.radius),
            Vec2::new(c.center.x + c.radius, c.center.y + c.radius),
        ),
        Geometry::Arc(a) => Aabb::EMPTY
            .expand(a.start_point())
            .expand(a.end_point())
            .expand(a.center),
        Geometry::Polyline(p) => {
            let mut b = Aabb::EMPTY;
            for v in &p.vertices {
                b = b.expand(*v);
            }
            b
        }
        Geometry::Ellipse(e) => {
            let poly = e.to_polyline(24);
            let mut b = Aabb::EMPTY;
            for v in &poly.vertices {
                b = b.expand(*v);
            }
            b
        }
        Geometry::Spline(s) => {
            let poly = s.to_polyline(8);
            let mut b = Aabb::EMPTY;
            for v in &poly.vertices {
                b = b.expand(*v);
            }
            b
        }
        Geometry::Text(t) => {
            Aabb::from_points(t.position, t.position + Vec2::new(t.height * 4.0, t.height))
        }
        Geometry::MText(t) => {
            Aabb::from_points(t.position, t.position + Vec2::new(t.width.max(1.0), t.height))
        }
        Geometry::Dimension(d) => Aabb::EMPTY
            .expand(d.def1)
            .expand(d.def2)
            .expand(d.dim_line),
        Geometry::Hatch(h) => {
            let mut b = Aabb::EMPTY;
            for v in &h.boundary {
                b = b.expand(*v);
            }
            b
        }
        Geometry::Insert(i) => Aabb::from_points(
            i.position - Vec2::new(0.5, 0.5),
            i.position + Vec2::new(0.5, 0.5),
        ),
        Geometry::Leader(l) => {
            let mut b = Aabb::EMPTY;
            for v in &l.vertices {
                b = b.expand(*v);
            }
            b
        }
        Geometry::Ray(r) => Aabb::from_points(r.origin, r.origin + r.direction * 10.0),
        Geometry::XLine(x) => {
            Aabb::from_points(x.point - x.direction * 10.0, x.point + x.direction * 10.0)
        }
        Geometry::Xref(x) => Aabb::from_points(x.position, x.position + Vec2::new(1.0, 1.0)),
    }
}

fn aabb_valid(b: &Aabb) -> bool {
    b.min.x.is_finite()
        && b.min.y.is_finite()
        && b.max.x.is_finite()
        && b.max.y.is_finite()
        && b.min.x <= b.max.x
        && b.min.y <= b.max.y
}

/// True if AABB intersects the selection rectangle (axis-aligned).
pub fn aabb_intersects_rect(b: &Aabb, min: Vec2, max: Vec2) -> bool {
    if !aabb_valid(b) {
        return false;
    }
    b.max.x >= min.x && b.min.x <= max.x && b.max.y >= min.y && b.min.y <= max.y
}

/// True if AABB is fully inside the rectangle (window select, AutoCAD-style).
pub fn aabb_inside_rect(b: &Aabb, min: Vec2, max: Vec2) -> bool {
    if !aabb_valid(b) {
        return false;
    }
    b.min.x >= min.x && b.max.x <= max.x && b.min.y >= min.y && b.max.y <= max.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_geom::Line;

    #[test]
    fn line_aabb() {
        let g = Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(2.0, 3.0)));
        let b = geometry_aabb(&g);
        assert!((b.min.x - 0.0).abs() < 1e-9);
        assert!((b.max.y - 3.0).abs() < 1e-9);
    }
}

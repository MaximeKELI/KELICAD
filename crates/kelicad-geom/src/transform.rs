//! Rigid / similarity transforms on primitives.

use std::f64::consts::PI;

use kelicad_math::Vec2;

use crate::arc::Arc;
use crate::circle::Circle;
use crate::line::Line;
use crate::polyline::Polyline;

/// Translate a point.
#[inline]
pub fn translate_point(p: Vec2, delta: Vec2) -> Vec2 {
    p + delta
}

/// Rotate a point around `origin` by `radians`.
pub fn rotate_point(p: Vec2, origin: Vec2, radians: f64) -> Vec2 {
    origin + (p - origin).rotate(radians)
}

/// Scale a point about `origin`.
pub fn scale_point(p: Vec2, origin: Vec2, factor: f64) -> Vec2 {
    origin + (p - origin) * factor
}

/// Mirror a point across the infinite line through `a` and `b`.
pub fn mirror_point(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 == 0.0 {
        return p;
    }
    let t = (p - a).dot(ab) / len2;
    let proj = a + ab * t;
    proj * 2.0 - p
}

/// Translate a line.
pub fn translate_line(line: Line, delta: Vec2) -> Line {
    Line::new(
        translate_point(line.start, delta),
        translate_point(line.end, delta),
    )
}

/// Rotate a line.
pub fn rotate_line(line: Line, origin: Vec2, radians: f64) -> Line {
    Line::new(
        rotate_point(line.start, origin, radians),
        rotate_point(line.end, origin, radians),
    )
}

/// Scale a line.
pub fn scale_line(line: Line, origin: Vec2, factor: f64) -> Line {
    Line::new(
        scale_point(line.start, origin, factor),
        scale_point(line.end, origin, factor),
    )
}

/// Mirror a line.
pub fn mirror_line(line: Line, a: Vec2, b: Vec2) -> Line {
    Line::new(mirror_point(line.start, a, b), mirror_point(line.end, a, b))
}

/// Translate a circle.
pub fn translate_circle(c: Circle, delta: Vec2) -> Circle {
    Circle::new(translate_point(c.center, delta), c.radius)
}

/// Rotate a circle (center only).
pub fn rotate_circle(c: Circle, origin: Vec2, radians: f64) -> Circle {
    Circle::new(rotate_point(c.center, origin, radians), c.radius)
}

/// Scale a circle (radius scales by |factor|).
pub fn scale_circle(c: Circle, origin: Vec2, factor: f64) -> Circle {
    Circle::new(
        scale_point(c.center, origin, factor),
        c.radius * factor.abs(),
    )
}

/// Mirror a circle.
pub fn mirror_circle(c: Circle, a: Vec2, b: Vec2) -> Circle {
    Circle::new(mirror_point(c.center, a, b), c.radius)
}

/// Translate an arc.
pub fn translate_arc(arc: Arc, delta: Vec2) -> Arc {
    Arc {
        center: translate_point(arc.center, delta),
        ..arc
    }
}

/// Rotate an arc (angles shift by the same rotation).
pub fn rotate_arc(arc: Arc, origin: Vec2, radians: f64) -> Arc {
    Arc {
        center: rotate_point(arc.center, origin, radians),
        start_angle: arc.start_angle + radians,
        end_angle: arc.end_angle + radians,
        ..arc
    }
}

/// Scale an arc about origin.
pub fn scale_arc(arc: Arc, origin: Vec2, factor: f64) -> Arc {
    Arc {
        center: scale_point(arc.center, origin, factor),
        radius: arc.radius * factor.abs(),
        clockwise: if factor < 0.0 {
            !arc.clockwise
        } else {
            arc.clockwise
        },
        ..arc
    }
}

/// Mirror an arc across a line (reverses orientation).
pub fn mirror_arc(arc: Arc, a: Vec2, b: Vec2) -> Arc {
    let center = mirror_point(arc.center, a, b);
    let axis = (b - a).angle();
    // Reflect angles around the mirror axis direction.
    let reflect = |ang: f64| 2.0 * axis - ang;
    Arc {
        center,
        radius: arc.radius,
        start_angle: reflect(arc.start_angle),
        end_angle: reflect(arc.end_angle),
        clockwise: !arc.clockwise,
    }
}

/// Translate polyline vertices.
pub fn translate_polyline(poly: &Polyline, delta: Vec2) -> Polyline {
    Polyline {
        vertices: poly
            .vertices
            .iter()
            .map(|&p| translate_point(p, delta))
            .collect(),
        closed: poly.closed,
    }
}

/// Rotate polyline.
pub fn rotate_polyline(poly: &Polyline, origin: Vec2, radians: f64) -> Polyline {
    Polyline {
        vertices: poly
            .vertices
            .iter()
            .map(|&p| rotate_point(p, origin, radians))
            .collect(),
        closed: poly.closed,
    }
}

/// Scale polyline.
pub fn scale_polyline(poly: &Polyline, origin: Vec2, factor: f64) -> Polyline {
    Polyline {
        vertices: poly
            .vertices
            .iter()
            .map(|&p| scale_point(p, origin, factor))
            .collect(),
        closed: poly.closed,
    }
}

/// Mirror polyline.
pub fn mirror_polyline(poly: &Polyline, a: Vec2, b: Vec2) -> Polyline {
    Polyline {
        vertices: poly
            .vertices
            .iter()
            .map(|&p| mirror_point(p, a, b))
            .collect(),
        closed: poly.closed,
    }
}

/// Offset a line to a parallel segment at signed distance (left = positive).
pub fn offset_line(line: Line, distance: f64) -> Option<Line> {
    let d = line.delta();
    let len = d.length();
    if len == 0.0 {
        return None;
    }
    let n = d.perp() / len;
    Some(Line::new(
        line.start + n * distance,
        line.end + n * distance,
    ))
}

/// Degrees helper for command input.
pub fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_math::Tolerance;

    #[test]
    fn rotate_90_around_origin() {
        let p = rotate_point(Vec2::X, Vec2::ZERO, PI / 2.0);
        assert!(p.nearly_eq(Vec2::Y, Tolerance::DEFAULT));
    }

    #[test]
    fn mirror_across_x_axis() {
        let p = mirror_point(Vec2::new(1.0, 2.0), Vec2::ZERO, Vec2::X);
        assert!(p.nearly_eq(Vec2::new(1.0, -2.0), Tolerance::DEFAULT));
    }

    #[test]
    fn offset_line_unit() {
        let line = Line::new(Vec2::ZERO, Vec2::X);
        let off = offset_line(line, 1.0).unwrap();
        assert!(off.start.nearly_eq(Vec2::new(0.0, 1.0), Tolerance::DEFAULT));
        assert!(off.end.nearly_eq(Vec2::new(1.0, 1.0), Tolerance::DEFAULT));
    }
}

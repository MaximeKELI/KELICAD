//! Pairwise intersections for M0 primitives.

use kelicad_math::{Tolerance, Vec2};

use crate::circle::Circle;
use crate::line::Line;

/// Result of a segment–segment intersection query.
#[derive(Debug, Clone, PartialEq)]
pub enum LineLineResult {
    /// No intersection (parallel disjoint, or segments miss).
    None,
    /// Single point intersection (proper or at endpoint).
    Point(Vec2),
    /// Collinear overlapping region (inclusive endpoints of the overlap).
    Overlap {
        /// Overlap start.
        a: Vec2,
        /// Overlap end.
        b: Vec2,
    },
}

/// Result of a segment–circle intersection.
#[derive(Debug, Clone, PartialEq)]
pub enum LineCircleResult {
    /// No intersection.
    None,
    /// One intersection (tangent or single crossing on the segment).
    One(Vec2),
    /// Two intersections on the segment.
    Two(Vec2, Vec2),
}

/// Result of a circle–circle intersection.
#[derive(Debug, Clone, PartialEq)]
pub enum CircleCircleResult {
    /// No intersection (separate or one inside the other without touch).
    None,
    /// Circles coincide (infinite intersections).
    Coincident,
    /// Single tangent point.
    One(Vec2),
    /// Two intersection points.
    Two(Vec2, Vec2),
}

/// Intersect two finite segments.
pub fn line_line(a: Line, b: Line, tol: Tolerance) -> LineLineResult {
    if a.is_degenerate(tol) || b.is_degenerate(tol) {
        return LineLineResult::None;
    }

    let r = a.delta();
    let s = b.delta();
    let denom = r.cross(s);
    let qp = b.start - a.start;

    if tol.is_zero(denom) {
        // Parallel.
        if !tol.is_zero(qp.cross(r)) {
            return LineLineResult::None;
        }
        // Collinear — project onto `r`.
        let r_len2 = r.length_squared();
        let mut t0 = qp.dot(r) / r_len2;
        let mut t1 = t0 + s.dot(r) / r_len2;
        if t0 > t1 {
            core::mem::swap(&mut t0, &mut t1);
        }
        let lo = t0.max(0.0);
        let hi = t1.min(1.0);
        if hi < lo - tol.abs / r.length().max(1.0) {
            return LineLineResult::None;
        }
        let pa = a.point_at(lo.clamp(0.0, 1.0));
        let pb = a.point_at(hi.clamp(0.0, 1.0));
        if pa.nearly_eq(pb, tol) {
            LineLineResult::Point(pa)
        } else {
            LineLineResult::Overlap { a: pa, b: pb }
        }
    } else {
        let t = qp.cross(s) / denom;
        let u = qp.cross(r) / denom;
        if t < -tol.abs || t > 1.0 + tol.abs || u < -tol.abs || u > 1.0 + tol.abs {
            return LineLineResult::None;
        }
        LineLineResult::Point(a.point_at(t.clamp(0.0, 1.0)))
    }
}

/// Intersect a finite segment with a circle.
pub fn line_circle(line: Line, circle: Circle, tol: Tolerance) -> LineCircleResult {
    if circle.is_degenerate(tol) || line.is_degenerate(tol) {
        return LineCircleResult::None;
    }

    let d = line.delta();
    let f = line.start - circle.center;
    let a = d.dot(d);
    let b = 2.0 * f.dot(d);
    let c = f.dot(f) - circle.radius * circle.radius;
    let disc = b * b - 4.0 * a * c;

    if disc < -tol.epsilon(circle.radius * circle.radius) {
        return LineCircleResult::None;
    }

    let sqrt_disc = disc.max(0.0).sqrt();
    let inv = 1.0 / (2.0 * a);
    let t1 = (-b - sqrt_disc) * inv;
    let t2 = (-b + sqrt_disc) * inv;

    let mut pts = Vec::new();
    for t in [t1, t2] {
        if t >= -tol.abs && t <= 1.0 + tol.abs {
            let p = line.point_at(t.clamp(0.0, 1.0));
            if pts
                .iter()
                .all(|q: &Vec2| !q.nearly_eq(p, tol))
            {
                pts.push(p);
            }
        }
    }

    match pts.len() {
        0 => LineCircleResult::None,
        1 => LineCircleResult::One(pts[0]),
        _ => LineCircleResult::Two(pts[0], pts[1]),
    }
}

/// Intersect two circles.
pub fn circle_circle(a: Circle, b: Circle, tol: Tolerance) -> CircleCircleResult {
    if a.is_degenerate(tol) || b.is_degenerate(tol) {
        return CircleCircleResult::None;
    }

    let diff = b.center - a.center;
    let dist = diff.length();

    if tol.is_zero(dist) && tol.eq(a.radius, b.radius) {
        return CircleCircleResult::Coincident;
    }
    if tol.is_zero(dist) {
        return CircleCircleResult::None;
    }

    if dist > a.radius + b.radius + tol.abs {
        return CircleCircleResult::None;
    }
    if dist < (a.radius - b.radius).abs() - tol.abs {
        return CircleCircleResult::None;
    }

    let dir = diff / dist;
    let x = (dist * dist - b.radius * b.radius + a.radius * a.radius) / (2.0 * dist);
    let h2 = a.radius * a.radius - x * x;

    let mid = a.center + dir * x;
    if h2 < -tol.epsilon(a.radius * a.radius) {
        return CircleCircleResult::None;
    }
    if h2.abs() <= tol.epsilon(a.radius * a.radius) {
        return CircleCircleResult::One(mid);
    }

    let h = h2.sqrt();
    let perp = dir.perp() * h;
    CircleCircleResult::Two(mid + perp, mid - perp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn crossing_segments() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let b = Line::new(Vec2::new(0.0, 2.0), Vec2::new(2.0, 0.0));
        match line_line(a, b, Tolerance::DEFAULT) {
            LineLineResult::Point(p) => {
                assert!(p.nearly_eq(Vec2::new(1.0, 1.0), Tolerance::DEFAULT));
            }
            other => panic!("expected point, got {other:?}"),
        }
    }

    #[test]
    fn parallel_disjoint() {
        let a = Line::new(Vec2::ZERO, Vec2::X);
        let b = Line::new(Vec2::Y, Vec2::new(1.0, 1.0));
        assert_eq!(line_line(a, b, Tolerance::DEFAULT), LineLineResult::None);
    }

    #[test]
    fn diameter_segment_circle() {
        let line = Line::new(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0));
        let circle = Circle::new(Vec2::ZERO, 1.0);
        match line_circle(line, circle, Tolerance::DEFAULT) {
            LineCircleResult::Two(p, q) => {
                assert!(p.nearly_eq(Vec2::new(-1.0, 0.0), Tolerance::DEFAULT)
                    || q.nearly_eq(Vec2::new(-1.0, 0.0), Tolerance::DEFAULT));
                assert!(p.nearly_eq(Vec2::new(1.0, 0.0), Tolerance::DEFAULT)
                    || q.nearly_eq(Vec2::new(1.0, 0.0), Tolerance::DEFAULT));
            }
            other => panic!("expected two points, got {other:?}"),
        }
    }

    #[test]
    fn two_circles_unit() {
        let a = Circle::new(Vec2::ZERO, 1.0);
        let b = Circle::new(Vec2::new(1.0, 0.0), 1.0);
        match circle_circle(a, b, Tolerance::DEFAULT) {
            CircleCircleResult::Two(p, q) => {
                assert!(p.nearly_eq(Vec2::new(0.5, 0.866_025_403_78), Tolerance::COARSE)
                    || q.nearly_eq(Vec2::new(0.5, 0.866_025_403_78), Tolerance::COARSE));
            }
            other => panic!("expected two points, got {other:?}"),
        }
    }

    proptest! {
        #[test]
        fn line_line_symmetric(
            ax0 in -50f64..50.0, ay0 in -50f64..50.0,
            ax1 in -50f64..50.0, ay1 in -50f64..50.0,
            bx0 in -50f64..50.0, by0 in -50f64..50.0,
            bx1 in -50f64..50.0, by1 in -50f64..50.0,
        ) {
            let a = Line::new(Vec2::new(ax0, ay0), Vec2::new(ax1, ay1));
            let b = Line::new(Vec2::new(bx0, by0), Vec2::new(bx1, by1));
            let r1 = line_line(a, b, Tolerance::DEFAULT);
            let r2 = line_line(b, a, Tolerance::DEFAULT);
            match (r1, r2) {
                (LineLineResult::None, LineLineResult::None) => {}
                (LineLineResult::Point(p), LineLineResult::Point(q)) => {
                    prop_assert!(p.nearly_eq(q, Tolerance::COARSE));
                }
                (LineLineResult::Overlap { .. }, LineLineResult::Overlap { .. }) => {}
                (LineLineResult::Overlap { .. }, LineLineResult::Point(_))
                | (LineLineResult::Point(_), LineLineResult::Overlap { .. }) => {}
                (x, y) => prop_assert!(false, "asymmetric {:?} vs {:?}", x, y),
            }
        }

        #[test]
        fn circle_circle_points_on_both(
            ax in -20f64..20.0, ay in -20f64..20.0, ar in 0.5f64..10.0,
            bx in -20f64..20.0, by in -20f64..20.0, br in 0.5f64..10.0,
        ) {
            let a = Circle::new(Vec2::new(ax, ay), ar);
            let b = Circle::new(Vec2::new(bx, by), br);
            let tol = Tolerance::COARSE;
            match circle_circle(a, b, tol) {
                CircleCircleResult::One(p) | CircleCircleResult::Two(p, _) => {
                    prop_assert!((p.distance(a.center) - a.radius).abs() < 1e-5);
                    prop_assert!((p.distance(b.center) - b.radius).abs() < 1e-5);
                }
                CircleCircleResult::Two(_, q) => {
                    prop_assert!((q.distance(a.center) - a.radius).abs() < 1e-5);
                }
                _ => {}
            }
        }
    }
}

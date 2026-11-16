//! Rigorous intersection / AABB / transform edge cases.

use kelicad_math::{Tolerance, Vec2};
use proptest::prelude::*;

use crate::aabb::Aabb;
use crate::circle::Circle;
use crate::intersect::{
    circle_circle, line_circle, line_line, CircleCircleResult, LineCircleResult, LineLineResult,
};
use crate::line::Line;
use crate::transform::{
    offset_line, rotate_point, scale_point, translate_point,
};

#[test]
fn collinear_overlap_interior() {
    let a = Line::new(Vec2::ZERO, Vec2::new(4.0, 0.0));
    let b = Line::new(Vec2::new(1.0, 0.0), Vec2::new(3.0, 0.0));
    match line_line(a, b, Tolerance::DEFAULT) {
        LineLineResult::Overlap { a: p, b: q } => {
            assert!(p.nearly_eq(Vec2::new(1.0, 0.0), Tolerance::DEFAULT));
            assert!(q.nearly_eq(Vec2::new(3.0, 0.0), Tolerance::DEFAULT));
        }
        other => panic!("expected overlap, got {other:?}"),
    }
}

#[test]
fn collinear_touch_endpoint() {
    let a = Line::new(Vec2::ZERO, Vec2::X);
    let b = Line::new(Vec2::X, Vec2::new(2.0, 0.0));
    match line_line(a, b, Tolerance::DEFAULT) {
        LineLineResult::Point(p) => assert!(p.nearly_eq(Vec2::X, Tolerance::DEFAULT)),
        other => panic!("expected point, got {other:?}"),
    }
}

#[test]
fn collinear_disjoint() {
    let a = Line::new(Vec2::ZERO, Vec2::X);
    let b = Line::new(Vec2::new(2.0, 0.0), Vec2::new(3.0, 0.0));
    assert_eq!(line_line(a, b, Tolerance::DEFAULT), LineLineResult::None);
}

#[test]
fn degenerate_segment_yields_none() {
    let a = Line::new(Vec2::ZERO, Vec2::new(1e-15, 0.0));
    let b = Line::new(Vec2::Y, Vec2::new(1.0, 1.0));
    assert_eq!(line_line(a, b, Tolerance::DEFAULT), LineLineResult::None);
}

#[test]
fn tangent_line_circle() {
    let line = Line::new(Vec2::new(-2.0, 1.0), Vec2::new(2.0, 1.0));
    let circle = Circle::new(Vec2::ZERO, 1.0);
    match line_circle(line, circle, Tolerance::DEFAULT) {
        LineCircleResult::One(p) => {
            assert!(p.nearly_eq(Vec2::new(0.0, 1.0), Tolerance::COARSE));
        }
        other => panic!("expected tangent, got {other:?}"),
    }
}

#[test]
fn segment_misses_circle() {
    let line = Line::new(Vec2::new(2.0, 2.0), Vec2::new(3.0, 3.0));
    let circle = Circle::new(Vec2::ZERO, 1.0);
    assert_eq!(
        line_circle(line, circle, Tolerance::DEFAULT),
        LineCircleResult::None
    );
}

#[test]
fn concentric_different_radius() {
    let a = Circle::new(Vec2::ZERO, 1.0);
    let b = Circle::new(Vec2::ZERO, 2.0);
    assert_eq!(
        circle_circle(a, b, Tolerance::DEFAULT),
        CircleCircleResult::None
    );
}

#[test]
fn concentric_same_radius_coincident() {
    let a = Circle::new(Vec2::new(1.0, 1.0), 3.0);
    let b = Circle::new(Vec2::new(1.0, 1.0), 3.0);
    assert_eq!(
        circle_circle(a, b, Tolerance::DEFAULT),
        CircleCircleResult::Coincident
    );
}

#[test]
fn externally_tangent_circles() {
    let a = Circle::new(Vec2::ZERO, 1.0);
    let b = Circle::new(Vec2::new(2.0, 0.0), 1.0);
    match circle_circle(a, b, Tolerance::DEFAULT) {
        CircleCircleResult::One(p) => {
            assert!(p.nearly_eq(Vec2::new(1.0, 0.0), Tolerance::COARSE));
        }
        other => panic!("expected one, got {other:?}"),
    }
}

#[test]
fn aabb_union_contains_both() {
    let a = Aabb::from_points(Vec2::ZERO, Vec2::new(1.0, 1.0));
    let b = Aabb::from_points(Vec2::new(2.0, 2.0), Vec2::new(3.0, 3.0));
    let u = a.union(b);
    assert!(u.contains_point(Vec2::ZERO, Tolerance::DEFAULT));
    assert!(u.contains_point(Vec2::new(3.0, 3.0), Tolerance::DEFAULT));
}

#[test]
fn translate_then_inverse() {
    let p = Vec2::new(3.0, -4.0);
    let d = Vec2::new(1.5, 2.5);
    let q = translate_point(p, d);
    let back = translate_point(q, -d);
    assert!(back.nearly_eq(p, Tolerance::DEFAULT));
}

#[test]
fn rotate_360_identity() {
    let p = Vec2::new(7.0, -3.0);
    let pivot = Vec2::new(1.0, 2.0);
    let q = rotate_point(p, pivot, std::f64::consts::TAU);
    assert!(q.nearly_eq(p, Tolerance::COARSE));
}

#[test]
fn scale_about_origin_doubles() {
    let p = Vec2::new(2.0, -3.0);
    let q = scale_point(p, Vec2::ZERO, 2.0);
    assert!(q.nearly_eq(Vec2::new(4.0, -6.0), Tolerance::DEFAULT));
}

#[test]
fn offset_line_preserves_length() {
    let line = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
    let off = offset_line(line, 2.0).expect("offset");
    assert!((off.length() - line.length()).abs() < 1e-9);
    assert!((off.start.y - 2.0).abs() < 1e-9);
}

proptest! {
    #[test]
    fn line_circle_points_on_circle(
        sx in -20f64..20.0, sy in -20f64..20.0,
        ex in -20f64..20.0, ey in -20f64..20.0,
        cx in -10f64..10.0, cy in -10f64..10.0,
        r in 0.5f64..8.0,
    ) {
        let line = Line::new(Vec2::new(sx, sy), Vec2::new(ex, ey));
        let circle = Circle::new(Vec2::new(cx, cy), r);
        let tol = Tolerance::COARSE;
        match line_circle(line, circle, tol) {
            LineCircleResult::One(p) => {
                prop_assert!((p.distance(circle.center) - circle.radius).abs() < 1e-4);
            }
            LineCircleResult::Two(p, q) => {
                prop_assert!((p.distance(circle.center) - circle.radius).abs() < 1e-4);
                prop_assert!((q.distance(circle.center) - circle.radius).abs() < 1e-4);
            }
            _ => {}
        }
    }

    #[test]
    fn aabb_contains_its_corners(
        x0 in -50f64..50.0, y0 in -50f64..50.0,
        x1 in -50f64..50.0, y1 in -50f64..50.0,
    ) {
        let bb = Aabb::from_points(Vec2::new(x0, y0), Vec2::new(x1, y1));
        prop_assert!(bb.contains_point(bb.min, Tolerance::DEFAULT));
        prop_assert!(bb.contains_point(bb.max, Tolerance::DEFAULT));
    }
}

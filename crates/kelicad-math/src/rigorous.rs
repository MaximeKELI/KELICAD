//! Rigorous property and edge-case tests for math primitives.

use proptest::prelude::*;

use crate::mat::{Mat3, Mat4};
use crate::predicates::{orient2d, Orient};
use crate::tolerance::Tolerance;
use crate::vec::{Vec2, Vec3};

#[test]
fn tolerance_nan_and_inf_never_eq() {
    let t = Tolerance::DEFAULT;
    assert!(!t.eq(f64::NAN, f64::NAN));
    assert!(!t.eq(f64::INFINITY, f64::INFINITY));
    assert!(!t.is_zero(f64::NAN));
}

#[test]
fn tolerance_cmp_trichotomy_near_boundary() {
    let t = Tolerance::new(1e-6);
    assert_eq!(t.cmp(1.0, 1.0 + 1e-7), 0);
    assert_eq!(t.cmp(1.0, 1.0 + 1e-5), -1);
    assert_eq!(t.cmp(1.0 + 1e-5, 1.0), 1);
}

#[test]
fn vec2_zero_normalize_is_none() {
    assert!(Vec2::ZERO.try_normalize(Tolerance::DEFAULT).is_none());
    assert!(Vec2::new(1e-15, 0.0)
        .try_normalize(Tolerance::DEFAULT)
        .is_none());
}

#[test]
fn vec3_cross_right_handed() {
    let c = Vec3::new(1.0, 0.0, 0.0).cross(Vec3::new(0.0, 1.0, 0.0));
    assert!(c.nearly_eq(Vec3::new(0.0, 0.0, 1.0), Tolerance::DEFAULT));
}

#[test]
fn vec3_cross_anticommutative() {
    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(-4.0, 0.5, 1.0);
    let ab = a.cross(b);
    let ba = b.cross(a);
    assert!((ab + ba).nearly_eq(Vec3::ZERO, Tolerance::DEFAULT));
}

#[test]
fn mat3_inverse_roundtrip_elements() {
    let m = Mat3::translation(3.0, -2.0).mul_mat(Mat3::rotation(0.3));
    let inv = m.inverse().expect("invertible");
    let id = m.mul_mat(inv);
    for (i, (&got, &expected)) in id.m.iter().zip(Mat3::IDENTITY.m.iter()).enumerate() {
        assert!(
            (got - expected).abs() < 1e-9,
            "m[{i}] = {got} vs {expected}"
        );
    }
}

#[test]
fn mat3_singular_has_no_inverse() {
    let m = Mat3 {
        m: [1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    };
    assert!(m.inverse().is_none());
}

#[test]
fn mat3_transform_vector_ignores_translation() {
    let m = Mat3::translation(100.0, -50.0).mul_mat(Mat3::rotation(std::f64::consts::FRAC_PI_2));
    let v = m.transform_vector(Vec2::X);
    assert!(v.nearly_eq(Vec2::Y, Tolerance::DEFAULT));
}

#[test]
fn mat4_translation_transforms_point() {
    let m = Mat4::translation(Vec3::new(1.0, 2.0, 3.0));
    let p = m.transform_point(Vec3::new(4.0, 5.0, 6.0));
    assert!(p.nearly_eq(Vec3::new(5.0, 7.0, 9.0), Tolerance::DEFAULT));
}

#[test]
fn orient2d_ccw_cw_collinear() {
    let o = Vec2::ZERO;
    let a = Vec2::new(1.0, 0.0);
    let b = Vec2::new(0.0, 1.0);
    assert_eq!(orient2d(o, a, b, Tolerance::DEFAULT), Orient::CounterClockwise);
    assert_eq!(orient2d(o, b, a, Tolerance::DEFAULT), Orient::Clockwise);
    assert_eq!(
        orient2d(o, a, Vec2::new(2.0, 0.0), Tolerance::DEFAULT),
        Orient::Collinear
    );
}

#[test]
fn lerp_midpoint() {
    let a = Vec2::new(-2.0, 4.0);
    let b = Vec2::new(4.0, -2.0);
    let m = a.lerp(b, 0.5);
    assert!(m.nearly_eq(Vec2::new(1.0, 1.0), Tolerance::DEFAULT));
}

proptest! {
    #[test]
    fn vec2_length_nonnegative(x in -1e4f64..1e4, y in -1e4f64..1e4) {
        prop_assume!(x.is_finite() && y.is_finite());
        let v = Vec2::new(x, y);
        prop_assert!(v.length() >= 0.0);
        prop_assert!(
            (v.length_squared() - v.length() * v.length()).abs()
                < 1e-6 * (1.0 + v.length_squared())
        );
    }

    #[test]
    fn vec2_add_commutative(
        x0 in -1e3f64..1e3, y0 in -1e3f64..1e3,
        x1 in -1e3f64..1e3, y1 in -1e3f64..1e3
    ) {
        let a = Vec2::new(x0, y0);
        let b = Vec2::new(x1, y1);
        prop_assert!((a + b).nearly_eq(b + a, Tolerance::DEFAULT));
    }

    #[test]
    fn mat3_identity_neutral(x in -1e3f64..1e3, y in -1e3f64..1e3) {
        let p = Vec2::new(x, y);
        prop_assert!(Mat3::IDENTITY.transform_point(p).nearly_eq(p, Tolerance::DEFAULT));
    }

    #[test]
    fn rotate_preserves_length(
        x in -1e3f64..1e3, y in -1e3f64..1e3, ang in -std::f64::consts::TAU..std::f64::consts::TAU
    ) {
        let v = Vec2::new(x, y);
        let r = v.rotate(ang);
        prop_assert!((v.length() - r.length()).abs() < 1e-8 * (1.0 + v.length()));
    }

    #[test]
    fn mat3_scale_multiplies_length(
        x in -100f64..100.0, y in -100f64..100.0, s in 0.1f64..10.0
    ) {
        let p = Vec2::new(x, y);
        let q = Mat3::scale(s).transform_point(p);
        prop_assert!((q.length() - p.length() * s).abs() < 1e-8 * (1.0 + q.length()));
    }

    #[test]
    fn vec3_dot_symmetric(
        ax in -50f64..50.0, ay in -50f64..50.0, az in -50f64..50.0,
        bx in -50f64..50.0, by in -50f64..50.0, bz in -50f64..50.0,
    ) {
        let a = Vec3::new(ax, ay, az);
        let b = Vec3::new(bx, by, bz);
        prop_assert!((a.dot(b) - b.dot(a)).abs() < 1e-9);
    }
}

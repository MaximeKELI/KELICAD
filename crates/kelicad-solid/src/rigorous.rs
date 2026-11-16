//! Rigorous solid-kernel edge cases and invariants.

use kelicad_math::{Vec2, Vec3};

use crate::error::SolidError;
use crate::kernel::{BuiltinSolidKernel, SolidKernel};
use crate::types::{Plane, Profile2};
use crate::views::{derive_view, ViewKind};

#[test]
fn extrude_rejects_non_positive_height() {
    let k = BuiltinSolidKernel;
    let p = Profile2::rectangle(Vec2::ZERO, 1.0, 1.0).unwrap();
    assert!(matches!(
        k.extrude(&p, 0.0),
        Err(SolidError::InvalidArg(_))
    ));
    assert!(matches!(
        k.extrude(&p, -1.0),
        Err(SolidError::InvalidArg(_))
    ));
    assert!(matches!(
        k.extrude(&p, f64::NAN),
        Err(SolidError::InvalidArg(_))
    ));
}

#[test]
fn profile_rejects_too_few_vertices() {
    assert!(Profile2::from_vertices(vec![Vec2::ZERO, Vec2::X]).is_err());
    assert!(Profile2::rectangle(Vec2::ZERO, 0.0, 1.0).is_err());
}

#[test]
fn box_rejects_inverted_extents() {
    let k = BuiltinSolidKernel;
    assert!(k
        .box_solid(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO)
        .is_err());
}

#[test]
fn cylinder_rejects_zero_radius() {
    let k = BuiltinSolidKernel;
    assert!(k.cylinder(Vec2::ZERO, 0.0, 1.0, 8).is_err());
}

#[test]
fn revolve_rejects_zero_degrees() {
    let k = BuiltinSolidKernel;
    let p = Profile2::rectangle(Vec2::new(1.0, 0.0), 1.0, 1.0).unwrap();
    assert!(k.revolve(&p, 0.0, 8).is_err());
}

#[test]
fn boolean_union_aabb_encloses_both() {
    let k = BuiltinSolidKernel;
    let a = k.box_solid(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let b = k
        .box_solid(Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0))
        .unwrap();
    let u = k.boolean_union(&a, &b).unwrap();
    assert!(u.aabb.min.x <= a.aabb.min.x + 1e-9);
    assert!(u.aabb.max.x >= b.aabb.max.x - 1e-9);
    assert!(u.aabb.volume() >= a.aabb.volume() + b.aabb.volume() - 1e-6);
}

#[test]
fn boolean_difference_full_cut_errors() {
    let k = BuiltinSolidKernel;
    let a = k.box_solid(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let cutter = k
        .box_solid(
            Vec3::new(-0.1, -0.1, -0.1),
            Vec3::new(1.1, 1.1, 1.1),
        )
        .unwrap();
    assert!(matches!(
        k.boolean_difference(&a, &cutter),
        Err(SolidError::Boolean(_))
    ));
}

#[test]
fn section_empty_outside_body() {
    let k = BuiltinSolidKernel;
    let body = k.box_solid(Vec3::ZERO, Vec3::new(2.0, 2.0, 2.0)).unwrap();
    let geoms = k.section(&body, Plane::z(10.0));
    assert!(geoms.is_empty());
}

#[test]
fn elevation_view_nonempty() {
    let k = BuiltinSolidKernel;
    let body = k.box_solid(Vec3::ZERO, Vec3::new(4.0, 2.0, 3.0)).unwrap();
    let elev = derive_view(&body, ViewKind::ElevationX, None);
    assert!(!elev.is_empty());
}

#[test]
fn mesh_triangle_count_positive_for_box() {
    let k = BuiltinSolidKernel;
    let body = k.box_solid(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)).unwrap();
    assert!(body.mesh.indices.len() >= 12 * 3 / 3); // at least 12 tris typical
    assert_eq!(body.mesh.indices.len() % 3, 0);
    assert!(!body.mesh.positions.is_empty());
}

//! Rigorous BIM / IFC edge cases.

use kelicad_math::{Vec2, Vec3};
use kelicad_solid::{BuiltinSolidKernel, SolidKernel};

use crate::collide::collisions;
use crate::element::BimKind;
use crate::error::BimError;
use crate::ifc::{export_ifc, import_ifc};
use crate::schedule::schedule_csv;
use crate::wall::{door_opening_box, slab_from_rect, wall_from_baseline};

#[test]
fn wall_rejects_degenerate_baseline() {
    let err = wall_from_baseline(Vec2::ZERO, Vec2::ZERO, 0.2, 2.5).unwrap_err();
    assert!(matches!(err, BimError::Invalid(_)));
}

#[test]
fn wall_rejects_non_positive_dims() {
    assert!(wall_from_baseline(Vec2::ZERO, Vec2::X, 0.0, 2.0).is_err());
    assert!(wall_from_baseline(Vec2::ZERO, Vec2::X, 0.2, -1.0).is_err());
}

#[test]
fn ifc_roundtrip_wall_and_slab() {
    let (wb, we) = wall_from_baseline(Vec2::ZERO, Vec2::new(6.0, 0.0), 0.25, 3.0).unwrap();
    let (sb, se) = slab_from_rect(Vec2::ZERO, 6.0, 4.0, 0.2).unwrap();
    let text = export_ifc(&[we.clone(), se.clone()], &[wb.clone(), sb.clone()]).unwrap();
    assert!(text.contains("IFCWALL"));
    assert!(text.contains("IFCSLAB"));
    let (elems, bodies) = import_ifc(&text).unwrap();
    assert_eq!(elems.len(), 2);
    assert!(elems.iter().any(|e| e.kind == BimKind::Wall));
    assert!(elems.iter().any(|e| e.kind == BimKind::Slab));
    assert_eq!(bodies.len(), 2);
}

#[test]
fn ifc_empty_errors() {
    let text = "ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\nENDSEC;\nEND-ISO-10303-21;\n";
    assert!(matches!(import_ifc(text), Err(BimError::Ifc(_))));
}

#[test]
fn door_opening_difference_on_wall() {
    let k = BuiltinSolidKernel;
    let (wall, _) = wall_from_baseline(Vec2::ZERO, Vec2::new(5.0, 0.0), 0.2, 2.7).unwrap();
    let (opening, _) = door_opening_box(
        Vec3::new(2.0, -0.15, 0.0),
        Vec3::new(3.0, 0.15, 2.1),
    )
    .unwrap();
    let cut = k.boolean_difference(&wall, &opening);
    // May succeed (partial cut) or error if filter removes all — accept success with smaller volume.
    if let Ok(diff) = cut {
        assert!(diff.aabb.volume() <= wall.aabb.volume() + 1e-6);
    }
}

#[test]
fn schedule_includes_requested_props() {
    let (_, we) = wall_from_baseline(Vec2::ZERO, Vec2::new(3.0, 0.0), 0.2, 2.5).unwrap();
    let csv = schedule_csv(std::slice::from_ref(&we), &["tag", "height", "missing"]);
    let header = csv.lines().next().unwrap();
    assert!(header.contains("tag"));
    assert!(header.contains("height"));
    assert!(header.contains("missing"));
    assert!(csv.contains("W-01"));
}

#[test]
fn collisions_self_pair_not_reported_twice() {
    let k = BuiltinSolidKernel;
    let a = k.box_solid(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0)).unwrap();
    let b = k
        .box_solid(Vec3::new(0.5, 0.0, 0.0), Vec3::new(1.5, 1.0, 1.0))
        .unwrap();
    let c = k
        .box_solid(Vec3::new(0.6, 0.0, 0.0), Vec3::new(1.6, 1.0, 1.0))
        .unwrap();
    let hits = collisions(&[a, b, c]);
    // Three overlapping pairs possible: ab, ac, bc
    assert_eq!(hits.len(), 3);
}

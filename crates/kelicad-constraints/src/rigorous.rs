//! Rigorous constraint solver cases.

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, LayerId};
use kelicad_geom::Line;
use kelicad_math::{Tolerance, Vec2};
use kelicad_ops::{DocSession, Operation};
use uuid::Uuid;

use crate::solver::{solve, SolveStatus};
use crate::system::ConstraintSystem;
use crate::types::{Constraint, Endpoint};

fn eid(n: u128) -> EntityId {
    EntityId::from_uuid(Uuid::from_u128(n))
}

fn line_entity(id: EntityId, layer: LayerId, start: Vec2, end: Vec2) -> EntityRecord {
    EntityRecord {
        id,
        kind: EntityKind::Line,
        geometry: Geometry::Line(Line::new(start, end)),
        layer_id: layer,
    }
}

#[test]
fn empty_system_underconstrained() {
    let session = DocSession::new();
    let sys = ConstraintSystem::new();
    let r = solve(&sys, &session.document).unwrap();
    assert_eq!(r.status, SolveStatus::Underconstrained);
    assert!(r.ops.is_empty());
}

#[test]
fn perpendicular_pair() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let a = eid(1);
    let b = eid(2);
    session
        .apply(Operation::CreateEntity {
            record: line_entity(a, layer, Vec2::ZERO, Vec2::new(4.0, 0.0)),
        })
        .unwrap();
    session
        .apply(Operation::CreateEntity {
            record: line_entity(b, layer, Vec2::ZERO, Vec2::new(3.0, 1.0)),
        })
        .unwrap();
    let mut sys = ConstraintSystem::new();
    sys.add(Constraint::Perpendicular { a, b }).unwrap();
    let result = solve(&sys, &session.document).unwrap();
    assert!(matches!(
        result.status,
        SolveStatus::Solved | SolveStatus::Underconstrained
    ));
    session.apply(Operation::Batch { ops: result.ops }).unwrap();
    let Geometry::Line(la) = &session.document.get_entity(a).unwrap().geometry else {
        panic!();
    };
    let Geometry::Line(lb) = &session.document.get_entity(b).unwrap().geometry else {
        panic!();
    };
    assert!(la.delta().dot(lb.delta()).abs() < 1e-4);
}

#[test]
fn equal_length_pair() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let a = eid(10);
    let b = eid(11);
    session
        .apply(Operation::CreateEntity {
            record: line_entity(a, layer, Vec2::ZERO, Vec2::new(4.0, 0.0)),
        })
        .unwrap();
    session
        .apply(Operation::CreateEntity {
            record: line_entity(b, layer, Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0)),
        })
        .unwrap();
    let mut sys = ConstraintSystem::new();
    sys.add(Constraint::EqualLength { a, b }).unwrap();
    let result = solve(&sys, &session.document).unwrap();
    session.apply(Operation::Batch { ops: result.ops }).unwrap();
    let Geometry::Line(la) = &session.document.get_entity(a).unwrap().geometry else {
        panic!();
    };
    let Geometry::Line(lb) = &session.document.get_entity(b).unwrap().geometry else {
        panic!();
    };
    assert!((la.length() - lb.length()).abs() < 1e-4);
}

#[test]
fn vertical_and_distance() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let id = eid(20);
    session
        .apply(Operation::CreateEntity {
            record: line_entity(id, layer, Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0)),
        })
        .unwrap();
    let mut sys = ConstraintSystem::new();
    sys.add(Constraint::Vertical { entity: id }).unwrap();
    sys.add(Constraint::Distance {
        entity: id,
        value: 6.0,
    })
    .unwrap();
    let result = solve(&sys, &session.document).unwrap();
    session
        .apply(Operation::Batch {
            ops: result.ops.clone(),
        })
        .unwrap();
    let Geometry::Line(l) = &session.document.get_entity(id).unwrap().geometry else {
        panic!();
    };
    assert!((l.end.x - l.start.x).abs() < 1e-5);
    assert!((l.length() - 6.0).abs() < 1e-4);
}

#[test]
fn missing_entity_errors() {
    let session = DocSession::new();
    let mut sys = ConstraintSystem::new();
    sys.add(Constraint::Horizontal {
        entity: eid(999),
    })
    .unwrap();
    let err = solve(&sys, &session.document).unwrap_err();
    assert!(matches!(err, crate::error::ConstraintError::NotALine(_)));
}

#[test]
fn coincident_preserves_other_endpoint_approximately() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let a = eid(30);
    let b = eid(31);
    session
        .apply(Operation::CreateEntity {
            record: line_entity(a, layer, Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0)),
        })
        .unwrap();
    session
        .apply(Operation::CreateEntity {
            record: line_entity(b, layer, Vec2::new(2.2, 0.1), Vec2::new(5.0, 0.1)),
        })
        .unwrap();
    let mut sys = ConstraintSystem::new();
    sys.add(Constraint::Coincident {
        a,
        a_end: Endpoint::End,
        b,
        b_end: Endpoint::Start,
    })
    .unwrap();
    let result = solve(&sys, &session.document).unwrap();
    session.apply(Operation::Batch { ops: result.ops }).unwrap();
    let Geometry::Line(la) = &session.document.get_entity(a).unwrap().geometry else {
        panic!();
    };
    let Geometry::Line(lb) = &session.document.get_entity(b).unwrap().geometry else {
        panic!();
    };
    assert!(la.end.nearly_eq(lb.start, Tolerance::COARSE));
    // Free ends should not jump wildly.
    assert!(la.start.distance(Vec2::ZERO) < 0.5);
    assert!(lb.end.distance(Vec2::new(5.0, 0.1)) < 0.5);
}

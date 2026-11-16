//! Rigorous undo / batch / remote apply invariants.

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, Layer, LayerId};
use kelicad_geom::Line;
use kelicad_math::Vec2;
use uuid::Uuid;

use crate::operation::{OpError, Operation};
use crate::session::DocSession;

fn line_record(id: u128, layer: LayerId, start: Vec2, end: Vec2) -> EntityRecord {
    EntityRecord {
        id: EntityId::from_uuid(Uuid::from_u128(id)),
        kind: EntityKind::Line,
        geometry: Geometry::Line(Line::new(start, end)),
        layer_id: layer,
    }
}

#[test]
fn batch_undo_restores_all() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let a = line_record(100, layer, Vec2::ZERO, Vec2::new(1.0, 0.0));
    let b = line_record(101, layer, Vec2::ZERO, Vec2::new(0.0, 1.0));
    session
        .apply(Operation::Batch {
            ops: vec![
                Operation::CreateEntity { record: a },
                Operation::CreateEntity { record: b },
            ],
        })
        .unwrap();
    assert_eq!(session.document.entity_count(), 2);
    assert!(session.undo().unwrap());
    assert_eq!(session.document.entity_count(), 0);
    assert!(session.redo().unwrap());
    assert_eq!(session.document.entity_count(), 2);
}

#[test]
fn duplicate_create_fails_without_mutating() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let record = line_record(200, layer, Vec2::ZERO, Vec2::X);
    session
        .apply(Operation::CreateEntity {
            record: record.clone(),
        })
        .unwrap();
    let err = session
        .apply(Operation::CreateEntity { record })
        .unwrap_err();
    assert!(matches!(err, OpError::EntityExists(_)));
    assert_eq!(session.document.entity_count(), 1);
    assert!(session.journal.can_undo());
    // Failed apply must not push a journal entry beyond the first create.
    session.undo().unwrap();
    assert_eq!(session.document.entity_count(), 0);
    assert!(!session.journal.can_undo());
}

#[test]
fn delete_then_undo_restores_geometry() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let record = line_record(300, layer, Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0));
    let id = record.id;
    session.apply(Operation::CreateEntity { record }).unwrap();
    session.apply(Operation::DeleteEntity { id }).unwrap();
    assert!(session.document.get_entity(id).is_none());
    session.undo().unwrap();
    let restored = session.document.get_entity(id).unwrap();
    match &restored.geometry {
        Geometry::Line(l) => {
            assert_eq!(l.start, Vec2::new(1.0, 2.0));
            assert_eq!(l.end, Vec2::new(3.0, 4.0));
        }
        _ => panic!("expected line"),
    }
}

#[test]
fn apply_remote_create_is_upsert() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    let id = EntityId::from_uuid(Uuid::from_u128(400));
    let first = line_record(400, layer, Vec2::ZERO, Vec2::X);
    let second = EntityRecord {
        id,
        kind: EntityKind::Line,
        geometry: Geometry::Line(Line::new(Vec2::Y, Vec2::new(1.0, 1.0))),
        layer_id: layer,
    };
    session
        .apply_remote(&Operation::CreateEntity { record: first })
        .unwrap();
    session
        .apply_remote(&Operation::CreateEntity { record: second })
        .unwrap();
    assert!(!session.journal.can_undo());
    assert_eq!(session.document.entity_count(), 1);
    match &session.document.get_entity(id).unwrap().geometry {
        Geometry::Line(l) => assert_eq!(l.start, Vec2::Y),
        _ => panic!("expected line"),
    }
}

#[test]
fn apply_remote_delete_missing_is_ok() {
    let mut session = DocSession::new();
    let missing = EntityId::from_uuid(Uuid::from_u128(999));
    session
        .apply_remote(&Operation::DeleteEntity { id: missing })
        .unwrap();
    assert_eq!(session.document.entity_count(), 0);
}

#[test]
fn set_entity_layer_undo() {
    let mut session = DocSession::new();
    let default = session.document.default_layer_id();
    let walls = LayerId::from_uuid(Uuid::from_u128(55));
    session
        .apply(Operation::CreateLayer {
            layer: Layer::new(walls, "Walls", [255, 0, 0]),
        })
        .unwrap();
    let record = line_record(500, default, Vec2::ZERO, Vec2::X);
    let id = record.id;
    session.apply(Operation::CreateEntity { record }).unwrap();
    session
        .apply(Operation::SetEntityLayer {
            id,
            layer_id: walls,
        })
        .unwrap();
    assert_eq!(session.document.get_entity(id).unwrap().layer_id, walls);
    session.undo().unwrap();
    assert_eq!(session.document.get_entity(id).unwrap().layer_id, default);
}

#[test]
fn nested_batch_undo_is_atomic() {
    let mut session = DocSession::new();
    let layer = session.document.default_layer_id();
    session
        .apply(Operation::Batch {
            ops: vec![
                Operation::CreateEntity {
                    record: line_record(600, layer, Vec2::ZERO, Vec2::X),
                },
                Operation::Batch {
                    ops: vec![Operation::CreateEntity {
                        record: line_record(601, layer, Vec2::ZERO, Vec2::Y),
                    }],
                },
            ],
        })
        .unwrap();
    assert_eq!(session.document.entity_count(), 2);
    session.undo().unwrap();
    assert_eq!(session.document.entity_count(), 0);
}

#[test]
fn delete_default_layer_fails() {
    let mut session = DocSession::new();
    let default = session.document.default_layer_id();
    let err = session
        .apply(Operation::DeleteLayer { id: default })
        .unwrap_err();
    assert!(matches!(err, OpError::Doc(_)));
}

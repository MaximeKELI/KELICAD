//! Rigorous LWW / tombstone / fingerprint invariants.

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, LayerId};
use kelicad_geom::Line;
use kelicad_math::Vec2;
use kelicad_ops::Operation;
use uuid::Uuid;

use crate::envelope::{ActorId, HybridTime};
use crate::lww::{targets_of, LwwTable, TargetKey};
use crate::replica::Replica;
use crate::version::{fingerprint, snapshot_session};

fn line_op(id: u128) -> Operation {
    Operation::CreateEntity {
        record: EntityRecord {
            id: EntityId::from_uuid(Uuid::from_u128(id)),
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::X)),
            layer_id: LayerId::from_uuid(Uuid::nil()),
            props: Default::default(),
        },
    }
}

#[test]
fn targets_of_batch_flattens() {
    let op = Operation::Batch {
        ops: vec![
            line_op(1),
            Operation::DeleteEntity {
                id: EntityId::from_uuid(Uuid::from_u128(2)),
            },
        ],
    };
    let keys = targets_of(&op);
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&TargetKey::Entity(EntityId::from_uuid(
        Uuid::from_u128(1)
    ))));
}

#[test]
fn older_write_does_not_apply() {
    let mut table = LwwTable::new();
    let id = EntityId::from_uuid(Uuid::from_u128(1));
    let key = TargetKey::Entity(id);
    let t1 = HybridTime {
        lamport: 5,
        actor: ActorId::from_u128(1),
    };
    let t0 = HybridTime {
        lamport: 3,
        actor: ActorId::from_u128(2),
    };
    table.record(&line_op(1), t1);
    assert!(!table.would_apply(&key, t0));
    assert!(table.would_apply(
        &key,
        HybridTime {
            lamport: 6,
            actor: ActorId::from_u128(1),
        }
    ));
}

#[test]
fn tombstone_blocks_stale_create() {
    let mut table = LwwTable::new();
    let id = EntityId::from_uuid(Uuid::from_u128(9));
    let del = Operation::DeleteEntity { id };
    let t_del = HybridTime {
        lamport: 10,
        actor: ActorId::from_u128(1),
    };
    table.record(&del, t_del);
    assert!(table.is_tombstoned(
        id,
        HybridTime {
            lamport: 5,
            actor: ActorId::from_u128(2),
        }
    ));
    assert!(!table.is_tombstoned(
        id,
        HybridTime {
            lamport: 11,
            actor: ActorId::from_u128(2),
        }
    ));
}

#[test]
fn recreate_clears_tombstone() {
    let mut table = LwwTable::new();
    let id = EntityId::from_uuid(Uuid::from_u128(9));
    table.record(
        &Operation::DeleteEntity { id },
        HybridTime {
            lamport: 1,
            actor: ActorId::from_u128(1),
        },
    );
    table.record(
        &line_op(9),
        HybridTime {
            lamport: 2,
            actor: ActorId::from_u128(1),
        },
    );
    assert!(!table.is_tombstoned(
        id,
        HybridTime {
            lamport: 1,
            actor: ActorId::from_u128(1),
        }
    ));
}

#[test]
fn fingerprint_stable_for_same_doc() {
    let mut a = Replica::new(ActorId::from_u128(1), "A", [1, 2, 3]);
    a.commit_local(line_op(1)).unwrap();
    let fp1 = a.doc_fingerprint();
    let snap = snapshot_session(&a.session).unwrap();
    assert!(!snap.is_empty());
    assert_eq!(fp1, a.doc_fingerprint());
    let ids = vec!["op-a".into(), "op-b".into()];
    assert_eq!(fingerprint(&ids), fingerprint(&["op-b".into(), "op-a".into()]));
}

#[test]
fn equal_lamport_actor_tiebreak_deterministic() {
    let mut table = LwwTable::new();
    let id = EntityId::from_uuid(Uuid::from_u128(1));
    let key = TargetKey::Entity(id);
    let t_low = HybridTime {
        lamport: 7,
        actor: ActorId::from_u128(1),
    };
    let t_high = HybridTime {
        lamport: 7,
        actor: ActorId::from_u128(2),
    };
    table.record(&line_op(1), t_low);
    // Higher actor wins at equal lamport (HybridTime Ord); equal time still applies.
    assert!(table.would_apply(&key, t_high));
    assert!(table.would_apply(&key, t_low));
    assert!(!table.would_apply(
        &key,
        HybridTime {
            lamport: 6,
            actor: ActorId::from_u128(99),
        }
    ));
}

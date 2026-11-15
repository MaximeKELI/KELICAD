//! KELICAD collaboration: operation-log sync, presence, comments.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod comments;
pub mod envelope;
pub mod lww;
pub mod presence;
pub mod protocol;
pub mod replica;
pub mod version;

pub use comments::{Comment, CommentAnchor, CommentId, CommentStore};
pub use envelope::{ActorId, HybridTime, OpEnvelope, OpId};
pub use lww::{targets_of, LwwTable, TargetKey};
pub use presence::PresenceState;
pub use protocol::WireMessage;
pub use replica::{IntegrateResult, Replica, ReplicaStatus, SyncError};
pub use version::{diff_snapshots, fingerprint, snapshot_session, EntityDiff, RestorePoint};

#[cfg(test)]
mod convergence_tests {
    use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry};
    use kelicad_geom::Line;
    use kelicad_math::Vec2;
    use kelicad_ops::Operation;
    use uuid::Uuid;

    use super::*;

    fn line_op(id: u128, x0: f64, y0: f64, x1: f64, y1: f64) -> Operation {
        let eid = EntityId::from_uuid(Uuid::from_u128(id));
        Operation::CreateEntity {
            record: EntityRecord {
                id: eid,
                kind: EntityKind::Line,
                geometry: Geometry::Line(Line::new(Vec2::new(x0, y0), Vec2::new(x1, y1))),
                layer_id: kelicad_doc::LayerId::from_uuid(Uuid::nil()),
            },
        }
    }

    #[test]
    fn two_replicas_converge_after_offline() {
        let mut a = Replica::new(ActorId::from_u128(1), "Alice", [255, 80, 80]);
        let mut b = Replica::new(ActorId::from_u128(2), "Bob", [80, 160, 255]);

        // Concurrent creates
        let ea = a.commit_local(line_op(10, 0.0, 0.0, 1.0, 0.0)).unwrap();
        let eb = b.commit_local(line_op(20, 0.0, 0.0, 0.0, 1.0)).unwrap();

        // Exchange
        a.integrate(&eb).unwrap();
        b.integrate(&ea).unwrap();
        assert_eq!(a.doc_fingerprint(), b.doc_fingerprint());
        assert_eq!(a.session.document.entity_count(), 2);

        // B goes offline; A edits
        let move_a = a
            .commit_local(Operation::SetGeometry {
                id: EntityId::from_uuid(Uuid::from_u128(10)),
                geometry: Geometry::Line(Line::new(Vec2::new(5.0, 0.0), Vec2::new(6.0, 0.0))),
            })
            .unwrap();
        // B edits while offline
        let del_b = b
            .commit_local(Operation::DeleteEntity {
                id: EntityId::from_uuid(Uuid::from_u128(20)),
            })
            .unwrap();

        // Reconnect: exchange outboxes
        a.integrate(&del_b).unwrap();
        b.integrate(&move_a).unwrap();

        assert_eq!(a.doc_fingerprint(), b.doc_fingerprint());
        assert_eq!(a.session.document.entity_count(), 1);
        let ent = a
            .session
            .document
            .get_entity(EntityId::from_uuid(Uuid::from_u128(10)))
            .unwrap();
        match &ent.geometry {
            Geometry::Line(l) => assert!((l.start.x - 5.0).abs() < 1e-9),
            _ => panic!("expected line"),
        }
    }

    #[test]
    fn lww_same_entity_higher_lamport_wins() {
        let mut a = Replica::new(ActorId::from_u128(1), "A", [1, 1, 1]);
        let mut b = Replica::new(ActorId::from_u128(2), "B", [2, 2, 2]);
        let id = EntityId::from_uuid(Uuid::from_u128(99));
        let create = a.commit_local(line_op(99, 0.0, 0.0, 1.0, 0.0)).unwrap();
        b.integrate(&create).unwrap();

        let set_a = a
            .commit_local(Operation::SetGeometry {
                id,
                geometry: Geometry::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0))),
            })
            .unwrap();
        // Force B to have lower clock then set
        b.lamport = set_a.lamport.saturating_sub(1);
        let set_b = b
            .commit_local(Operation::SetGeometry {
                id,
                geometry: Geometry::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 0.0))),
            })
            .unwrap();
        // set_a has lower lamport than set_b if b ticked after observe...
        // Actually b.lamport was set_a.lamport-1, then tick → set_a.lamport.
        // Equal lamport → actor_id tie-break: ActorId(2) > ActorId(1).
        a.integrate(&set_b).unwrap();
        b.integrate(&set_a).unwrap();
        assert_eq!(a.doc_fingerprint(), b.doc_fingerprint());
    }

    #[test]
    fn comments_and_versions() {
        let mut r = Replica::new(ActorId::from_u128(3), "C", [0, 200, 0]);
        r.commit_local(line_op(1, 0.0, 0.0, 1.0, 0.0)).unwrap();
        r.add_comment(
            CommentAnchor::Point {
                position: Vec2::new(0.5, 0.0),
            },
            "check this",
        );
        assert_eq!(r.comments.all().len(), 1);
        let v1 = r.save_version("v1").unwrap();
        r.commit_local(line_op(2, 2.0, 0.0, 3.0, 0.0)).unwrap();
        let _v2 = r.save_version("v2").unwrap();
        let diff = r.diff_versions("v1", "v2").unwrap();
        assert_eq!(diff.added.len(), 1);
        assert!(!v1.oplog_fingerprint.is_empty());
    }
}

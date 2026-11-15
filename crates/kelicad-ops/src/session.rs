//! Document + journal session (apply / undo / redo).

use kelicad_doc::Document;

use crate::journal::OpJournal;
use crate::operation::{OpError, Operation};

/// Editable document session. All mutations go through [`DocSession::apply`].
#[derive(Debug, Clone)]
pub struct DocSession {
    /// Current document state.
    pub document: Document,
    /// Operation journal.
    pub journal: OpJournal,
}

impl DocSession {
    /// New empty session.
    pub fn new() -> Self {
        Self {
            document: Document::new(),
            journal: OpJournal::new(),
        }
    }

    /// Apply an operation, recording undo information.
    pub fn apply(&mut self, op: Operation) -> Result<(), OpError> {
        let inverse = op.apply(&mut self.document)?;
        self.journal.commit(op, inverse);
        Ok(())
    }

    /// Undo the last operation.
    pub fn undo(&mut self) -> Result<bool, OpError> {
        let Some(inverse) = self.journal.undo_inverse() else {
            return Ok(false);
        };
        // Apply inverse without journaling (cursor already moved).
        let _ = inverse.apply(&mut self.document)?;
        Ok(true)
    }

    /// Redo the next undone operation.
    pub fn redo(&mut self) -> Result<bool, OpError> {
        let Some(forward) = self.journal.redo_forward() else {
            return Ok(false);
        };
        let _ = forward.apply(&mut self.document)?;
        Ok(true)
    }
}

impl Default for DocSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, Layer, LayerId};
    use kelicad_geom::Line;
    use kelicad_math::Vec2;
    use uuid::Uuid;

    fn line_record(id: u128, layer: LayerId) -> EntityRecord {
        EntityRecord {
            id: EntityId::from_uuid(Uuid::from_u128(id)),
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0))),
            layer_id: layer,
        }
    }

    #[test]
    fn create_undo_redo() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let record = line_record(1, layer);
        let id = record.id;

        session
            .apply(Operation::CreateEntity { record })
            .expect("create");
        assert_eq!(session.document.entity_count(), 1);

        assert!(session.undo().unwrap());
        assert_eq!(session.document.entity_count(), 0);

        assert!(session.redo().unwrap());
        assert!(session.document.get_entity(id).is_some());
    }

    #[test]
    fn set_geometry_undo() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let record = line_record(2, layer);
        let id = record.id;
        session.apply(Operation::CreateEntity { record }).unwrap();

        let new_geom = Geometry::Line(Line::new(Vec2::new(1.0, 1.0), Vec2::new(2.0, 2.0)));
        session
            .apply(Operation::SetGeometry {
                id,
                geometry: new_geom.clone(),
            })
            .unwrap();
        assert_eq!(session.document.get_entity(id).unwrap().geometry, new_geom);

        session.undo().unwrap();
        match &session.document.get_entity(id).unwrap().geometry {
            Geometry::Line(l) => {
                assert_eq!(l.start, Vec2::ZERO);
                assert_eq!(l.end, Vec2::new(10.0, 0.0));
            }
            _ => panic!("expected line"),
        }
    }

    #[test]
    fn new_commit_truncates_redo() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        session
            .apply(Operation::CreateEntity {
                record: line_record(3, layer),
            })
            .unwrap();
        session.undo().unwrap();
        session
            .apply(Operation::CreateEntity {
                record: line_record(4, layer),
            })
            .unwrap();
        assert!(!session.journal.can_redo());
        assert_eq!(session.document.entity_count(), 1);
        assert!(session
            .document
            .get_entity(EntityId::from_uuid(Uuid::from_u128(4)))
            .is_some());
    }

    #[test]
    fn create_layer_and_assign() {
        let mut session = DocSession::new();
        let lid = LayerId::from_uuid(Uuid::from_u128(99));
        session
            .apply(Operation::CreateLayer {
                layer: Layer::new(lid, "Walls", [255, 0, 0]),
            })
            .unwrap();
        let record = line_record(5, lid);
        session.apply(Operation::CreateEntity { record }).unwrap();
        assert_eq!(
            session
                .document
                .get_entity(EntityId::from_uuid(Uuid::from_u128(5)))
                .unwrap()
                .layer_id,
            lid
        );
    }
}

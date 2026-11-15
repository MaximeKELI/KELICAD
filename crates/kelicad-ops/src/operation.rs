//! Atomic operations applied to a document.

use kelicad_doc::{DocError, Document, EntityId, EntityRecord, Geometry, Layer, LayerId};

/// Errors while applying an operation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    /// Document-level error.
    #[error(transparent)]
    Doc(#[from] DocError),
    /// Entity already exists.
    #[error("entity {0} already exists")]
    EntityExists(EntityId),
    /// Entity not found.
    #[error("entity {0} not found")]
    EntityNotFound(EntityId),
    /// Layer already exists.
    #[error("layer {0} already exists")]
    LayerExists(LayerId),
}

/// Atomic document mutation. Inverse is produced when applying for undo.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    /// Create an entity; fails if id already exists.
    CreateEntity {
        /// Entity to insert.
        record: EntityRecord,
    },
    /// Replace geometry (and kind) of an existing entity.
    SetGeometry {
        /// Target entity.
        id: EntityId,
        /// New geometry.
        geometry: Geometry,
    },
    /// Change layer assignment.
    SetEntityLayer {
        /// Target entity.
        id: EntityId,
        /// New layer.
        layer_id: LayerId,
    },
    /// Delete an entity.
    DeleteEntity {
        /// Target entity.
        id: EntityId,
    },
    /// Create a layer.
    CreateLayer {
        /// Layer to insert.
        layer: Layer,
    },
    /// Delete a layer (not default, not in use).
    DeleteLayer {
        /// Target layer.
        id: LayerId,
    },
    /// Set the current layer.
    SetCurrentLayer {
        /// Layer id.
        id: LayerId,
    },
}

impl Operation {
    /// Apply to `doc`, returning the inverse operation for undo.
    pub fn apply(&self, doc: &mut Document) -> Result<Operation, OpError> {
        match self {
            Self::CreateEntity { record } => {
                if doc.get_entity(record.id).is_some() {
                    return Err(OpError::EntityExists(record.id));
                }
                if doc.get_layer(record.layer_id).is_none() {
                    return Err(OpError::Doc(DocError::UnknownLayer(record.layer_id)));
                }
                let inverse = Operation::DeleteEntity { id: record.id };
                doc.upsert_entity(record.clone());
                Ok(inverse)
            }
            Self::SetGeometry { id, geometry } => {
                let Some(prev) = doc.get_entity(*id).cloned() else {
                    return Err(OpError::EntityNotFound(*id));
                };
                let inverse = Operation::SetGeometry {
                    id: *id,
                    geometry: prev.geometry.clone(),
                };
                let mut next = prev;
                next.kind = geometry.kind();
                next.geometry = geometry.clone();
                doc.upsert_entity(next);
                Ok(inverse)
            }
            Self::SetEntityLayer { id, layer_id } => {
                if doc.get_layer(*layer_id).is_none() {
                    return Err(OpError::Doc(DocError::UnknownLayer(*layer_id)));
                }
                let Some(prev) = doc.get_entity(*id).cloned() else {
                    return Err(OpError::EntityNotFound(*id));
                };
                let inverse = Operation::SetEntityLayer {
                    id: *id,
                    layer_id: prev.layer_id,
                };
                let mut next = prev;
                next.layer_id = *layer_id;
                doc.upsert_entity(next);
                Ok(inverse)
            }
            Self::DeleteEntity { id } => {
                let Some(prev) = doc.remove_entity(*id) else {
                    return Err(OpError::EntityNotFound(*id));
                };
                Ok(Operation::CreateEntity { record: prev })
            }
            Self::CreateLayer { layer } => {
                if doc.get_layer(layer.id).is_some() {
                    return Err(OpError::LayerExists(layer.id));
                }
                let inverse = Operation::DeleteLayer { id: layer.id };
                doc.upsert_layer(layer.clone());
                Ok(inverse)
            }
            Self::DeleteLayer { id } => {
                let removed = doc.remove_layer(*id)?;
                let Some(layer) = removed else {
                    return Err(OpError::Doc(DocError::UnknownLayer(*id)));
                };
                Ok(Operation::CreateLayer { layer })
            }
            Self::SetCurrentLayer { id } => {
                let prev = doc.current_layer;
                doc.set_current_layer(*id)?;
                Ok(Operation::SetCurrentLayer { id: prev })
            }
        }
    }
}

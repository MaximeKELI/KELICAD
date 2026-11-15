//! In-memory document (ECS-style sparse maps).

use std::collections::HashMap;

use uuid::Uuid;

use crate::blocks::{BlockDef, BlockId};
use crate::entity::{EntityId, EntityKind};
use crate::geometry::Geometry;
use crate::layer::{Layer, LayerId};
use crate::layout::{Layout, LayoutId};

/// Stored entity record.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityRecord {
    /// Id.
    pub id: EntityId,
    /// Kind (cached from geometry).
    pub kind: EntityKind,
    /// Geometry component.
    pub geometry: Geometry,
    /// Layer assignment.
    pub layer_id: LayerId,
}

/// Document state derived from the operation journal.
#[derive(Debug, Clone)]
pub struct Document {
    /// Schema / document format version for this in-memory model.
    pub version: u32,
    /// Entities by id.
    entities: HashMap<EntityId, EntityRecord>,
    /// Layers by id.
    layers: HashMap<LayerId, Layer>,
    /// Active layer for new entities.
    pub current_layer: LayerId,
    /// Name of the default layer created with every document.
    default_layer_id: LayerId,
    /// Block definitions by id.
    blocks: HashMap<BlockId, BlockDef>,
    /// Block name (uppercase) → id.
    block_names: HashMap<String, BlockId>,
    /// Paper-space layouts.
    layouts: HashMap<LayoutId, Layout>,
    /// Active layout (`None` = model space).
    pub current_layout: Option<LayoutId>,
}

impl Document {
    /// In-memory model version (independent of `.kelicad` file schema).
    pub const MODEL_VERSION: u32 = 0;

    /// Create an empty document with a default layer `"0"`.
    pub fn new() -> Self {
        let default_layer_id = LayerId::from_uuid(Uuid::nil());
        let layer = Layer::new(default_layer_id, "0", [255, 255, 255]);
        let mut layers = HashMap::new();
        layers.insert(default_layer_id, layer);
        Self {
            version: Self::MODEL_VERSION,
            entities: HashMap::new(),
            layers,
            current_layer: default_layer_id,
            default_layer_id,
            blocks: HashMap::new(),
            block_names: HashMap::new(),
            layouts: HashMap::new(),
            current_layout: None,
        }
    }

    /// Default layer id (`"0"`).
    pub fn default_layer_id(&self) -> LayerId {
        self.default_layer_id
    }

    /// Number of entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get an entity.
    pub fn get_entity(&self, id: EntityId) -> Option<&EntityRecord> {
        self.entities.get(&id)
    }

    /// Iterate entities.
    pub fn entities(&self) -> impl Iterator<Item = &EntityRecord> {
        self.entities.values()
    }

    /// Get a layer.
    pub fn get_layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.get(&id)
    }

    /// Iterate layers.
    pub fn layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers.values()
    }

    /// Insert or replace an entity (used by ops only).
    pub fn upsert_entity(&mut self, record: EntityRecord) {
        self.entities.insert(record.id, record);
    }

    /// Remove an entity; returns the previous record if any.
    pub fn remove_entity(&mut self, id: EntityId) -> Option<EntityRecord> {
        self.entities.remove(&id)
    }

    /// Insert or replace a layer (used by ops only).
    pub fn upsert_layer(&mut self, layer: Layer) {
        self.layers.insert(layer.id, layer);
    }

    /// Remove a layer if it is not the default layer and unused.
    pub fn remove_layer(&mut self, id: LayerId) -> Result<Option<Layer>, DocError> {
        if id == self.default_layer_id {
            return Err(DocError::CannotRemoveDefaultLayer);
        }
        if self.entities.values().any(|e| e.layer_id == id) {
            return Err(DocError::LayerInUse);
        }
        if self.current_layer == id {
            self.current_layer = self.default_layer_id;
        }
        Ok(self.layers.remove(&id))
    }

    /// Set current layer; errors if unknown.
    pub fn set_current_layer(&mut self, id: LayerId) -> Result<(), DocError> {
        if !self.layers.contains_key(&id) {
            return Err(DocError::UnknownLayer(id));
        }
        self.current_layer = id;
        Ok(())
    }

    /// Upsert a block definition.
    pub fn upsert_block(&mut self, def: BlockDef) {
        let key = def.name.to_ascii_uppercase();
        self.block_names.insert(key, def.id);
        self.blocks.insert(def.id, def);
    }

    /// Remove a block by id.
    pub fn remove_block(&mut self, id: BlockId) -> Option<BlockDef> {
        if let Some(def) = self.blocks.remove(&id) {
            self.block_names.remove(&def.name.to_ascii_uppercase());
            Some(def)
        } else {
            None
        }
    }

    /// Get block by id.
    pub fn get_block(&self, id: BlockId) -> Option<&BlockDef> {
        self.blocks.get(&id)
    }

    /// Lookup block by name (case-insensitive).
    pub fn block_by_name(&self, name: &str) -> Option<&BlockDef> {
        let id = self.block_names.get(&name.to_ascii_uppercase())?;
        self.blocks.get(id)
    }

    /// Iterate blocks.
    pub fn blocks(&self) -> impl Iterator<Item = &BlockDef> {
        self.blocks.values()
    }

    /// Upsert a layout.
    pub fn upsert_layout(&mut self, layout: Layout) {
        self.layouts.insert(layout.id, layout);
    }

    /// Remove layout.
    pub fn remove_layout(&mut self, id: LayoutId) -> Option<Layout> {
        if self.current_layout == Some(id) {
            self.current_layout = None;
        }
        self.layouts.remove(&id)
    }

    /// Get layout.
    pub fn get_layout(&self, id: LayoutId) -> Option<&Layout> {
        self.layouts.get(&id)
    }

    /// Iterate layouts.
    pub fn layouts(&self) -> impl Iterator<Item = &Layout> {
        self.layouts.values()
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Document-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocError {
    /// Layer id not present.
    UnknownLayer(LayerId),
    /// Default layer `"0"` cannot be removed.
    CannotRemoveDefaultLayer,
    /// Layer still referenced by entities.
    LayerInUse,
    /// Block id not present.
    UnknownBlock(BlockId),
    /// Block name already exists.
    BlockExists(String),
    /// Layout missing.
    UnknownLayout(LayoutId),
}

impl std::fmt::Display for DocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownLayer(id) => write!(f, "unknown layer {id}"),
            Self::CannotRemoveDefaultLayer => write!(f, "cannot remove default layer"),
            Self::LayerInUse => write!(f, "layer is still in use"),
            Self::UnknownBlock(id) => write!(f, "unknown block {id}"),
            Self::BlockExists(name) => write!(f, "block exists: {name}"),
            Self::UnknownLayout(id) => write!(f, "unknown layout {id}"),
        }
    }
}

impl std::error::Error for DocError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Geometry;
    use kelicad_math::Vec2;

    #[test]
    fn new_document_has_default_layer() {
        let doc = Document::new();
        assert_eq!(doc.entity_count(), 0);
        let layer = doc.get_layer(doc.default_layer_id()).unwrap();
        assert_eq!(layer.name, "0");
    }

    #[test]
    fn upsert_and_get_entity() {
        let mut doc = Document::new();
        let id = EntityId::from_uuid(Uuid::from_u128(1));
        doc.upsert_entity(EntityRecord {
            id,
            kind: EntityKind::Point,
            geometry: Geometry::Point(Vec2::new(1.0, 2.0)),
            layer_id: doc.default_layer_id(),
        });
        assert_eq!(doc.entity_count(), 1);
        assert!(doc.get_entity(id).is_some());
    }
}

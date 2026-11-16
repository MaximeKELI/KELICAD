//! In-memory document (ECS-style sparse maps).

use std::collections::HashMap;

use uuid::Uuid;

use crate::blocks::{BlockDef, BlockId};
use crate::entity::{EntityId, EntityKind};
use crate::geometry::Geometry;
use crate::layer::{Layer, LayerId};
use crate::layout::{Layout, LayoutId, PlotStyle};
use crate::styles::{DimStyle, DimStyleId, TextStyle, TextStyleId};
use crate::xref::{XrefDef, XrefId};

/// Stored entity record.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntityRecord {
    /// Id.
    pub id: EntityId,
    /// Kind (cached from geometry).
    pub kind: EntityKind,
    /// Geometry component.
    pub geometry: Geometry,
    /// Layer assignment.
    pub layer_id: LayerId,
    /// Optional property overrides (BYLAYER when default).
    #[serde(default)]
    pub props: crate::props::EntityProps,
}

impl EntityRecord {
    /// Create a record with default (BYLAYER) props.
    pub fn new(id: EntityId, geometry: Geometry, layer_id: LayerId) -> Self {
        Self {
            id,
            kind: geometry.kind(),
            geometry,
            layer_id,
            props: crate::props::EntityProps::default(),
        }
    }
}

/// Drawing units for the project (display / property inspector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawingUnits {
    /// Millimetres (paper CAD default).
    #[default]
    Millimeters,
    /// Metres.
    Meters,
    /// Inches.
    Inches,
    /// Unitless / abstract.
    Unitless,
}

impl DrawingUnits {
    /// Parse FR/EN aliases.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
                Some(Self::Millimeters)
            }
            "m" | "meter" | "meters" | "metre" | "metres" => Some(Self::Meters),
            "in" | "inch" | "inches" | "pouce" | "pouces" => Some(Self::Inches),
            "none" | "unitless" | "sans" => Some(Self::Unitless),
            _ => None,
        }
    }

    /// Canonical short name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Millimeters => "mm",
            Self::Meters => "m",
            Self::Inches => "in",
            Self::Unitless => "unitless",
        }
    }
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
    /// Text styles.
    text_styles: HashMap<TextStyleId, TextStyle>,
    /// Dimension styles.
    dim_styles: HashMap<DimStyleId, DimStyle>,
    /// Current text style for new text.
    pub current_text_style: TextStyleId,
    /// Current dim style for new dimensions.
    pub current_dim_style: DimStyleId,
    /// External references / underlays.
    xrefs: HashMap<XrefId, XrefDef>,
    /// Plot style tables by name.
    plot_styles: HashMap<String, PlotStyle>,
    /// Project drawing units.
    pub units: DrawingUnits,
    /// Free-form project title (title block / properties).
    pub project_title: String,
    /// Project scale label (e.g. `1:100`).
    pub project_scale: String,
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
        let mut text_styles = HashMap::new();
        let std_text = TextStyle::standard();
        text_styles.insert(std_text.id, std_text);
        let mut dim_styles = HashMap::new();
        let std_dim = DimStyle::standard();
        dim_styles.insert(std_dim.id, std_dim);
        let mut plot_styles = HashMap::new();
        let mono = PlotStyle::monochrome();
        plot_styles.insert(mono.name.clone(), mono);
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
            text_styles,
            dim_styles,
            current_text_style: TextStyleId::STANDARD,
            current_dim_style: DimStyleId::STANDARD,
            xrefs: HashMap::new(),
            plot_styles,
            units: DrawingUnits::Millimeters,
            project_title: "KELICAD".into(),
            project_scale: "1:100".into(),
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

    /// Upsert a text style.
    pub fn upsert_text_style(&mut self, style: TextStyle) {
        self.text_styles.insert(style.id, style);
    }

    /// Get text style.
    pub fn get_text_style(&self, id: TextStyleId) -> Option<&TextStyle> {
        self.text_styles.get(&id)
    }

    /// Iterate text styles.
    pub fn text_styles(&self) -> impl Iterator<Item = &TextStyle> {
        self.text_styles.values()
    }

    /// Upsert a dim style.
    pub fn upsert_dim_style(&mut self, style: DimStyle) {
        self.dim_styles.insert(style.id, style);
    }

    /// Get dim style.
    pub fn get_dim_style(&self, id: DimStyleId) -> Option<&DimStyle> {
        self.dim_styles.get(&id)
    }

    /// Iterate dim styles.
    pub fn dim_styles(&self) -> impl Iterator<Item = &DimStyle> {
        self.dim_styles.values()
    }

    /// Upsert an xref definition.
    pub fn upsert_xref(&mut self, def: XrefDef) {
        self.xrefs.insert(def.id, def);
    }

    /// Remove xref definition.
    pub fn remove_xref(&mut self, id: XrefId) -> Option<XrefDef> {
        self.xrefs.remove(&id)
    }

    /// Get xref.
    pub fn get_xref(&self, id: XrefId) -> Option<&XrefDef> {
        self.xrefs.get(&id)
    }

    /// Lookup xref by name (case-insensitive).
    pub fn xref_by_name(&self, name: &str) -> Option<&XrefDef> {
        let key = name.to_ascii_uppercase();
        self.xrefs
            .values()
            .find(|x| x.name.to_ascii_uppercase() == key)
    }

    /// Iterate xrefs.
    pub fn xrefs(&self) -> impl Iterator<Item = &XrefDef> {
        self.xrefs.values()
    }

    /// Upsert plot style.
    pub fn upsert_plot_style(&mut self, style: PlotStyle) {
        self.plot_styles.insert(style.name.clone(), style);
    }

    /// Get plot style by name.
    pub fn get_plot_style(&self, name: &str) -> Option<&PlotStyle> {
        self.plot_styles.get(name)
    }

    /// Iterate plot styles.
    pub fn plot_styles(&self) -> impl Iterator<Item = &PlotStyle> {
        self.plot_styles.values()
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
    /// XREF definition missing.
    UnknownXref(crate::xref::XrefId),
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
            Self::UnknownXref(id) => write!(f, "unknown xref {id}"),
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
            props: Default::default(),
        });
        assert_eq!(doc.entity_count(), 1);
        assert!(doc.get_entity(id).is_some());
    }
}

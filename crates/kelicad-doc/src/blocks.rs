//! Block definitions and inserts.

use std::collections::HashMap;

use kelicad_math::Vec2;
use uuid::Uuid;

use crate::entity::EntityId;
use crate::geometry::Geometry;

/// Block definition id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BlockId(Uuid);

impl BlockId {
    /// From UUID.
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Underlying UUID.
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Attribute definition / value.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockAttribute {
    /// Tag.
    pub tag: String,
    /// Prompt (for ATTDEF).
    pub prompt: String,
    /// Value.
    pub value: String,
    /// Position relative to block origin.
    pub position: Vec2,
    /// Text height.
    pub height: f64,
}

/// Named block definition (geometry in block coordinates).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BlockDef {
    /// Id.
    pub id: BlockId,
    /// Name (unique, case-insensitive lookup).
    pub name: String,
    /// Base / origin.
    pub origin: Vec2,
    /// Contained geometry (anonymous; not separate document entities).
    pub entities: Vec<Geometry>,
    /// Attribute definitions.
    pub attributes: Vec<BlockAttribute>,
}

/// Insert / block reference in model space.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InsertGeom {
    /// Block definition id.
    pub block_id: BlockId,
    /// Insertion point in model space.
    pub position: Vec2,
    /// Scale X/Y (uniform if equal).
    pub scale: Vec2,
    /// Rotation radians.
    pub rotation: f64,
    /// Attribute values (tag → value).
    pub attribs: HashMap<String, String>,
}

/// Source entities used when creating a block from selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockSource {
    /// Entity ids that were absorbed into the block.
    pub entity_ids: Vec<EntityId>,
}

/// Expand an insert into world-space geometries (origin → scale → rotate → translate).
pub fn expand_insert(def: &BlockDef, ins: &InsertGeom) -> Vec<Geometry> {
    let sx = if ins.scale.x.abs() < 1e-12 {
        1.0
    } else {
        ins.scale.x
    };
    let sy = if ins.scale.y.abs() < 1e-12 {
        1.0
    } else {
        ins.scale.y
    };
    // Uniform scale preferred; non-uniform approximates with avg for rotate-friendly paths.
    let s = if (sx - sy).abs() < 1e-9 {
        sx
    } else {
        (sx.abs() + sy.abs()) * 0.5
    };
    let origin = def.origin;
    def.entities
        .iter()
        .map(|g| {
            g.translated(Vec2::new(-origin.x, -origin.y))
                .scaled(Vec2::ZERO, s)
                .rotated(Vec2::ZERO, ins.rotation)
                .translated(ins.position)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_geom::Line;

    #[test]
    fn expand_translates_and_scales() {
        let id = BlockId::from_uuid(uuid::Uuid::nil());
        let def = BlockDef {
            id,
            name: "T".into(),
            origin: Vec2::ZERO,
            entities: vec![Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(2.0, 0.0)))],
            attributes: vec![],
        };
        let ins = InsertGeom {
            block_id: id,
            position: Vec2::new(10.0, 5.0),
            scale: Vec2::new(2.0, 2.0),
            rotation: 0.0,
            attribs: Default::default(),
        };
        let out = expand_insert(&def, &ins);
        match &out[0] {
            Geometry::Line(l) => {
                assert!((l.start - Vec2::new(10.0, 5.0)).length() < 1e-9);
                assert!((l.end - Vec2::new(14.0, 5.0)).length() < 1e-9);
            }
            _ => panic!("expected line"),
        }
    }
}

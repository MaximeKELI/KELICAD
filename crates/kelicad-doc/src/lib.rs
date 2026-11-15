//! Document model: entities, components, layers, blocks, layouts.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod annotation;
pub mod blocks;
pub mod document;
pub mod entity;
pub mod geometry;
pub mod layer;
pub mod layout;

pub use annotation::{
    DimKind, DimensionGeom, HatchGeom, HatchPattern, MTextGeom, TextAlign, TextGeom,
};
pub use blocks::{BlockAttribute, BlockDef, BlockId, InsertGeom};
pub use document::{DocError, Document, EntityRecord};
pub use entity::{EntityId, EntityKind};
pub use geometry::Geometry;
pub use layer::{Layer, LayerFlags, LayerId};
pub use layout::{Layout, LayoutId, PaperSize, Viewport};

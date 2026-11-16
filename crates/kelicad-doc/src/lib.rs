//! Document model: entities, components, layers, blocks, layouts, styles, xrefs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod annotation;
pub mod blocks;
pub mod document;
pub mod entity;
pub mod geometry;
pub mod layer;
pub mod layout;
pub mod linetype;
pub mod mtext;
pub mod ray;
pub mod styles;
pub mod templates;
pub mod xref;

pub use annotation::{
    DimKind, DimensionGeom, HatchGeom, HatchPattern, LeaderGeom, MTextGeom, TextAlign, TextGeom,
};
pub use blocks::{BlockAttribute, BlockDef, BlockId, InsertGeom};
pub use document::{DocError, Document, EntityRecord};
pub use entity::{EntityId, EntityKind};
pub use geometry::Geometry;
pub use layer::{Layer, LayerFlags, LayerId};
pub use layout::{Layout, LayoutId, PaperSize, PlotStyle, Viewport};
pub use linetype::{resolve_linetype, Linetype};
pub use mtext::{parse_mtext, MTextAlign, MTextRun};
pub use ray::{RayGeom, XLineGeom};
pub use styles::{DimArrowKind, DimStyle, DimStyleId, TextStyle, TextStyleId};
pub use templates::{template_entities, TemplateEntity, TemplateKind};
pub use xref::{XrefDef, XrefId, XrefInsertGeom, XrefKind};

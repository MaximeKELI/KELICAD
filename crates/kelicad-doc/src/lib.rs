//! Document model: entities, components, and layers.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod document;
pub mod entity;
pub mod geometry;
pub mod layer;

pub use document::{DocError, Document, EntityRecord};
pub use entity::{EntityId, EntityKind};
pub use geometry::Geometry;
pub use layer::{Layer, LayerFlags, LayerId};

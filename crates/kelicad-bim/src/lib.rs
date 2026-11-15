//! BIM elements, schedules, collisions, IFC subset (ADR 0011).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod collide;
mod element;
mod error;
mod ifc;
mod schedule;
mod wall;

pub use collide::collisions;
pub use element::{BimElement, BimId, BimKind};
pub use error::BimError;
pub use ifc::{export_ifc, import_ifc};
pub use schedule::schedule_csv;
pub use wall::{door_opening_box, slab_from_rect, wall_from_baseline};

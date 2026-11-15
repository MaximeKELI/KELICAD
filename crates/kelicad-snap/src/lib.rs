//! Object snaps for the command engine.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod engine;
pub mod kinds;

pub use engine::{SnapEngine, SnapHit, SnapQuery};
pub use kinds::SnapKind;

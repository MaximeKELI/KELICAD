//! `.kelicad` SQLite persistence (schema v0).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod geometry_json;
pub mod kelicad_file;

pub use error::IoError;
pub use kelicad_file::{load_document, save_document, SCHEMA_VERSION};

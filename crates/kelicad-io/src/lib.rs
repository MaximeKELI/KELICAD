//! Document persistence and interchange (`.kelicad`, DXF, SVG, PDF).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod dxf;
pub mod error;
pub mod geometry_json;
pub mod kelicad_file;
pub mod pdf;
pub mod svg;

pub use dxf::{export_dxf, import_dxf, ImportReport};
pub use error::IoError;
pub use kelicad_file::{load_document, save_document, SCHEMA_VERSION};
pub use pdf::export_pdf;
pub use svg::export_svg;

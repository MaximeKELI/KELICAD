//! Document persistence and interchange (`.kelicad`, DXF, SVG, PDF).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod dxf;
pub mod error;
pub mod geometry_json;
pub mod pdf_lite;
pub mod shp;
pub mod svg;

#[cfg(feature = "native")]
pub mod kelicad_file;
#[cfg(feature = "native")]
pub mod pdf;

pub use dxf::{export_dxf, import_dxf, ImportReport};
pub use error::IoError;
pub use pdf_lite::export_pdf_lite;
pub use shp::{import_shp, import_shp_bytes, ShpImportReport};
pub use svg::export_svg;

#[cfg(feature = "native")]
pub use kelicad_file::{load_document, save_document, SCHEMA_VERSION};
#[cfg(feature = "native")]
pub use pdf::export_pdf;

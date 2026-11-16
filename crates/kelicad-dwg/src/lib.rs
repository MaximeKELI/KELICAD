//! Pluggable DWG bridge (ADR 0012).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod bridge;
mod error;
mod external;
mod mock;

#[cfg(test)]
mod rigorous;

pub use bridge::{export_dwg, import_dwg, DwgBridge};
pub use error::DwgError;
pub use external::ExternalDwgBridge;
pub use mock::{is_proxy_dwg, MockDwgBridge, PROXY_MAGIC};

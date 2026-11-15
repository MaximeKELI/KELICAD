//! Solid kernel trait and Builtin mesh implementation (ADR 0011).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod aabb3;
mod error;
mod kernel;
mod mesh;
mod section;
mod types;
mod views;

pub use aabb3::Aabb3;
pub use error::SolidError;
pub use kernel::{BuiltinSolidKernel, SolidKernel};
pub use mesh::Mesh;
pub use types::{Plane, Profile2, SolidBody, SolidId};
pub use views::{derive_view, ViewKind};

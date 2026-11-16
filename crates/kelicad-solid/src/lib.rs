//! Solid kernel trait and Builtin mesh implementation (ADR 0011).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod aabb3;
mod advanced;
mod error;
mod kernel;
mod mesh;
mod section;
mod types;
mod views;

#[cfg(test)]
mod rigorous;

pub use aabb3::Aabb3;
pub use advanced::{cone_mesh, loft_xy_profiles, sphere_mesh, sweep_xy_along_path, wedge_mesh};
pub use error::SolidError;
pub use kernel::{BuiltinSolidKernel, SolidKernel};
pub use mesh::Mesh;
pub use types::{Plane, Profile2, SolidBody, SolidId};
pub use views::{derive_view, ViewKind};

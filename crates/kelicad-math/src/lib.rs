//! Core math types for KELICAD.
//!
//! Absolute floating-point equality is never used for geometry; prefer
//! [`Tolerance`] and the robust predicates in [`predicates`].

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod mat;
pub mod predicates;
pub mod tolerance;
pub mod vec;

#[cfg(test)]
mod rigorous;

pub use mat::{Mat3, Mat4};
pub use predicates::{incircle, orient2d, Orient};
pub use tolerance::Tolerance;
pub use vec::{Vec2, Vec3};

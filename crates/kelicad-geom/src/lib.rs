//! 2D geometry primitives and intersections.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod aabb;
pub mod arc;
pub mod circle;
pub mod intersect;
pub mod line;
pub mod polyline;
pub mod transform;
pub mod trim;

#[cfg(test)]
mod rigorous;

pub use aabb::Aabb;
pub use arc::Arc;
pub use circle::Circle;
pub use intersect::{
    circle_circle, line_circle, line_line, CircleCircleResult, LineCircleResult, LineLineResult,
};
pub use line::Line;
pub use polyline::Polyline;

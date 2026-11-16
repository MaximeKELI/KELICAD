//! 2D geometry primitives and intersections.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod aabb;
pub mod arc;
pub mod circle;
pub mod construct;
pub mod ellipse;
pub mod intersect;
pub mod line;
pub mod linetype;
pub mod modify;
pub mod pat;
pub mod polyline;
pub mod spline;
pub mod transform;
pub mod trim;

#[cfg(test)]
mod rigorous;

pub use aabb::Aabb;
pub use arc::Arc;
pub use circle::Circle;
pub use construct::{
    angle_bisector, chamfer_lines, circle_from_2_points, circle_from_3_points, fillet_lines,
    regular_polygon_center_vertex, regular_polygon_side, tangents_circle_circle,
    tangents_point_circle,
};
pub use ellipse::Ellipse;
pub use intersect::{
    circle_circle, line_circle, line_line, CircleCircleResult, LineCircleResult, LineLineResult,
};
pub use line::Line;
pub use linetype::{dash_line, dash_polyline};
pub use modify::{
    align_2point, align_transform_point, break_line_at, break_line_near, join_collinear_lines,
    join_polylines, lengthen_line, polar_array_copies, rectangular_array_offsets, stretch_line,
    stretch_point, stretch_polyline,
};
pub use pat::{
    builtin_pattern, builtin_patterns, generate_hatch, load_pat_file, parse_pat, resolve_pattern,
    PatLineDef, PatPattern,
};
pub use polyline::Polyline;
pub use spline::Spline;

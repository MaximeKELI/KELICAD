//! Associative derived 2D views from solids.

use kelicad_doc::Geometry;
use kelicad_math::Vec2;

use crate::section::{aabb_outline_xy, section_segments, segments_to_geometries};
use crate::types::{Plane, SolidBody};

/// Kind of derived drawing view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// Top / plan (looking down −Z), outline from AABB + mid Z section.
    Plan,
    /// Elevation looking along −Y (X right, Z up).
    ElevationX,
    /// Elevation looking along −X (Y right, Z up).
    ElevationY,
    /// Arbitrary plane section.
    Section,
}

/// Derive 2D geometries from a solid body.
pub fn derive_view(
    body: &SolidBody,
    kind: ViewKind,
    section_plane: Option<Plane>,
) -> Vec<Geometry> {
    match kind {
        ViewKind::Plan => {
            let mut out = vec![aabb_outline_xy(
                Vec2::new(body.aabb.min.x, body.aabb.min.y),
                Vec2::new(body.aabb.max.x, body.aabb.max.y),
            )];
            let z = (body.aabb.min.z + body.aabb.max.z) * 0.5;
            let segs = section_segments(&body.mesh, Plane::z(z));
            out.extend(segments_to_geometries(&segs, Plane::z(z)));
            out
        }
        ViewKind::ElevationX => {
            // Project AABB to XZ as XY polyline.
            let min = Vec2::new(body.aabb.min.x, body.aabb.min.z);
            let max = Vec2::new(body.aabb.max.x, body.aabb.max.z);
            let mut out = vec![aabb_outline_xy(min, max)];
            let y = (body.aabb.min.y + body.aabb.max.y) * 0.5;
            let plane = Plane {
                point: kelicad_math::Vec3::new(0.0, y, 0.0),
                normal: kelicad_math::Vec3::new(0.0, 1.0, 0.0),
            };
            let segs = section_segments(&body.mesh, plane);
            out.extend(segments_to_geometries(&segs, plane));
            out
        }
        ViewKind::ElevationY => {
            let min = Vec2::new(body.aabb.min.y, body.aabb.min.z);
            let max = Vec2::new(body.aabb.max.y, body.aabb.max.z);
            let mut out = vec![aabb_outline_xy(min, max)];
            let x = (body.aabb.min.x + body.aabb.max.x) * 0.5;
            let plane = Plane {
                point: kelicad_math::Vec3::new(x, 0.0, 0.0),
                normal: kelicad_math::Vec3::new(1.0, 0.0, 0.0),
            };
            let segs = section_segments(&body.mesh, plane);
            out.extend(segments_to_geometries(&segs, plane));
            out
        }
        ViewKind::Section => {
            let plane = section_plane.unwrap_or_else(|| {
                let z = (body.aabb.min.z + body.aabb.max.z) * 0.5;
                Plane::z(z)
            });
            let segs = section_segments(&body.mesh, plane);
            segments_to_geometries(&segs, plane)
        }
    }
}

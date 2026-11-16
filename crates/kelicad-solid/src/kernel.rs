//! SolidKernel trait and Builtin mesh implementation.

use kelicad_doc::Geometry;
use kelicad_math::{Vec2, Vec3};

use crate::aabb3::Aabb3;
use crate::error::SolidError;
use crate::mesh::{box_mesh, cylinder_mesh, extrude_xy_profile, revolve_xy_profile, Mesh};
use crate::section::{section_segments, segments_to_geometries};
use crate::types::{Plane, Profile2, SolidBody, SolidId};

/// Replaceable solid modelling backend (ADR 0011).
pub trait SolidKernel {
    /// Extrude a closed XY profile along +Z.
    fn extrude(&self, profile: &Profile2, height: f64) -> Result<SolidBody, SolidError>;
    /// Revolve profile around +Y by `degrees`.
    fn revolve(
        &self,
        profile: &Profile2,
        degrees: f64,
        segments: usize,
    ) -> Result<SolidBody, SolidError>;
    /// Axis-aligned box.
    fn box_solid(&self, min: Vec3, max: Vec3) -> Result<SolidBody, SolidError>;
    /// Cylinder along +Z.
    fn cylinder(
        &self,
        center_xy: Vec2,
        radius: f64,
        height: f64,
        segments: usize,
    ) -> Result<SolidBody, SolidError>;
    /// Mesh union (append + union AABB).
    fn boolean_union(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError>;
    /// Difference A − B (triangle filter by B AABB; reliable for wall/opening).
    fn boolean_difference(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError>;
    /// Intersection A ∩ B (keep triangles of A whose centroid is in B AABB).
    fn boolean_intersection(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError>;
    /// Section plane → 2D geometries.
    fn section(&self, body: &SolidBody, plane: Plane) -> Vec<Geometry>;
    /// Cached AABB.
    fn aabb(&self, body: &SolidBody) -> Aabb3;
}

/// Pure-Rust triangle-mesh kernel (default for WASM/CI).
#[derive(Debug, Clone, Default)]
pub struct BuiltinSolidKernel;

impl SolidKernel for BuiltinSolidKernel {
    fn extrude(&self, profile: &Profile2, height: f64) -> Result<SolidBody, SolidError> {
        if height <= 0.0 || !height.is_finite() {
            return Err(SolidError::InvalidArg(
                "height must be finite and > 0".into(),
            ));
        }
        let mesh = extrude_xy_profile(&profile.vertices, height);
        Ok(SolidBody::from_mesh(SolidId::new(), mesh))
    }

    fn revolve(
        &self,
        profile: &Profile2,
        degrees: f64,
        segments: usize,
    ) -> Result<SolidBody, SolidError> {
        if degrees.abs() < 1e-9 {
            return Err(SolidError::InvalidArg(
                "revolve degrees must be non-zero".into(),
            ));
        }
        let mesh = revolve_xy_profile(&profile.vertices, degrees, segments);
        if mesh.is_empty() {
            return Err(SolidError::InvalidArg("revolve produced empty mesh".into()));
        }
        Ok(SolidBody::from_mesh(SolidId::new(), mesh))
    }

    fn box_solid(&self, min: Vec3, max: Vec3) -> Result<SolidBody, SolidError> {
        if max.x <= min.x || max.y <= min.y || max.z <= min.z {
            return Err(SolidError::InvalidArg(
                "box max must be > min on each axis".into(),
            ));
        }
        Ok(SolidBody::from_mesh(SolidId::new(), box_mesh(min, max)))
    }

    fn cylinder(
        &self,
        center_xy: Vec2,
        radius: f64,
        height: f64,
        segments: usize,
    ) -> Result<SolidBody, SolidError> {
        if radius <= 0.0 || height <= 0.0 {
            return Err(SolidError::InvalidArg("radius/height must be > 0".into()));
        }
        Ok(SolidBody::from_mesh(
            SolidId::new(),
            cylinder_mesh(center_xy, radius, height, segments),
        ))
    }

    fn boolean_union(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError> {
        let mut mesh = a.mesh.clone();
        mesh.append(&b.mesh);
        Ok(SolidBody::from_mesh(SolidId::new(), mesh))
    }

    fn boolean_difference(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError> {
        filter_triangles(a, b.aabb, false)
    }

    fn boolean_intersection(&self, a: &SolidBody, b: &SolidBody) -> Result<SolidBody, SolidError> {
        // Intersect AABB first; empty → error.
        let inter = aabb_intersection(a.aabb, b.aabb).ok_or_else(|| {
            SolidError::Boolean("intersection is empty (AABBs disjoint)".into())
        })?;
        let mut body = filter_triangles(a, inter, true)?;
        body.aabb = inter;
        Ok(body)
    }

    fn section(&self, body: &SolidBody, plane: Plane) -> Vec<Geometry> {
        let segs = section_segments(&body.mesh, plane);
        segments_to_geometries(&segs, plane)
    }

    fn aabb(&self, body: &SolidBody) -> Aabb3 {
        body.aabb
    }
}

fn point_in_aabb(p: Vec3, b: Aabb3) -> bool {
    p.x >= b.min.x
        && p.x <= b.max.x
        && p.y >= b.min.y
        && p.y <= b.max.y
        && p.z >= b.min.z
        && p.z <= b.max.z
}

fn aabb_intersection(a: Aabb3, b: Aabb3) -> Option<Aabb3> {
    let min = Vec3::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y), a.min.z.max(b.min.z));
    let max = Vec3::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y), a.max.z.min(b.max.z));
    if max.x <= min.x || max.y <= min.y || max.z <= min.z {
        None
    } else {
        Some(Aabb3 { min, max })
    }
}

/// Keep (`keep_inside=true`) or discard (`false`) triangles by centroid vs `region`.
fn filter_triangles(
    a: &SolidBody,
    region: Aabb3,
    keep_inside: bool,
) -> Result<SolidBody, SolidError> {
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let src = &a.mesh;
    let mut i = 0;
    while i + 2 < src.indices.len() {
        let ia = src.indices[i] as usize;
        let ib = src.indices[i + 1] as usize;
        let ic = src.indices[i + 2] as usize;
        i += 3;
        let pa = src.positions[ia];
        let pb = src.positions[ib];
        let pc = src.positions[ic];
        let centroid = Vec3::new(
            (pa.x + pb.x + pc.x) / 3.0,
            (pa.y + pb.y + pc.y) / 3.0,
            (pa.z + pb.z + pc.z) / 3.0,
        );
        // Majority-of-vertices vote for more stable CSG on thin features.
        let votes = [pa, pb, pc]
            .iter()
            .filter(|p| point_in_aabb(**p, region))
            .count();
        let inside = votes >= 2 || (votes == 1 && point_in_aabb(centroid, region));
        if inside != keep_inside {
            continue;
        }
        let base = positions.len() as u32;
        positions.push(pa);
        positions.push(pb);
        positions.push(pc);
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
    if indices.is_empty() {
        return Err(SolidError::Boolean(
            "boolean filter removed all triangles".into(),
        ));
    }
    let mesh = Mesh { positions, indices };
    let mut body = SolidBody::from_mesh(SolidId::new(), mesh);
    if !keep_inside && region.overlaps_volume(a.aabb) {
        body.aabb = shrink_aabb_against_cutter(a.aabb, region);
        let mesh_aabb = body.mesh.aabb();
        body.aabb = Aabb3 {
            min: Vec3::new(
                body.aabb.min.x.max(mesh_aabb.min.x),
                body.aabb.min.y.max(mesh_aabb.min.y),
                body.aabb.min.z.max(mesh_aabb.min.z),
            ),
            max: Vec3::new(
                body.aabb.max.x.min(mesh_aabb.max.x),
                body.aabb.max.y.min(mesh_aabb.max.y),
                body.aabb.max.z.min(mesh_aabb.max.z),
            ),
        };
    }
    Ok(body)
}

/// If cutter overlaps an exterior face, pull that face inward (simple AABB CSG).
fn shrink_aabb_against_cutter(a: Aabb3, cut: Aabb3) -> Aabb3 {
    let mut out = a;
    // If cutter covers full YZ at the max-X end, shrink max.x.
    if cut.min.x < a.max.x
        && cut.max.x >= a.max.x - 1e-9
        && cut.min.y <= a.min.y + 1e-6
        && cut.max.y >= a.max.y - 1e-6
        && cut.min.z <= a.min.z + 1e-6
        && cut.max.z >= a.min.z
    {
        out.max.x = out.max.x.min(cut.min.x);
    }
    if cut.max.x > a.min.x
        && cut.min.x <= a.min.x + 1e-9
        && cut.min.y <= a.min.y + 1e-6
        && cut.max.y >= a.max.y - 1e-6
    {
        out.min.x = out.min.x.max(cut.max.x);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{derive_view, ViewKind};

    #[test]
    fn extrude_rectangle_height() {
        let k = BuiltinSolidKernel;
        let p = Profile2::rectangle(Vec2::ZERO, 4.0, 3.0).unwrap();
        let body = k.extrude(&p, 2.5).unwrap();
        assert!((body.aabb.max.z - body.aabb.min.z - 2.5).abs() < 1e-9);
        assert!((body.aabb.max.x - body.aabb.min.x - 4.0).abs() < 1e-9);
        let plan = derive_view(&body, ViewKind::Plan, None);
        assert!(!plan.is_empty());
        assert!(plan.iter().any(|g| matches!(g, Geometry::Polyline(_))));
    }

    #[test]
    fn section_midplane_outline() {
        let k = BuiltinSolidKernel;
        let body = k.box_solid(Vec3::ZERO, Vec3::new(4.0, 3.0, 2.0)).unwrap();
        let geoms = k.section(&body, Plane::z(1.0));
        assert!(!geoms.is_empty());
    }

    #[test]
    fn boolean_difference_shrinks_aabb() {
        let k = BuiltinSolidKernel;
        let wall = k.box_solid(Vec3::ZERO, Vec3::new(10.0, 0.2, 3.0)).unwrap();
        let opening = k
            .box_solid(Vec3::new(9.0, -0.05, 0.0), Vec3::new(11.0, 0.25, 2.2))
            .unwrap();
        let diff = k.boolean_difference(&wall, &opening).unwrap();
        assert!(diff.aabb.volume() < wall.aabb.volume());
        assert!(diff.aabb.max.x < wall.aabb.max.x - 0.5);
    }

    #[test]
    fn revolve_quarter_nonempty() {
        let k = BuiltinSolidKernel;
        let profile = Profile2::from_vertices(vec![
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(1.0, 1.0),
        ])
        .unwrap();
        let body = k.revolve(&profile, 90.0, 12).unwrap();
        assert!(!body.mesh.is_empty());
        assert!(body.aabb.volume() > 0.0);
    }
}

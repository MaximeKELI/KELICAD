//! Plane–mesh section → 2D-ish line segments (in plane coords projected to XY for Z planes).

use kelicad_doc::Geometry;
use kelicad_geom::{Line, Polyline};
use kelicad_math::{Vec2, Vec3};

use crate::mesh::Mesh;
use crate::types::Plane;

/// Intersect mesh with plane; return line segments as 3D endpoints.
pub fn section_segments(mesh: &Mesh, plane: Plane) -> Vec<(Vec3, Vec3)> {
    let mut segs = Vec::new();
    let idx = &mesh.indices;
    let pos = &mesh.positions;
    let mut i = 0;
    while i + 2 < idx.len() {
        let a = pos[idx[i] as usize];
        let b = pos[idx[i + 1] as usize];
        let c = pos[idx[i + 2] as usize];
        i += 3;
        if let Some(seg) = triangle_plane_intersect(a, b, c, plane) {
            segs.push(seg);
        }
    }
    segs
}

fn triangle_plane_intersect(a: Vec3, b: Vec3, c: Vec3, plane: Plane) -> Option<(Vec3, Vec3)> {
    let da = plane.signed_distance(a);
    let db = plane.signed_distance(b);
    let dc = plane.signed_distance(c);
    let mut hits = Vec::with_capacity(3);
    push_edge_hit(&mut hits, a, b, da, db);
    push_edge_hit(&mut hits, b, c, db, dc);
    push_edge_hit(&mut hits, c, a, dc, da);
    // Dedup near points.
    if hits.len() >= 2 {
        let p0 = hits[0];
        let p1 = hits.iter().copied().find(|p| (*p - p0).length() > 1e-9)?;
        Some((p0, p1))
    } else {
        None
    }
}

fn push_edge_hit(out: &mut Vec<Vec3>, a: Vec3, b: Vec3, da: f64, db: f64) {
    if da * db < 0.0 {
        let t = da / (da - db);
        out.push(a + (b - a) * t);
    } else if da.abs() < 1e-9 {
        out.push(a);
    } else if db.abs() < 1e-9 {
        out.push(b);
    }
}

/// Convert section segments to document geometries (project: drop nearly-constant axis).
pub fn segments_to_geometries(segs: &[(Vec3, Vec3)], plane: Plane) -> Vec<Geometry> {
    let n = plane.normal;
    let abs = Vec3::new(n.x.abs(), n.y.abs(), n.z.abs());
    segs.iter()
        .map(|(a, b)| {
            let (p0, p1) = project_pair(*a, *b, abs);
            Geometry::Line(Line::new(p0, p1))
        })
        .collect()
}

fn project_pair(a: Vec3, b: Vec3, abs_n: Vec3) -> (Vec2, Vec2) {
    // Project onto the two axes with smallest |normal| components.
    if abs_n.z >= abs_n.x && abs_n.z >= abs_n.y {
        // Horizontal section → XY
        (Vec2::new(a.x, a.y), Vec2::new(b.x, b.y))
    } else if abs_n.y >= abs_n.x {
        // Elevation looking along Y → XZ as XY
        (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z))
    } else {
        // Elevation looking along X → YZ as XY
        (Vec2::new(a.y, a.z), Vec2::new(b.y, b.z))
    }
}

/// Outline of AABB projected for plan/elevation helpers.
pub fn aabb_outline_xy(min: Vec2, max: Vec2) -> Geometry {
    Geometry::Polyline(Polyline {
        vertices: vec![min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)],
        closed: true,
    })
}

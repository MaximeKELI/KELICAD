//! Triangle mesh storage.

use kelicad_math::Vec3;

use crate::aabb3::Aabb3;

/// Indexed triangle mesh.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    /// Vertex positions.
    pub positions: Vec<Vec3>,
    /// Triangle indices (len % 3 == 0).
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Empty mesh.
    pub fn empty() -> Self {
        Self {
            positions: vec![],
            indices: vec![],
        }
    }

    /// True if no triangles.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Triangle count.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// AABB of all vertices.
    pub fn aabb(&self) -> Aabb3 {
        Aabb3::from_points(self.positions.iter().copied())
    }

    /// Append another mesh (offsets indices).
    pub fn append(&mut self, other: &Mesh) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Translate all vertices.
    pub fn translated(&self, delta: Vec3) -> Self {
        Self {
            positions: self.positions.iter().map(|p| *p + delta).collect(),
            indices: self.indices.clone(),
        }
    }
}

/// Ear-clip style fan triangulation for a simple polygon (assumes near-convex / no holes).
pub fn triangulate_polygon(verts: &[Vec3]) -> Vec<[u32; 3]> {
    let n = verts.len();
    if n < 3 {
        return vec![];
    }
    // Fan from vertex 0 — correct for convex profiles (rectangles, walls).
    let mut tris = Vec::with_capacity(n - 2);
    for i in 1..n - 1 {
        tris.push([0, i as u32, (i + 1) as u32]);
    }
    tris
}

/// Extrude a closed XY profile along +Z by `height`.
pub fn extrude_xy_profile(profile: &[kelicad_math::Vec2], height: f64) -> Mesh {
    let n = profile.len();
    let mut positions = Vec::with_capacity(n * 2);
    for v in profile {
        positions.push(Vec3::new(v.x, v.y, 0.0));
    }
    for v in profile {
        positions.push(Vec3::new(v.x, v.y, height));
    }
    let mut indices = Vec::new();
    // Bottom (CW when viewed from +Z so normal points -Z).
    let bottom: Vec<Vec3> = positions[..n].to_vec();
    for tri in triangulate_polygon(&bottom) {
        indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
    }
    // Top (CCW, normal +Z).
    let top: Vec<Vec3> = positions[n..].to_vec();
    for tri in triangulate_polygon(&top) {
        indices.extend_from_slice(&[n as u32 + tri[0], n as u32 + tri[1], n as u32 + tri[2]]);
    }
    // Sides.
    for i in 0..n {
        let j = (i + 1) % n;
        let b0 = i as u32;
        let b1 = j as u32;
        let t0 = n as u32 + i as u32;
        let t1 = n as u32 + j as u32;
        indices.extend_from_slice(&[b0, b1, t1, b0, t1, t0]);
    }
    Mesh { positions, indices }
}

/// Axis-aligned box mesh.
pub fn box_mesh(min: Vec3, max: Vec3) -> Mesh {
    let profile = [
        kelicad_math::Vec2::new(min.x, min.y),
        kelicad_math::Vec2::new(max.x, min.y),
        kelicad_math::Vec2::new(max.x, max.y),
        kelicad_math::Vec2::new(min.x, max.y),
    ];
    let mut m = extrude_xy_profile(&profile, max.z - min.z);
    // Shift to min.z
    for p in &mut m.positions {
        p.z += min.z;
    }
    m
}

/// Approximate cylinder along Z.
pub fn cylinder_mesh(
    center_xy: kelicad_math::Vec2,
    radius: f64,
    height: f64,
    segments: usize,
) -> Mesh {
    let segs = segments.max(8);
    let mut profile = Vec::with_capacity(segs);
    for i in 0..segs {
        let a = (i as f64) * std::f64::consts::TAU / (segs as f64);
        profile.push(kelicad_math::Vec2::new(
            center_xy.x + radius * a.cos(),
            center_xy.y + radius * a.sin(),
        ));
    }
    extrude_xy_profile(&profile, height)
}

/// Revolve an XY profile around +Y through `degrees` (tessellated).
pub fn revolve_xy_profile(profile: &[kelicad_math::Vec2], degrees: f64, segments: usize) -> Mesh {
    let segs = segments.max(4);
    let rad = degrees.to_radians();
    let n = profile.len();
    let mut positions = Vec::new();
    for s in 0..=segs {
        let t = (s as f64) / (segs as f64);
        let ang = rad * t;
        let (sin, cos) = ang.sin_cos();
        // Rotate around Y: (x,y,z=0) -> (x*cos, y, x*sin)
        for v in profile {
            positions.push(Vec3::new(v.x * cos, v.y, v.x * sin));
        }
    }
    let mut indices = Vec::new();
    for s in 0..segs {
        let a = (s * n) as u32;
        let b = ((s + 1) * n) as u32;
        for i in 0..n {
            let j = ((i + 1) % n) as u32;
            let i = i as u32;
            indices.extend_from_slice(&[a + i, a + j, b + j, a + i, b + j, b + i]);
        }
    }
    // Caps if not full 360.
    if (degrees - 360.0).abs() > 1e-6 && n >= 3 {
        let start: Vec<Vec3> = positions[..n].to_vec();
        for tri in triangulate_polygon(&start) {
            indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
        }
        let end_base = (segs * n) as u32;
        let end: Vec<Vec3> = positions[segs * n..].to_vec();
        for tri in triangulate_polygon(&end) {
            indices.extend_from_slice(&[end_base + tri[0], end_base + tri[1], end_base + tri[2]]);
        }
    }
    Mesh { positions, indices }
}

//! Solid identifiers, profiles, planes, bodies.

use kelicad_math::{Vec2, Vec3};
use uuid::Uuid;

use crate::aabb3::Aabb3;
use crate::error::SolidError;
use crate::mesh::Mesh;

/// Stable solid body id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SolidId(pub Uuid);

impl SolidId {
    /// Allocate a new id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for SolidId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SolidId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Closed 2D profile in the XY plane (z = 0).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Profile2 {
    /// Vertices in order (CCW preferred). Last may equal first; closed implied.
    pub vertices: Vec<Vec2>,
}

impl Profile2 {
    /// From a closed polyline vertex list (at least 3 distinct).
    pub fn from_vertices(mut vertices: Vec<Vec2>) -> Result<Self, SolidError> {
        if vertices.len() >= 2 && vertices.first() == vertices.last() {
            vertices.pop();
        }
        if vertices.len() < 3 {
            return Err(SolidError::InvalidProfile(
                "need ≥3 vertices for a closed profile".into(),
            ));
        }
        Ok(Self { vertices })
    }

    /// Axis-aligned rectangle.
    pub fn rectangle(origin: Vec2, width: f64, height: f64) -> Result<Self, SolidError> {
        if width <= 0.0 || height <= 0.0 {
            return Err(SolidError::InvalidProfile(
                "rectangle width/height must be > 0".into(),
            ));
        }
        Self::from_vertices(vec![
            origin,
            Vec2::new(origin.x + width, origin.y),
            Vec2::new(origin.x + width, origin.y + height),
            Vec2::new(origin.x, origin.y + height),
        ])
    }
}

/// Infinite plane: `normal · (p - point) = 0`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Plane {
    /// A point on the plane.
    pub point: Vec3,
    /// Unit-ish normal.
    pub normal: Vec3,
}

impl Plane {
    /// Horizontal plane at z.
    pub fn z(z: f64) -> Self {
        Self {
            point: Vec3::new(0.0, 0.0, z),
            normal: Vec3::new(0.0, 0.0, 1.0),
        }
    }

    /// Signed distance from point to plane.
    pub fn signed_distance(self, p: Vec3) -> f64 {
        let n = self.normal;
        let len = n.length();
        if len < 1e-15 {
            return 0.0;
        }
        (n / len).dot(p - self.point)
    }
}

/// A solid body: triangle mesh + cached AABB.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SolidBody {
    /// Id.
    pub id: SolidId,
    /// Triangle mesh.
    pub mesh: Mesh,
    /// Cached AABB.
    pub aabb: Aabb3,
}

impl SolidBody {
    /// Build from mesh, computing AABB.
    pub fn from_mesh(id: SolidId, mesh: Mesh) -> Self {
        let aabb = mesh.aabb();
        Self { id, mesh, aabb }
    }
}

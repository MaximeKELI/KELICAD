//! 3D axis-aligned bounding box.

use kelicad_math::Vec3;

/// Axis-aligned bounding box in 3D.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Aabb3 {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb3 {
    /// Empty inverted box (for accumulation).
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    /// Box from two corners.
    pub fn from_corners(a: Vec3, b: Vec3) -> Self {
        Self {
            min: Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// Expand to include a point.
    pub fn include_point(&mut self, p: Vec3) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.min.z = self.min.z.min(p.z);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
        self.max.z = self.max.z.max(p.z);
    }

    /// From an iterator of points.
    pub fn from_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut b = Self::empty();
        for p in points {
            b.include_point(p);
        }
        b
    }

    /// True if this box intersects `other` (inclusive, epsilon-expanded).
    pub fn intersects(self, other: Self) -> bool {
        let eps = 1e-9;
        self.min.x <= other.max.x + eps
            && self.max.x + eps >= other.min.x
            && self.min.y <= other.max.y + eps
            && self.max.y + eps >= other.min.y
            && self.min.z <= other.max.z + eps
            && self.max.z + eps >= other.min.z
    }

    /// True if strictly overlapping (positive volume intersection).
    pub fn overlaps_volume(self, other: Self) -> bool {
        let eps = 1e-9;
        self.min.x < other.max.x - eps
            && self.max.x > other.min.x + eps
            && self.min.y < other.max.y - eps
            && self.max.y > other.min.y + eps
            && self.min.z < other.max.z - eps
            && self.max.z > other.min.z + eps
    }

    /// Size vector.
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    /// Approximate volume.
    pub fn volume(self) -> f64 {
        let s = self.size();
        (s.x.max(0.0)) * (s.y.max(0.0)) * (s.z.max(0.0))
    }

    /// Union of two boxes.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }
}

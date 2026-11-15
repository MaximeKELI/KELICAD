//! Axis-aligned bounding boxes.

use kelicad_math::{Tolerance, Vec2};

/// Axis-aligned bounding box in 2D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Minimum corner.
    pub min: Vec2,
    /// Maximum corner.
    pub max: Vec2,
}

impl Aabb {
    /// Empty box (inverted extents); union with any point expands correctly.
    pub const EMPTY: Self = Self {
        min: Vec2::new(f64::INFINITY, f64::INFINITY),
        max: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    /// Create from two corners (order-independent).
    pub fn from_points(a: Vec2, b: Vec2) -> Self {
        Self {
            min: Vec2::new(a.x.min(b.x), a.y.min(b.y)),
            max: Vec2::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// From center and half-extents.
    pub fn from_center_half_extents(center: Vec2, half: Vec2) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Expand to include a point.
    pub fn expand(self, p: Vec2) -> Self {
        Self {
            min: Vec2::new(self.min.x.min(p.x), self.min.y.min(p.y)),
            max: Vec2::new(self.max.x.max(p.x), self.max.y.max(p.y)),
        }
    }

    /// Union with another box.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Vec2::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Vec2::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// True if the boxes overlap (including touching edges within `tol`).
    pub fn intersects(self, other: Self, tol: Tolerance) -> bool {
        self.min.x <= other.max.x + tol.abs
            && self.max.x + tol.abs >= other.min.x
            && self.min.y <= other.max.y + tol.abs
            && self.max.y + tol.abs >= other.min.y
    }

    /// True if point is inside or on boundary within `tol`.
    pub fn contains_point(self, p: Vec2, tol: Tolerance) -> bool {
        p.x >= self.min.x - tol.abs
            && p.x <= self.max.x + tol.abs
            && p.y >= self.min.y - tol.abs
            && p.y <= self.max.y + tol.abs
    }

    /// Center.
    pub fn center(self) -> Vec2 {
        Vec2::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    /// Width × height extents.
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_from_empty() {
        let b = Aabb::EMPTY
            .expand(Vec2::new(1.0, 2.0))
            .expand(Vec2::new(-1.0, 4.0));
        assert_eq!(b.min, Vec2::new(-1.0, 2.0));
        assert_eq!(b.max, Vec2::new(1.0, 4.0));
    }

    #[test]
    fn intersecting_boxes() {
        let a = Aabb::from_points(Vec2::ZERO, Vec2::new(2.0, 2.0));
        let b = Aabb::from_points(Vec2::new(1.0, 1.0), Vec2::new(3.0, 3.0));
        assert!(a.intersects(b, Tolerance::DEFAULT));
    }
}

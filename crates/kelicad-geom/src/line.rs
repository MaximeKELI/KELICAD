//! Finite line segments.

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;

/// Finite line segment from `start` to `end`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    /// Start point.
    pub start: Vec2,
    /// End point.
    pub end: Vec2,
}

impl Line {
    /// Create a segment.
    pub const fn new(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }

    /// Direction vector (not normalized).
    #[inline]
    pub fn delta(self) -> Vec2 {
        self.end - self.start
    }

    /// Length.
    #[inline]
    pub fn length(self) -> f64 {
        self.delta().length()
    }

    /// True if degenerate (near-zero length).
    pub fn is_degenerate(self, tol: Tolerance) -> bool {
        tol.is_zero(self.length())
    }

    /// Point at parameter `t` where `t=0` is start and `t=1` is end.
    pub fn point_at(self, t: f64) -> Vec2 {
        self.start.lerp(self.end, t)
    }

    /// Closest point on the infinite line through the segment, as parameter `t`.
    pub fn closest_param(self, p: Vec2) -> f64 {
        let d = self.delta();
        let len2 = d.length_squared();
        if len2 == 0.0 {
            return 0.0;
        }
        (p - self.start).dot(d) / len2
    }

    /// Closest point on the finite segment.
    pub fn closest_point(self, p: Vec2) -> Vec2 {
        let t = self.closest_param(p).clamp(0.0, 1.0);
        self.point_at(t)
    }

    /// Squared distance from point to segment.
    pub fn distance_squared(self, p: Vec2) -> f64 {
        self.closest_point(p).distance_squared(p)
    }

    /// Axis-aligned bounding box.
    pub fn aabb(self) -> Aabb {
        Aabb::from_points(self.start, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midpoint() {
        let l = Line::new(Vec2::ZERO, Vec2::new(2.0, 4.0));
        assert!(l
            .point_at(0.5)
            .nearly_eq(Vec2::new(1.0, 2.0), Tolerance::DEFAULT));
    }

    #[test]
    fn closest_point_clamped() {
        let l = Line::new(Vec2::ZERO, Vec2::X);
        let c = l.closest_point(Vec2::new(2.0, 1.0));
        assert!(c.nearly_eq(Vec2::X, Tolerance::DEFAULT));
    }
}

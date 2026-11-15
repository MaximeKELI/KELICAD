//! Circles.

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;

/// Circle defined by center and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    /// Center.
    pub center: Vec2,
    /// Radius (must be ≥ 0 for a valid circle).
    pub radius: f64,
}

impl Circle {
    /// Create a circle.
    pub const fn new(center: Vec2, radius: f64) -> Self {
        Self { center, radius }
    }

    /// True if radius is nearly zero or negative.
    pub fn is_degenerate(self, tol: Tolerance) -> bool {
        self.radius < tol.abs
    }

    /// Point at angle `radians` from +X.
    pub fn point_at_angle(self, radians: f64) -> Vec2 {
        let (s, c) = radians.sin_cos();
        self.center + Vec2::new(c, s) * self.radius
    }

    /// Axis-aligned bounding box.
    pub fn aabb(self) -> Aabb {
        let r = Vec2::new(self.radius, self.radius);
        Aabb {
            min: self.center - r,
            max: self.center + r,
        }
    }

    /// Signed distance (negative inside).
    pub fn signed_distance(self, p: Vec2) -> f64 {
        p.distance(self.center) - self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_on_circle() {
        let c = Circle::new(Vec2::ZERO, 2.0);
        let p = c.point_at_angle(0.0);
        assert!(p.nearly_eq(Vec2::new(2.0, 0.0), Tolerance::DEFAULT));
    }
}

//! Ellipses (LibreCAD / AutoCAD-compatible parameters).

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;
use crate::polyline::Polyline;

/// Ellipse defined by center, major-axis endpoint relative to center, and axis ratio.
///
/// Matches the LibreCAD / DXF model: `major` is the vector from center to the
/// major-axis tip; `ratio = minor_radius / major_radius` (clamped to (0, 1]).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Ellipse {
    /// Center.
    pub center: Vec2,
    /// Major-axis tip relative to center (length = major radius).
    pub major: Vec2,
    /// Minor / major radius ratio.
    pub ratio: f64,
    /// Parametric start angle (radians). Full ellipse when start≈end≈0.
    pub start_angle: f64,
    /// Parametric end angle (radians).
    pub end_angle: f64,
}

impl Ellipse {
    /// Full ellipse from center, major tip (absolute), and ratio.
    pub fn full(center: Vec2, major_tip: Vec2, ratio: f64) -> Self {
        Self {
            center,
            major: major_tip - center,
            ratio: ratio.clamp(1e-12, 1.0),
            start_angle: 0.0,
            end_angle: 0.0,
        }
    }

    /// Axis-aligned ellipse (major along +X).
    pub fn axis_aligned(center: Vec2, rx: f64, ry: f64) -> Self {
        let (major_r, ratio) = if rx.abs() >= ry.abs() {
            (rx.abs(), (ry.abs() / rx.abs().max(1e-12)).min(1.0))
        } else {
            // Swap: major along Y.
            return Self {
                center,
                major: Vec2::new(0.0, ry.abs()),
                ratio: (rx.abs() / ry.abs().max(1e-12)).min(1.0),
                start_angle: 0.0,
                end_angle: 0.0,
            };
        };
        Self {
            center,
            major: Vec2::new(major_r, 0.0),
            ratio,
            start_angle: 0.0,
            end_angle: 0.0,
        }
    }

    /// Major radius.
    pub fn major_radius(self) -> f64 {
        self.major.length()
    }

    /// Minor radius.
    pub fn minor_radius(self) -> f64 {
        self.major_radius() * self.ratio
    }

    /// Rotation of the major axis from +X.
    pub fn rotation(self) -> f64 {
        self.major.angle()
    }

    /// True if nearly a point.
    pub fn is_degenerate(self, tol: Tolerance) -> bool {
        self.major_radius() < tol.abs || self.ratio < tol.abs
    }

    /// True if this is a full closed ellipse (not an elliptical arc).
    pub fn is_full(self, tol: Tolerance) -> bool {
        tol.is_zero(self.start_angle) && tol.is_zero(self.end_angle)
    }

    /// Point at parametric angle `t` (0 = major axis).
    pub fn point_at(self, t: f64) -> Vec2 {
        let (s, c) = t.sin_cos();
        let local = Vec2::new(c, s * self.ratio) * self.major_radius();
        let rot = self.rotation();
        self.center + local.rotate(rot)
    }

    /// Sample as a closed (or open arc) polyline.
    pub fn to_polyline(self, segments: usize) -> Polyline {
        let n = segments.max(8);
        let (a0, a1, closed) = if self.is_full(Tolerance::DEFAULT) {
            (0.0, std::f64::consts::TAU, true)
        } else {
            let a0 = self.start_angle;
            let mut a1 = self.end_angle;
            if a1 <= a0 {
                a1 += std::f64::consts::TAU;
            }
            (a0, a1, false)
        };
        let mut vertices = Vec::with_capacity(n + 1);
        for i in 0..=n {
            if !closed && i == n {
                break;
            }
            let t = a0 + (a1 - a0) * (i as f64) / (n as f64);
            vertices.push(self.point_at(t));
        }
        if !closed {
            vertices.push(self.point_at(a1));
        }
        Polyline { vertices, closed }
    }

    /// Axis-aligned bounding box (conservative for rotated ellipse).
    pub fn aabb(self) -> Aabb {
        // Sample densely enough for a tight AABB.
        let poly = self.to_polyline(64);
        let mut bb = Aabb::EMPTY;
        for p in &poly.vertices {
            bb = bb.expand(*p);
        }
        bb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_circle_as_ellipse() {
        let e = Ellipse::axis_aligned(Vec2::ZERO, 1.0, 1.0);
        assert!((e.major_radius() - 1.0).abs() < 1e-12);
        assert!((e.ratio - 1.0).abs() < 1e-12);
        let p = e.point_at(0.0);
        assert!(p.nearly_eq(Vec2::X, Tolerance::DEFAULT));
    }

    #[test]
    fn stretched_ellipse_extents() {
        let e = Ellipse::axis_aligned(Vec2::ZERO, 4.0, 2.0);
        let bb = e.aabb();
        assert!((bb.max.x - 4.0).abs() < 0.05);
        assert!((bb.max.y - 2.0).abs() < 0.05);
    }
}

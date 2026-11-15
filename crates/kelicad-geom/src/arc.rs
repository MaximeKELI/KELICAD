//! Circular arcs (CCW from `start_angle` to `end_angle` by default).

use std::f64::consts::TAU;

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;
use crate::circle::Circle;

/// Circular arc. Angles are in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    /// Center of the supporting circle.
    pub center: Vec2,
    /// Radius.
    pub radius: f64,
    /// Start angle from +X (radians).
    pub start_angle: f64,
    /// End angle from +X (radians).
    pub end_angle: f64,
    /// If true, sweep clockwise instead of CCW.
    pub clockwise: bool,
}

impl Arc {
    /// Create a CCW arc from `start_angle` to `end_angle`.
    ///
    /// If the angles are equal (within float equality), the arc has zero sweep.
    pub fn new(center: Vec2, radius: f64, start_angle: f64, end_angle: f64) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            clockwise: false,
        }
    }

    /// Supporting circle.
    pub fn circle(self) -> Circle {
        Circle::new(self.center, self.radius)
    }

    /// True if degenerate (near-zero radius or sweep).
    pub fn is_degenerate(self, tol: Tolerance) -> bool {
        self.radius < tol.abs || self.sweep_angle().abs() < tol.abs
    }

    /// Signed sweep in (−τ, τ]. Positive = CCW, negative = CW.
    /// Equal start/end angles yield a zero sweep (not a full circle).
    pub fn sweep_angle(self) -> f64 {
        if (self.end_angle - self.start_angle).abs() < f64::EPSILON {
            return 0.0;
        }
        let mut d = normalize_angle(self.end_angle - self.start_angle);
        if self.clockwise {
            if d > 0.0 {
                d -= TAU;
            }
        } else if d < 0.0 {
            d += TAU;
        }
        d
    }

    /// Start point.
    pub fn start_point(self) -> Vec2 {
        self.circle().point_at_angle(self.start_angle)
    }

    /// End point.
    pub fn end_point(self) -> Vec2 {
        self.circle().point_at_angle(self.end_angle)
    }

    /// Point at normalized parameter `t` in \[0, 1\] along the sweep.
    pub fn point_at(self, t: f64) -> Vec2 {
        let angle = self.start_angle + self.sweep_angle() * t;
        self.circle().point_at_angle(angle)
    }

    /// Conservative AABB from endpoints and axis extrema on the sweep.
    pub fn aabb(self) -> Aabb {
        let mut box_ = Aabb::EMPTY
            .expand(self.start_point())
            .expand(self.end_point());
        for &axis_angle in &[
            0.0,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        ] {
            if self.contains_angle(axis_angle, Tolerance::DEFAULT) {
                box_ = box_.expand(self.circle().point_at_angle(axis_angle));
            }
        }
        box_
    }

    /// True if `angle` lies on the arc sweep (inclusive within `tol`).
    pub fn contains_angle(self, angle: f64, tol: Tolerance) -> bool {
        let sweep = self.sweep_angle();
        if sweep.abs() < tol.abs {
            return (normalize_angle(angle - self.start_angle)).abs() <= tol.abs;
        }
        let mut a = normalize_angle(angle - self.start_angle);
        if self.clockwise {
            if a > 0.0 {
                a -= TAU;
            }
            a >= sweep - tol.abs && a <= tol.abs
        } else {
            if a < 0.0 {
                a += TAU;
            }
            a >= -tol.abs && a <= sweep + tol.abs
        }
    }
}

fn normalize_angle(a: f64) -> f64 {
    let mut x = a % TAU;
    if x <= -std::f64::consts::PI {
        x += TAU;
    } else if x > std::f64::consts::PI {
        x -= TAU;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_arc_endpoints() {
        let arc = Arc::new(Vec2::ZERO, 1.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!(arc.start_point().nearly_eq(Vec2::X, Tolerance::DEFAULT));
        assert!(arc.end_point().nearly_eq(Vec2::Y, Tolerance::DEFAULT));
    }

    #[test]
    fn quarter_arc_contains_mid() {
        let arc = Arc::new(Vec2::ZERO, 1.0, 0.0, std::f64::consts::FRAC_PI_2);
        assert!(arc.contains_angle(std::f64::consts::FRAC_PI_4, Tolerance::DEFAULT));
        assert!(!arc.contains_angle(std::f64::consts::PI, Tolerance::DEFAULT));
    }
}

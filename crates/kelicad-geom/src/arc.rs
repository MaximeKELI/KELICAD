//! Circular arcs (CCW from `start_angle` to `end_angle`).

use std::f64::consts::TAU;

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;
use crate::circle::Circle;

/// Circular arc. Angles are in radians; sweep is counter-clockwise from
/// `start_angle` toward `end_angle` unless `clockwise` is set.
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
    /// Create a CCW arc.
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

    /// True if degenerate.
    pub fn is_degenerate(self, tol: Tolerance) -> bool {
        self.radius < tol.abs || tol.is_zero(self.sweep_angle().abs())
    }

    /// Signed sweep angle in (−τ, τ], negative if clockwise.
    pub fn sweep_angle(self) -> f64 {
        let mut delta = normalize_angle(self.end_angle - self.start_angle);
        if self.clockwise {
            if delta > 0.0 {
                delta -= TAU;
            } else if delta == 0.0 {
                delta = -TAU;
            }
        } else if delta < 0.0 {
            delta += TAU;
        } else if delta == 0.0 {
            delta = TAU;
        }
        // Full circle when start == end is represented as ±τ; for zero-length
        // arcs callers should set equal angles with a flag — here equal angles
        // mean a full turn. For CAD short arcs, prefer distinct angles.
        if !self.clockwise && (self.end_angle - self.start_angle).abs() < f64::EPSILON {
            return 0.0;
        }
        if self.clockwise && (self.end_angle - self.start_angle).abs() < f64::EPSILON {
            return 0.0;
        }
        let mut d = normalize_angle(self.end_angle - self.start_angle);
        if self.clockwise {
            if d >= 0.0 {
                d -= TAU;
            }
        } else if d <= 0.0 {
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

    /// Conservative AABB (samples extrema and endpoints).
    pub fn aabb(self) -> Aabb {
        let mut box_ = Aabb::EMPTY
            .expand(self.start_point())
            .expand(self.end_point());
        // Include axis extrema if they lie on the arc.
        for &axis_angle in &[0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI, 3.0 * std::f64::consts::FRAC_PI_2]
        {
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
            return tol.eq(normalize_angle(angle), normalize_angle(self.start_angle));
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

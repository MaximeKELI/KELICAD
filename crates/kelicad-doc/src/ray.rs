//! Ray and construction line (XLINE) payloads.

use kelicad_math::Vec2;

/// Semi-infinite ray from `origin` along unit `direction`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RayGeom {
    /// Origin.
    pub origin: Vec2,
    /// Direction (stored normalized when constructed via helpers).
    pub direction: Vec2,
}

impl RayGeom {
    /// Create from origin and a point on the ray.
    pub fn from_points(origin: Vec2, through: Vec2) -> Option<Self> {
        let d = through - origin;
        let len = d.length();
        if len < 1e-12 {
            return None;
        }
        Some(Self {
            origin,
            direction: d / len,
        })
    }

    /// Point at distance `t` ≥ 0 along the ray.
    pub fn point_at(self, t: f64) -> Vec2 {
        self.origin + self.direction * t
    }
}

/// Infinite construction line through `point` along `direction`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct XLineGeom {
    /// Any point on the line.
    pub point: Vec2,
    /// Direction.
    pub direction: Vec2,
}

impl XLineGeom {
    /// Create from two distinct points.
    pub fn from_points(a: Vec2, b: Vec2) -> Option<Self> {
        let d = b - a;
        let len = d.length();
        if len < 1e-12 {
            return None;
        }
        Some(Self {
            point: a,
            direction: d / len,
        })
    }
}

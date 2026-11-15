//! 2D and 3D vectors.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use crate::tolerance::Tolerance;

/// Two-dimensional vector / point in document space.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
}

impl Vec2 {
    /// Origin.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Unit X.
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    /// Unit Y.
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    /// Create a vector.
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Squared length.
    #[inline]
    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean length.
    #[inline]
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Distance to another point.
    #[inline]
    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// Squared distance to another point.
    #[inline]
    pub fn distance_squared(self, other: Self) -> f64 {
        (self - other).length_squared()
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product magnitude (z-component of 3D cross).
    #[inline]
    pub fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Perpendicular vector rotated 90° counter-clockwise: `(-y, x)`.
    #[inline]
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }

    /// Normalize, or `None` if near-zero under `tol`.
    pub fn try_normalize(self, tol: Tolerance) -> Option<Self> {
        let len = self.length();
        if tol.is_zero(len) {
            None
        } else {
            Some(self / len)
        }
    }

    /// Linear interpolation.
    #[inline]
    pub fn lerp(self, other: Self, t: f64) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    /// Component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// True if both components are nearly equal under `tol`.
    pub fn nearly_eq(self, other: Self, tol: Tolerance) -> bool {
        tol.eq(self.x, other.x) && tol.eq(self.y, other.y)
    }

    /// Angle from +X in radians, range (−π, π].
    #[inline]
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// Rotate around origin by `radians`.
    pub fn rotate(self, radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl Mul<Vec2> for f64 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        rhs * self
    }
}

impl MulAssign<f64> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f64> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f64> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

/// Three-dimensional vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vec3 {
    /// Origin.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Create a vector.
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Lift a 2D vector to z = 0.
    #[inline]
    pub const fn from_vec2(v: Vec2) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: 0.0,
        }
    }

    /// Drop Z.
    #[inline]
    pub const fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Squared length.
    #[inline]
    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    /// Euclidean length.
    #[inline]
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Normalize, or `None` if near-zero under `tol`.
    pub fn try_normalize(self, tol: Tolerance) -> Option<Self> {
        let len = self.length();
        if tol.is_zero(len) {
            None
        } else {
            Some(self / len)
        }
    }

    /// True if nearly equal under `tol`.
    pub fn nearly_eq(self, other: Self, tol: Tolerance) -> bool {
        tol.eq(self.x, other.x) && tol.eq(self.y, other.y) && tol.eq(self.z, other.z)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn length_of_unit_x() {
        assert!((Vec2::X.length() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn perp_is_orthogonal() {
        let v = Vec2::new(3.0, 4.0);
        assert!(Tolerance::DEFAULT.is_zero(v.dot(v.perp())));
    }

    #[test]
    fn nearly_eq_respects_tolerance() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(1.0 + 1e-12, 2.0);
        assert!(a.nearly_eq(b, Tolerance::DEFAULT));
        assert!(!a.nearly_eq(b, Tolerance::new(1e-14)));
    }

    proptest! {
        #[test]
        fn normalize_unit_length(x in -1e6f64..1e6, y in -1e6f64..1e6) {
            let v = Vec2::new(x, y);
            if let Some(n) = v.try_normalize(Tolerance::DEFAULT) {
                prop_assert!((n.length() - 1.0).abs() < 1e-9);
            }
        }

        #[test]
        fn lerp_endpoints(x0 in -1e3f64..1e3, y0 in -1e3f64..1e3,
                          x1 in -1e3f64..1e3, y1 in -1e3f64..1e3) {
            let a = Vec2::new(x0, y0);
            let b = Vec2::new(x1, y1);
            prop_assert!(a.lerp(b, 0.0).nearly_eq(a, Tolerance::DEFAULT));
            prop_assert!(a.lerp(b, 1.0).nearly_eq(b, Tolerance::DEFAULT));
        }

        #[test]
        fn cross_anticommutative(x0 in -1e3f64..1e3, y0 in -1e3f64..1e3,
                                 x1 in -1e3f64..1e3, y1 in -1e3f64..1e3) {
            let a = Vec2::new(x0, y0);
            let b = Vec2::new(x1, y1);
            prop_assert!((a.cross(b) + b.cross(a)).abs() < 1e-9);
        }
    }
}

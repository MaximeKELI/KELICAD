//! 3×3 and 4×4 matrices (column-major, matching GPU conventions).

use crate::vec::{Vec2, Vec3};

/// 3×3 matrix in column-major order (`m[col * 3 + row]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    /// Column-major elements.
    pub m: [f64; 9],
}

impl Mat3 {
    /// Identity.
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    };

    /// Zero matrix.
    pub const ZERO: Self = Self { m: [0.0; 9] };

    /// Element at (row, col).
    #[inline]
    pub fn get(self, row: usize, col: usize) -> f64 {
        self.m[col * 3 + row]
    }

    /// Translation in the plane (homogeneous 2D).
    pub fn translation(tx: f64, ty: f64) -> Self {
        let mut m = Self::IDENTITY;
        m.m[6] = tx;
        m.m[7] = ty;
        m
    }

    /// Uniform scale about origin.
    pub fn scale(s: f64) -> Self {
        Self {
            m: [s, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Non-uniform scale.
    pub fn scale_nonuniform(sx: f64, sy: f64) -> Self {
        Self {
            m: [sx, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Rotation about origin by `radians`.
    pub fn rotation(radians: f64) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            m: [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Matrix–matrix product `self * other`.
    pub fn mul_mat(self, other: Self) -> Self {
        let mut out = Self::ZERO;
        for col in 0..3 {
            for row in 0..3 {
                let mut sum = 0.0;
                for k in 0..3 {
                    sum += self.get(row, k) * other.get(k, col);
                }
                out.m[col * 3 + row] = sum;
            }
        }
        out
    }

    /// Transform a 2D point (w = 1).
    pub fn transform_point(self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.m[0] * p.x + self.m[3] * p.y + self.m[6],
            self.m[1] * p.x + self.m[4] * p.y + self.m[7],
        )
    }

    /// Transform a 2D direction (w = 0).
    pub fn transform_vector(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.m[0] * v.x + self.m[3] * v.y,
            self.m[1] * v.x + self.m[4] * v.y,
        )
    }

    /// Determinant.
    pub fn determinant(self) -> f64 {
        self.m[0] * (self.m[4] * self.m[8] - self.m[7] * self.m[5])
            - self.m[3] * (self.m[1] * self.m[8] - self.m[7] * self.m[2])
            + self.m[6] * (self.m[1] * self.m[5] - self.m[4] * self.m[2])
    }

    /// Inverse, or `None` if singular.
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < f64::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        let m = &self.m;
        Some(Self {
            m: [
                (m[4] * m[8] - m[7] * m[5]) * inv_det,
                (m[2] * m[7] - m[1] * m[8]) * inv_det,
                (m[1] * m[5] - m[2] * m[4]) * inv_det,
                (m[5] * m[6] - m[3] * m[8]) * inv_det,
                (m[0] * m[8] - m[2] * m[6]) * inv_det,
                (m[2] * m[3] - m[0] * m[5]) * inv_det,
                (m[3] * m[7] - m[4] * m[6]) * inv_det,
                (m[1] * m[6] - m[0] * m[7]) * inv_det,
                (m[0] * m[4] - m[1] * m[3]) * inv_det,
            ],
        })
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// 4×4 matrix in column-major order (`m[col * 4 + row]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat4 {
    /// Column-major elements.
    pub m: [f64; 16],
}

impl Mat4 {
    /// Identity.
    pub const IDENTITY: Self = Self {
        m: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    /// Zero.
    pub const ZERO: Self = Self { m: [0.0; 16] };

    /// Element at (row, col).
    #[inline]
    pub fn get(self, row: usize, col: usize) -> f64 {
        self.m[col * 4 + row]
    }

    /// Translation.
    pub fn translation(t: Vec3) -> Self {
        let mut m = Self::IDENTITY;
        m.m[12] = t.x;
        m.m[13] = t.y;
        m.m[14] = t.z;
        m
    }

    /// Uniform scale.
    pub fn scale(s: f64) -> Self {
        Self {
            m: [
                s, 0.0, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 0.0, s, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// Orthographic projection (OpenGL-style NDC).
    pub fn orthographic(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        let mut m = Self::ZERO;
        m.m[0] = 2.0 / (right - left);
        m.m[5] = 2.0 / (top - bottom);
        m.m[10] = -2.0 / (far - near);
        m.m[12] = -(right + left) / (right - left);
        m.m[13] = -(top + bottom) / (top - bottom);
        m.m[14] = -(far + near) / (far - near);
        m.m[15] = 1.0;
        m
    }

    /// Matrix–matrix product.
    pub fn mul_mat(self, other: Self) -> Self {
        let mut out = Self::ZERO;
        for col in 0..4 {
            for row in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += self.get(row, k) * other.get(k, col);
                }
                out.m[col * 4 + row] = sum;
            }
        }
        out
    }

    /// Transform a point (w = 1).
    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let x = self.m[0] * p.x + self.m[4] * p.y + self.m[8] * p.z + self.m[12];
        let y = self.m[1] * p.x + self.m[5] * p.y + self.m[9] * p.z + self.m[13];
        let z = self.m[2] * p.x + self.m[6] * p.y + self.m[10] * p.z + self.m[14];
        let w = self.m[3] * p.x + self.m[7] * p.y + self.m[11] * p.z + self.m[15];
        if w.abs() > f64::EPSILON {
            Vec3::new(x / w, y / w, z / w)
        } else {
            Vec3::new(x, y, z)
        }
    }

    /// As `f32` array for GPU upload.
    pub fn to_f32_array(self) -> [f32; 16] {
        let mut out = [0.0f32; 16];
        for (i, v) in self.m.iter().enumerate() {
            out[i] = *v as f32;
        }
        out
    }
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tolerance::Tolerance;
    use crate::vec::Vec2;

    #[test]
    fn translation_moves_point() {
        let m = Mat3::translation(2.0, 3.0);
        let p = m.transform_point(Vec2::new(1.0, 1.0));
        assert!(p.nearly_eq(Vec2::new(3.0, 4.0), Tolerance::DEFAULT));
    }

    #[test]
    fn rotation_90_maps_x_to_y() {
        let m = Mat3::rotation(std::f64::consts::FRAC_PI_2);
        let p = m.transform_point(Vec2::X);
        assert!(p.nearly_eq(Vec2::Y, Tolerance::DEFAULT));
    }

    #[test]
    fn inverse_roundtrip() {
        let m = Mat3::translation(5.0, -2.0).mul_mat(Mat3::rotation(0.3)).mul_mat(Mat3::scale(2.0));
        let inv = m.inverse().expect("invertible");
        let p = Vec2::new(1.5, -4.0);
        let back = inv.transform_point(m.transform_point(p));
        assert!(back.nearly_eq(p, Tolerance::DEFAULT));
    }

    #[test]
    fn mat4_ortho_center_maps_near_origin() {
        let m = Mat4::orthographic(-10.0, 10.0, -5.0, 5.0, -1.0, 1.0);
        let p = m.transform_point(Vec3::ZERO);
        assert!(p.nearly_eq(Vec3::ZERO, Tolerance::DEFAULT));
    }
}

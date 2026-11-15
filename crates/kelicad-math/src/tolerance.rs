//! Explicit floating-point tolerances (never use raw `==` on geometry floats).

/// Absolute / relative tolerance pair for geometric comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Absolute epsilon.
    pub abs: f64,
    /// Relative epsilon scaled by magnitude.
    pub rel: f64,
}

impl Tolerance {
    /// Default document tolerance (~1e-9 absolute).
    pub const DEFAULT: Self = Self {
        abs: 1e-9,
        rel: 1e-12,
    };

    /// Coarser tolerance for screen-space / snap heuristics.
    pub const COARSE: Self = Self {
        abs: 1e-6,
        rel: 1e-9,
    };

    /// Create an absolute-only tolerance (`rel = 0`).
    pub const fn new(abs: f64) -> Self {
        Self { abs, rel: 0.0 }
    }

    /// Combined absolute + relative tolerance.
    pub const fn with_relative(abs: f64, rel: f64) -> Self {
        Self { abs, rel }
    }

    /// Effective epsilon for values around `scale`.
    #[inline]
    pub fn epsilon(self, scale: f64) -> f64 {
        self.abs.max(self.rel * scale.abs())
    }

    /// True if `a` and `b` are equal within this tolerance.
    #[inline]
    pub fn eq(self, a: f64, b: f64) -> bool {
        let diff = (a - b).abs();
        diff <= self.epsilon(a.abs().max(b.abs()))
    }

    /// True if `v` is nearly zero.
    #[inline]
    pub fn is_zero(self, v: f64) -> bool {
        v.abs() <= self.abs
    }

    /// Compare `a` and `b`; returns −1 / 0 / 1 with tolerance.
    pub fn cmp(self, a: f64, b: f64) -> i8 {
        if self.eq(a, b) {
            0
        } else if a < b {
            -1
        } else {
            1
        }
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_within_abs() {
        let t = Tolerance::new(1e-6);
        assert!(t.eq(1.0, 1.0 + 5e-7));
        assert!(!t.eq(1.0, 1.0 + 2e-6));
    }

    #[test]
    fn relative_scales() {
        let t = Tolerance::with_relative(0.0, 1e-6);
        assert!(t.eq(1_000_000.0, 1_000_000.5));
        assert!(!t.eq(1.0, 1.001));
    }
}

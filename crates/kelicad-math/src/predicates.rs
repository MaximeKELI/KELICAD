//! Robust geometric predicates (orientation, incircle).
//!
//! Uses adaptive absolute-error filtering inspired by Shewchuk; exact
//! arithmetic fallback is deferred to a later milestone if needed.

use crate::tolerance::Tolerance;
use crate::vec::Vec2;

/// Result of an orientation test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orient {
    /// Clockwise.
    Clockwise,
    /// Counter-clockwise.
    CounterClockwise,
    /// Collinear / cocircular within tolerance.
    Collinear,
}

impl Orient {
    /// Sign as −1 / 0 / +1 (CW / collinear / CCW).
    pub fn sign(self) -> i8 {
        match self {
            Self::Clockwise => -1,
            Self::Collinear => 0,
            Self::CounterClockwise => 1,
        }
    }
}

/// Orient of triangle `a → b → c` (left-turn is CCW).
///
/// Computes twice the signed area of triangle abc.
pub fn orient2d(a: Vec2, b: Vec2, c: Vec2, tol: Tolerance) -> Orient {
    let det = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    // Scale error bound by edge lengths.
    let scale = (b - a).length().max((c - a).length()).max(1.0);
    let eps = tol.epsilon(scale * scale);
    if det > eps {
        Orient::CounterClockwise
    } else if det < -eps {
        Orient::Clockwise
    } else {
        Orient::Collinear
    }
}

/// Incircle test: is `d` inside the circle through `a`, `b`, `c`?
///
/// Assumes `a`, `b`, `c` are in counter-clockwise order. Returns positive if
/// inside, negative if outside, zero if cocircular (within tolerance).
pub fn incircle(a: Vec2, b: Vec2, c: Vec2, d: Vec2, tol: Tolerance) -> f64 {
    // Determinant form of the incircle predicate.
    let adx = a.x - d.x;
    let ady = a.y - d.y;
    let bdx = b.x - d.x;
    let bdy = b.y - d.y;
    let cdx = c.x - d.x;
    let cdy = c.y - d.y;

    let abdet = adx * bdy - bdx * ady;
    let bcdet = bdx * cdy - cdx * bdy;
    let cadet = cdx * ady - adx * cdy;
    let alift = adx * adx + ady * ady;
    let blift = bdx * bdx + bdy * bdy;
    let clift = cdx * cdx + cdy * cdy;

    let det = alift * bcdet + blift * cadet + clift * abdet;
    let scale = alift.max(blift).max(clift).max(1.0);
    let eps = tol.epsilon(scale * scale);
    if det.abs() <= eps {
        0.0
    } else {
        det
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn ccw_unit_square() {
        let o = orient2d(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            Tolerance::DEFAULT,
        );
        assert_eq!(o, Orient::CounterClockwise);
    }

    #[test]
    fn cw_unit_square() {
        let o = orient2d(
            Vec2::ZERO,
            Vec2::Y,
            Vec2::X,
            Tolerance::DEFAULT,
        );
        assert_eq!(o, Orient::Clockwise);
    }

    #[test]
    fn collinear_points() {
        let o = orient2d(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            Tolerance::DEFAULT,
        );
        assert_eq!(o, Orient::Collinear);
    }

    #[test]
    fn point_inside_unit_circle_triangle() {
        // Equilateral-ish CCW triangle around origin-ish
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(-0.5, 0.866_025_403_78);
        let c = Vec2::new(-0.5, -0.866_025_403_78);
        let d = Vec2::ZERO;
        assert!(incircle(a, b, c, d, Tolerance::DEFAULT) > 0.0);
    }

    #[test]
    fn point_outside_triangle_circle() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(1.0, 0.0);
        let c = Vec2::new(0.0, 1.0);
        let d = Vec2::new(2.0, 2.0);
        assert!(incircle(a, b, c, d, Tolerance::DEFAULT) < 0.0);
    }

    proptest! {
        #[test]
        fn orient_negates_on_swap(
            ax in -1e3f64..1e3, ay in -1e3f64..1e3,
            bx in -1e3f64..1e3, by in -1e3f64..1e3,
            cx in -1e3f64..1e3, cy in -1e3f64..1e3,
        ) {
            let a = Vec2::new(ax, ay);
            let b = Vec2::new(bx, by);
            let c = Vec2::new(cx, cy);
            let o1 = orient2d(a, b, c, Tolerance::DEFAULT);
            let o2 = orient2d(a, c, b, Tolerance::DEFAULT);
            prop_assert_eq!(o1.sign(), -o2.sign());
        }
    }
}

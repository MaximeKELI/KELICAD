//! Cubic B-spline / fit-point spline (DXF-friendly subset).

use kelicad_math::{Tolerance, Vec2};

use crate::polyline::Polyline;

/// Open cubic B-spline with control points (clamped ends when degree=3).
///
/// Inspired by LibreCAD / AutoCAD SPLINE entity model, reimplemented under MIT.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Spline {
    /// Control points.
    pub control_points: Vec<Vec2>,
    /// Degree (1..=3 supported for sampling).
    pub degree: u8,
    /// Closed (periodic) flag.
    pub closed: bool,
    /// Optional fit points (for display / interpolation mode).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fit_points: Vec<Vec2>,
}

impl Spline {
    /// From control points.
    pub fn from_controls(control_points: Vec<Vec2>, degree: u8, closed: bool) -> Option<Self> {
        let deg = degree.clamp(1, 3);
        if control_points.len() < (deg as usize) + 1 {
            return None;
        }
        Some(Self {
            control_points,
            degree: deg,
            closed,
            fit_points: vec![],
        })
    }

    /// Quadratic/cubic through fit points via Catmull–Rom → Bezier sampling.
    pub fn from_fit_points(fit_points: Vec<Vec2>, closed: bool) -> Option<Self> {
        if fit_points.len() < 2 {
            return None;
        }
        // Store fit points; derive control polygon as the fit polyline for degree 1 fallback,
        // and use Catmull–Rom sampling in `to_polyline`.
        Some(Self {
            control_points: fit_points.clone(),
            degree: 3,
            closed,
            fit_points,
        })
    }

    /// True if too few points.
    pub fn is_degenerate(&self, _tol: Tolerance) -> bool {
        self.control_points.len() < 2
    }

    /// Sample to a polyline.
    pub fn to_polyline(&self, segments_per_span: usize) -> Polyline {
        let segs = segments_per_span.max(4);
        if !self.fit_points.is_empty() && self.fit_points.len() >= 2 {
            return sample_catmull_rom(&self.fit_points, self.closed, segs);
        }
        sample_bspline(&self.control_points, self.degree as usize, self.closed, segs)
    }
}

fn sample_catmull_rom(pts: &[Vec2], closed: bool, segs: usize) -> Polyline {
    let n = pts.len();
    if n == 1 {
        return Polyline {
            vertices: pts.to_vec(),
            closed: false,
        };
    }
    let mut vertices = Vec::new();
    let count = if closed { n } else { n - 1 };
    for i in 0..count {
        let p0 = pts[if i == 0 {
            if closed {
                n - 1
            } else {
                0
            }
        } else {
            i - 1
        }];
        let p1 = pts[i];
        let p2 = pts[(i + 1) % n];
        let p3 = pts[if closed {
            (i + 2) % n
        } else {
            (i + 2).min(n - 1)
        }];
        for s in 0..segs {
            let t = s as f64 / segs as f64;
            vertices.push(catmull(p0, p1, p2, p3, t));
        }
    }
    if !closed {
        vertices.push(*pts.last().unwrap());
    }
    Polyline {
        vertices,
        closed,
    }
}

fn catmull(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f64) -> Vec2 {
    let t2 = t * t;
    let t3 = t2 * t;
    p0 * (-0.5 * t3 + t2 - 0.5 * t)
        + p1 * (1.5 * t3 - 2.5 * t2 + 1.0)
        + p2 * (-1.5 * t3 + 2.0 * t2 + 0.5 * t)
        + p3 * (0.5 * t3 - 0.5 * t2)
}

fn sample_bspline(ctrl: &[Vec2], degree: usize, closed: bool, segs: usize) -> Polyline {
    if ctrl.len() < 2 {
        return Polyline {
            vertices: ctrl.to_vec(),
            closed: false,
        };
    }
    if degree <= 1 {
        return Polyline {
            vertices: ctrl.to_vec(),
            closed,
        };
    }
    // Open uniform knot vector, de Boor evaluation.
    let n = ctrl.len();
    let p = degree.min(n - 1);
    let m = n + p + 1;
    let mut knots = vec![0.0; m];
    for (i, k) in knots.iter_mut().enumerate() {
        *k = if i <= p {
            0.0
        } else if i >= n {
            (n - p) as f64
        } else {
            (i - p) as f64
        };
    }
    let u0 = knots[p];
    let u1 = knots[n];
    let steps = ((n - p) * segs).max(segs);
    let mut vertices = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let u = u0 + (u1 - u0) * (i as f64) / (steps as f64);
        // Clamp interior to avoid endpoint knot multiplicity issues.
        let u = u.clamp(u0, u1 - 1e-12);
        vertices.push(de_boor(ctrl, &knots, p, u));
    }
    // Force exact end.
    if let Some(last) = ctrl.last() {
        if let Some(v) = vertices.last_mut() {
            *v = *last;
        }
    }
    Polyline {
        vertices,
        closed,
    }
}

fn de_boor(ctrl: &[Vec2], knots: &[f64], p: usize, u: f64) -> Vec2 {
    let n = ctrl.len();
    // Find span
    let mut k = p;
    while k < n && !(knots[k] <= u && u < knots[k + 1]) {
        k += 1;
    }
    k = k.min(n - 1);
    let mut d: Vec<Vec2> = (0..=p)
        .map(|j| {
            let idx = k + j - p;
            ctrl[idx.min(n - 1)]
        })
        .collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = k - p + j;
            let denom = knots[i + p - r + 1] - knots[i];
            let alpha = if denom.abs() < 1e-15 {
                0.0
            } else {
                (u - knots[i]) / denom
            };
            d[j] = d[j - 1] * (1.0 - alpha) + d[j] * alpha;
        }
    }
    d[p]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_spline_samples_endpoints() {
        let s = Spline::from_fit_points(
            vec![Vec2::ZERO, Vec2::new(1.0, 1.0), Vec2::new(2.0, 0.0)],
            false,
        )
        .unwrap();
        let poly = s.to_polyline(8);
        assert!(poly.vertices.first().unwrap().nearly_eq(Vec2::ZERO, Tolerance::DEFAULT));
        assert!(poly
            .vertices
            .last()
            .unwrap()
            .nearly_eq(Vec2::new(2.0, 0.0), Tolerance::COARSE));
    }

    #[test]
    fn control_spline_nonempty() {
        let s = Spline::from_controls(
            vec![
                Vec2::ZERO,
                Vec2::new(1.0, 2.0),
                Vec2::new(2.0, 2.0),
                Vec2::new(3.0, 0.0),
            ],
            3,
            false,
        )
        .unwrap();
        let poly = s.to_polyline(6);
        assert!(poly.vertices.len() > 4);
    }
}

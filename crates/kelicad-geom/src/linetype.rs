//! Tessellate stroked segments according to a dash pattern.

use kelicad_math::Vec2;

use crate::line::Line;

/// Expand a solid segment into dashed sub-segments.
///
/// `pattern`: positive = draw, negative = gap, zero = dot (~2% of scale).
/// `scale` multiplies pattern lengths.
pub fn dash_line(seg: Line, pattern: &[f64], scale: f64) -> Vec<Line> {
    if pattern.is_empty() || seg.length() < 1e-12 {
        return vec![seg];
    }
    let scale = scale.abs().max(1e-12);
    let len = seg.length();
    let dir = seg.delta() / len;
    let mut t = 0.0;
    let mut out = Vec::new();
    let mut di = 0usize;
    while t < len - 1e-12 {
        let raw = pattern[di % pattern.len()];
        let mut dash = raw.abs() * scale;
        if raw.abs() < 1e-15 {
            // Dot: tiny dash.
            dash = (0.02 * scale).max(1e-6);
        }
        let t1 = (t + dash).min(len);
        if raw >= 0.0 {
            out.push(Line::new(seg.start + dir * t, seg.start + dir * t1));
        }
        t = t1;
        di += 1;
        if raw.abs() < 1e-15 {
            // After a dot, consume the following gap if present is already next pattern entry.
        }
    }
    out
}

/// Dash every segment of a polyline-like chain.
pub fn dash_polyline(vertices: &[Vec2], closed: bool, pattern: &[f64], scale: f64) -> Vec<Line> {
    if vertices.len() < 2 {
        return vec![];
    }
    let mut out = Vec::new();
    for w in vertices.windows(2) {
        out.extend(dash_line(Line::new(w[0], w[1]), pattern, scale));
    }
    if closed && vertices.len() >= 3 {
        if let (Some(&a), Some(&b)) = (vertices.last(), vertices.first()) {
            out.extend(dash_line(Line::new(a, b), pattern, scale));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_math::Vec2;

    #[test]
    fn continuous_unchanged() {
        let l = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        assert_eq!(dash_line(l, &[], 1.0).len(), 1);
    }

    #[test]
    fn dashed_produces_multiple() {
        let l = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        let segs = dash_line(l, &[1.0, -1.0], 1.0);
        assert!(segs.len() >= 5);
        assert!(segs.iter().all(|s| s.start.y.abs() < 1e-12));
    }
}

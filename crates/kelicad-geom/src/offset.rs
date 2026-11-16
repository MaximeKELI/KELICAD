//! Parallel offset of lines and polylines (miter joins).

use kelicad_math::{Tolerance, Vec2};

use crate::line::Line;
use crate::polyline::Polyline;
use crate::transform::offset_line;

/// Offset a polyline by `distance` (positive = left of travel direction).
///
/// Uses miter joins at vertices. Open polylines keep open ends; closed ones wrap.
pub fn offset_polyline(poly: &Polyline, distance: f64, tol: Tolerance) -> Option<Polyline> {
    if !distance.is_finite() || distance.abs() < tol.abs {
        return None;
    }
    let n = poly.vertices.len();
    if n < 2 {
        return None;
    }
    let segs = poly.segment_count();
    if segs == 0 {
        return None;
    }

    // Offset each supporting edge as infinite-direction segments.
    let mut raw_edges: Vec<Line> = Vec::with_capacity(segs);
    for i in 0..segs {
        let a = poly.vertices[i];
        let b = if i + 1 < n {
            poly.vertices[i + 1]
        } else {
            poly.vertices[0]
        };
        let edge = Line::new(a, b);
        if edge.is_degenerate(tol) {
            continue;
        }
        raw_edges.push(offset_line(edge, distance)?);
    }
    if raw_edges.len() < 2 && !poly.closed {
        // Single edge open poly → just the offset segment.
        return raw_edges.first().map(|l| Polyline::new(vec![l.start, l.end]));
    }
    if raw_edges.is_empty() {
        return None;
    }

    let mut verts = Vec::new();
    if poly.closed {
        let m = raw_edges.len();
        for i in 0..m {
            let prev = raw_edges[(i + m - 1) % m];
            let cur = raw_edges[i];
            verts.push(miter_or_fallback(prev, cur, tol));
        }
        if verts.len() < 3 {
            return None;
        }
        Some(Polyline::closed(verts))
    } else {
        // Open: first start, then miters, then last end.
        verts.push(raw_edges[0].start);
        for i in 0..raw_edges.len().saturating_sub(1) {
            verts.push(miter_or_fallback(raw_edges[i], raw_edges[i + 1], tol));
        }
        if let Some(last) = raw_edges.last() {
            verts.push(last.end);
        }
        if verts.len() < 2 {
            return None;
        }
        Some(Polyline::new(verts))
    }
}

fn miter_or_fallback(a: Line, b: Line, tol: Tolerance) -> Vec2 {
    let da = a.delta();
    let db = b.delta();
    let denom = da.cross(db);
    if tol.is_zero(denom) {
        // Parallel — use midpoint of a.end / b.start.
        return a.end.lerp(b.start, 0.5);
    }
    let t = (b.start - a.start).cross(db) / denom;
    a.start + da * t
}

/// Offset every segment of a polyline independently (no join) — useful for dashed preview.
pub fn offset_polyline_segments(poly: &Polyline, distance: f64) -> Vec<Line> {
    poly.segments()
        .filter_map(|s| offset_line(s, distance))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_rectangle_inward_and_outward() {
        let rect = Polyline::rectangle(Vec2::ZERO, Vec2::new(10.0, 6.0));
        // CCW: positive distance = left = interior → shrinks.
        let inner = offset_polyline(&rect, 1.0, Tolerance::DEFAULT).unwrap();
        assert!(inner.closed);
        let bb_in = inner.aabb();
        assert!(bb_in.max.x - bb_in.min.x < 10.0);
        // Negative = exterior → grows.
        let outer = offset_polyline(&rect, -1.0, Tolerance::DEFAULT).unwrap();
        let bb_out = outer.aabb();
        assert!(bb_out.max.x - bb_out.min.x > 10.0);
        assert!(bb_out.max.y - bb_out.min.y > 6.0);
    }

    #[test]
    fn offset_open_line() {
        let p = Polyline::new(vec![Vec2::ZERO, Vec2::new(10.0, 0.0)]);
        let off = offset_polyline(&p, 2.0, Tolerance::DEFAULT).unwrap();
        assert_eq!(off.vertices.len(), 2);
        assert!((off.vertices[0].y - 2.0).abs() < 1e-6);
    }
}

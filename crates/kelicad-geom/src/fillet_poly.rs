//! Fillet / chamfer all corners of a polyline (tessellated arcs).

use kelicad_math::{Tolerance, Vec2};

use crate::construct::{chamfer_lines, fillet_lines};
use crate::line::Line;
use crate::polyline::Polyline;

/// Fillet every interior corner of a closed (or open) polyline with radius `r`.
///
/// Arcs are sampled into the resulting polyline (`arc_segments` ≥ 4).
pub fn fillet_polyline(
    poly: &Polyline,
    r: f64,
    arc_segments: usize,
    tol: Tolerance,
) -> Option<Polyline> {
    if r <= tol.abs || poly.vertices.len() < 3 {
        return None;
    }
    let segs = arc_segments.max(4);
    let corners = corner_pairs(poly)?;
    let mut out = Vec::new();
    for (i, (prev, next)) in corners.iter().enumerate() {
        if let Some((ta, _tb, arc)) = fillet_lines(*prev, *next, r, tol) {
            // Emit trimmed prev end → arc samples → (next start handled next iter)
            if i == 0 && !poly.closed {
                out.push(ta.start);
            }
            // Point entering the fillet on prev edge:
            out.push(ta.end);
            for s in 1..segs {
                let t = s as f64 / segs as f64;
                out.push(arc.point_at(t));
            }
            // Last arc point ≈ start of trimmed next; will be refined by next corner.
        } else {
            // Keep original vertex.
            out.push(poly.vertices[corner_vertex_index(poly, i)]);
        }
    }
    if !poly.closed {
        if let Some(last) = poly.vertices.last() {
            out.push(*last);
        }
    }
    dedup_verts(&mut out, tol);
    if out.len() < 2 {
        return None;
    }
    Some(if poly.closed {
        Polyline::closed(out)
    } else {
        Polyline::new(out)
    })
}

/// Chamfer every corner of a polyline by distance `d` along each edge.
pub fn chamfer_polyline(poly: &Polyline, d: f64, tol: Tolerance) -> Option<Polyline> {
    if d <= tol.abs || poly.vertices.len() < 3 {
        return None;
    }
    let corners = corner_pairs(poly)?;
    let mut out = Vec::new();
    for (i, (prev, next)) in corners.iter().enumerate() {
        if let Some((ta, tb, ch)) = chamfer_lines(*prev, *next, d, d, tol) {
            if i == 0 && !poly.closed {
                out.push(ta.start);
            }
            out.push(ch.start);
            out.push(ch.end);
            let _ = tb;
        } else {
            out.push(poly.vertices[corner_vertex_index(poly, i)]);
        }
    }
    if !poly.closed {
        if let Some(last) = poly.vertices.last() {
            out.push(*last);
        }
    }
    dedup_verts(&mut out, tol);
    if out.len() < 2 {
        return None;
    }
    Some(if poly.closed {
        Polyline::closed(out)
    } else {
        Polyline::new(out)
    })
}

fn corner_vertex_index(poly: &Polyline, corner_i: usize) -> usize {
    if poly.closed {
        corner_i % poly.vertices.len()
    } else {
        // Open: corners are at vertices 1..n-2 (interior); we iterate all verts as hubs.
        corner_i.min(poly.vertices.len().saturating_sub(1))
    }
}

/// For each vertex that is a corner hub, return (incoming edge, outgoing edge).
fn corner_pairs(poly: &Polyline) -> Option<Vec<(Line, Line)>> {
    let n = poly.vertices.len();
    if n < 3 {
        return None;
    }
    let mut pairs = Vec::new();
    if poly.closed {
        for i in 0..n {
            let prev_i = (i + n - 1) % n;
            let next_i = (i + 1) % n;
            let incoming = Line::new(poly.vertices[prev_i], poly.vertices[i]);
            let outgoing = Line::new(poly.vertices[i], poly.vertices[next_i]);
            pairs.push((incoming, outgoing));
        }
    } else {
        // Interior vertices only.
        for i in 1..n - 1 {
            let incoming = Line::new(poly.vertices[i - 1], poly.vertices[i]);
            let outgoing = Line::new(poly.vertices[i], poly.vertices[i + 1]);
            pairs.push((incoming, outgoing));
        }
    }
    Some(pairs)
}

fn dedup_verts(v: &mut Vec<Vec2>, tol: Tolerance) {
    if v.is_empty() {
        return;
    }
    let mut out = vec![v[0]];
    for &p in v.iter().skip(1) {
        if !p.nearly_eq(*out.last().unwrap(), tol) {
            out.push(p);
        }
    }
    *v = out;
}

/// Parallel line through a point (same direction as `source`).
pub fn parallel_through(source: Line, through: Vec2, tol: Tolerance) -> Option<Line> {
    let dir = source.delta().try_normalize(tol)?;
    let len = source.length().max(tol.abs * 10.0);
    Some(Line::new(through - dir * (len * 0.5), through + dir * (len * 0.5)))
}

/// Midline between two points (segment).
pub fn midline(a: Vec2, b: Vec2, tol: Tolerance) -> Option<Line> {
    if a.nearly_eq(b, tol) {
        return None;
    }
    let mid = a.lerp(b, 0.5);
    let dir = (b - a).perp().try_normalize(tol)?;
    let half = a.distance(b) * 0.5;
    Some(Line::new(mid - dir * half, mid + dir * half))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fillet_square_more_vertices() {
        let sq = Polyline::rectangle(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let f = fillet_polyline(&sq, 1.0, 6, Tolerance::DEFAULT).unwrap();
        assert!(f.closed);
        assert!(f.vertices.len() > 4);
    }

    #[test]
    fn chamfer_square() {
        let sq = Polyline::rectangle(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let c = chamfer_polyline(&sq, 1.0, Tolerance::DEFAULT).unwrap();
        assert!(c.closed);
        assert_eq!(c.vertices.len(), 8);
    }

    #[test]
    fn parallel_through_point() {
        let src = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        let p = parallel_through(src, Vec2::new(0.0, 3.0), Tolerance::DEFAULT).unwrap();
        assert!((p.start.y - 3.0).abs() < 1e-6);
    }
}

//! Trim / extend helpers for finite segments, polylines, and arcs.

use kelicad_math::{Tolerance, Vec2};

use crate::arc::Arc;
use crate::circle::Circle;
use crate::intersect::{line_circle, line_line, LineCircleResult, LineLineResult};
use crate::line::Line;
use crate::polyline::Polyline;

/// Trim `target` using a cutting segment: keep the portion on the side of
/// `keep_point` relative to the intersection (or unchanged if no hit).
pub fn trim_line_by_cut(target: Line, cut: Line, keep_point: Vec2, tol: Tolerance) -> Option<Line> {
    let hit = match line_line(target, cut, tol) {
        LineLineResult::Point(p) => p,
        _ => return None,
    };
    let d_keep_start = keep_point.distance_squared(target.start);
    let d_keep_end = keep_point.distance_squared(target.end);
    // Keep the endpoint closer to keep_point; replace the other with hit.
    if d_keep_start <= d_keep_end {
        Some(Line::new(target.start, hit))
    } else {
        Some(Line::new(hit, target.end))
    }
}

/// Extend `target` to meet the infinite line through `boundary` if possible.
///
/// Extends the endpoint closer to `pick` (or both ends projected). Returns
/// `None` if parallel / no intersection ahead.
pub fn extend_line_to(target: Line, boundary: Line, pick: Vec2, tol: Tolerance) -> Option<Line> {
    let r = target.delta();
    let s = boundary.delta();
    let denom = r.cross(s);
    if tol.is_zero(denom) {
        return None;
    }
    let qp = boundary.start - target.start;
    let t = qp.cross(s) / denom;
    let hit = target.start + r * t;

    // Decide which endpoint to move: the one closer to pick, and only if
    // the hit lies beyond that endpoint along the line.
    let t_pick = target.closest_param(pick);
    if t_pick < 0.5 {
        // extend/move start
        if t <= tol.abs {
            Some(Line::new(hit, target.end))
        } else if t >= 1.0 - tol.abs {
            // hit is beyond end — extend end instead
            Some(Line::new(target.start, hit))
        } else {
            // intersection on segment — treat as trim-like: snap start
            Some(Line::new(hit, target.end))
        }
    } else if t >= 1.0 - tol.abs {
        Some(Line::new(target.start, hit))
    } else if t <= tol.abs {
        Some(Line::new(hit, target.end))
    } else {
        Some(Line::new(target.start, hit))
    }
}

/// Trim the polyline segment nearest `pick` using cutter `cut`.
///
/// Replaces that segment in-place; returns a new polyline.
pub fn trim_polyline_by_cut(
    poly: &Polyline,
    cut: Line,
    pick: Vec2,
    tol: Tolerance,
) -> Option<Polyline> {
    let mut best: Option<(usize, f64)> = None;
    for (i, seg) in poly.segments().enumerate() {
        let d = seg.distance_squared(pick);
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    let (idx, _) = best?;
    let seg = poly.segment(idx)?;
    let trimmed = trim_line_by_cut(seg, cut, pick, tol)?;
    let mut verts = poly.vertices.clone();
    if poly.closed {
        // Closed: replace endpoints of segment idx → (idx+1)%n carefully.
        let n = verts.len();
        let i1 = (idx + 1) % n;
        // Keep start of segment as verts[idx], move verts[i1] or insert.
        // Simpler approach: rebuild from segments with one replaced.
        let mut segs: Vec<Line> = poly.segments().collect();
        segs[idx] = trimmed;
        return polyline_from_segments(&segs, true, tol);
    }
    // Open: segment idx connects verts[idx]–verts[idx+1]
    if idx + 1 >= verts.len() {
        return None;
    }
    verts[idx] = trimmed.start;
    verts[idx + 1] = trimmed.end;
    Some(Polyline {
        vertices: verts,
        closed: false,
    })
}

/// Extend the polyline segment nearest `pick` to meet `boundary`.
pub fn extend_polyline_to(
    poly: &Polyline,
    boundary: Line,
    pick: Vec2,
    tol: Tolerance,
) -> Option<Polyline> {
    if poly.closed || poly.vertices.len() < 2 {
        return None;
    }
    // Prefer first or last segment based on pick proximity to ends.
    let first = poly.segment(0)?;
    let last_i = poly.segment_count() - 1;
    let last = poly.segment(last_i)?;
    let use_first = first.closest_point(pick).distance(pick) <= last.closest_point(pick).distance(pick);
    let (idx, seg) = if use_first { (0, first) } else { (last_i, last) };
    let ext = extend_line_to(seg, boundary, pick, tol)?;
    let mut verts = poly.vertices.clone();
    if idx == 0 {
        verts[0] = ext.start;
        if verts.len() > 1 {
            verts[1] = ext.end;
        }
    } else {
        let n = verts.len();
        verts[n - 2] = ext.start;
        verts[n - 1] = ext.end;
    }
    Some(Polyline::new(verts))
}

/// Trim an arc by a cutting line; keep the portion whose midpoint is closer to `keep`.
pub fn trim_arc_by_line(arc: Arc, cut: Line, keep: Vec2, tol: Tolerance) -> Option<Arc> {
    let circle = Circle::new(arc.center, arc.radius);
    let hits = match line_circle(cut, circle, tol) {
        LineCircleResult::Two(a, b) => vec![a, b],
        LineCircleResult::One(a) => vec![a],
        LineCircleResult::None => return None,
    };
    // Prefer hit that lies on the arc.
    let on_arc: Vec<Vec2> = hits
        .into_iter()
        .filter(|p| {
            let ang = (*p - arc.center).angle();
            arc.contains_angle(ang, tol)
        })
        .collect();
    let hit = on_arc.first().copied()?;
    let hit_ang = (hit - arc.center).angle();
    // Split arc at hit; keep side whose midpoint is closer to keep.
    let a = Arc {
        center: arc.center,
        radius: arc.radius,
        start_angle: arc.start_angle,
        end_angle: hit_ang,
        clockwise: arc.clockwise,
    };
    let b = Arc {
        center: arc.center,
        radius: arc.radius,
        start_angle: hit_ang,
        end_angle: arc.end_angle,
        clockwise: arc.clockwise,
    };
    let ma = a.point_at(0.5);
    let mb = b.point_at(0.5);
    if ma.distance(keep) <= mb.distance(keep) {
        Some(a)
    } else {
        Some(b)
    }
}

fn polyline_from_segments(segs: &[Line], closed: bool, tol: Tolerance) -> Option<Polyline> {
    if segs.is_empty() {
        return None;
    }
    let mut verts = vec![segs[0].start];
    for s in segs {
        if !s.end.nearly_eq(*verts.last().unwrap(), tol) {
            verts.push(s.end);
        } else {
            // still push end if distinct from last
            verts.push(s.end);
        }
    }
    if closed {
        if verts.len() >= 2 && verts.first().unwrap().nearly_eq(*verts.last().unwrap(), tol) {
            verts.pop();
        }
        Some(Polyline::closed(verts))
    } else {
        Some(Polyline::new(verts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_horizontal_by_vertical() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let cut = Line::new(Vec2::new(4.0, -1.0), Vec2::new(4.0, 1.0));
        let kept = trim_line_by_cut(target, cut, Vec2::new(1.0, 0.0), Tolerance::DEFAULT).unwrap();
        assert!(kept.start.nearly_eq(Vec2::ZERO, Tolerance::DEFAULT));
        assert!(kept.end.nearly_eq(Vec2::new(4.0, 0.0), Tolerance::DEFAULT));
    }

    #[test]
    fn extend_to_vertical() {
        let target = Line::new(Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0));
        let boundary = Line::new(Vec2::new(5.0, -1.0), Vec2::new(5.0, 1.0));
        let ext =
            extend_line_to(target, boundary, Vec2::new(2.0, 0.0), Tolerance::DEFAULT).unwrap();
        assert!(ext.end.nearly_eq(Vec2::new(5.0, 0.0), Tolerance::DEFAULT));
    }

    #[test]
    fn trim_polyline_open() {
        let p = Polyline::new(vec![
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
        ]);
        let cut = Line::new(Vec2::new(4.0, -1.0), Vec2::new(4.0, 1.0));
        let t = trim_polyline_by_cut(&p, cut, Vec2::new(1.0, 0.0), Tolerance::DEFAULT).unwrap();
        assert!(t.vertices[1].nearly_eq(Vec2::new(4.0, 0.0), Tolerance::DEFAULT));
    }
}

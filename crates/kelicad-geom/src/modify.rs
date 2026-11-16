//! Modify / edit primitives (ARRAY, BREAK, JOIN, STRETCH, LENGTHEN, ALIGN, EXPLODE helpers).

use kelicad_math::{Tolerance, Vec2};

use crate::line::Line;
use crate::polyline::Polyline;
use crate::transform::{rotate_point, translate_line};

/// Rectangular array offsets: `(cols-1)*(rows-1)+…` deltas excluding origin (0,0).
///
/// `cols`/`rows` ≥ 1. Returns translation vectors for copies (not including the original).
pub fn rectangular_array_offsets(cols: usize, rows: usize, dx: f64, dy: f64) -> Vec<Vec2> {
    let mut out = Vec::new();
    let cols = cols.max(1);
    let rows = rows.max(1);
    for r in 0..rows {
        for c in 0..cols {
            if c == 0 && r == 0 {
                continue;
            }
            out.push(Vec2::new(c as f64 * dx, r as f64 * dy));
        }
    }
    out
}

/// Polar array: `count` total items including original at angle 0.
/// Returns rotations (radians) and positions for copies 1..count-1 around `center`.
pub fn polar_array_copies(
    center: Vec2,
    base: Vec2,
    count: usize,
    fill_degrees: f64,
) -> Vec<(Vec2, f64)> {
    let count = count.max(1);
    if count == 1 {
        return vec![];
    }
    let mut out = Vec::with_capacity(count - 1);
    let step = fill_degrees.to_radians() / (count as f64);
    let radius = (base - center).length();
    let start_ang = (base - center).angle();
    for i in 1..count {
        let ang = start_ang + step * i as f64;
        let pos = center + Vec2::new(ang.cos(), ang.sin()) * radius;
        let rot = step * i as f64;
        out.push((pos, rot));
    }
    out
}

/// Break a line at parameter `t` ∈ (0,1). Returns two segments, or `None` if degenerate.
pub fn break_line_at(line: Line, t: f64, tol: Tolerance) -> Option<(Line, Line)> {
    if t <= 0.0 || t >= 1.0 || !t.is_finite() {
        return None;
    }
    let mid = line.point_at(t);
    let a = Line::new(line.start, mid);
    let b = Line::new(mid, line.end);
    if a.is_degenerate(tol) || b.is_degenerate(tol) {
        return None;
    }
    Some((a, b))
}

/// Break a line at the closest point to `pick` if interior.
pub fn break_line_near(line: Line, pick: Vec2, tol: Tolerance) -> Option<(Line, Line)> {
    let t = line.closest_param(pick);
    if t <= tol.abs || t >= 1.0 - tol.abs {
        return None;
    }
    break_line_at(line, t, tol)
}

/// Join two collinear lines that share an endpoint within `tol`.
pub fn join_collinear_lines(a: Line, b: Line, tol: Tolerance) -> Option<Line> {
    let da = a.delta();
    let db = b.delta();
    let na = da.try_normalize(tol)?;
    let nb = db.try_normalize(tol)?;
    // Parallel (same or opposite direction).
    if na.cross(nb).abs() > tol.abs * 10.0 {
        return None;
    }
    // Collect endpoints; find shared vertex pairs.
    let pts = [a.start, a.end, b.start, b.end];
    let mut shared = None;
    for i in 0..2 {
        for j in 2..4 {
            if pts[i].nearly_eq(pts[j], tol) {
                shared = Some((i, j));
                break;
            }
        }
        if shared.is_some() {
            break;
        }
    }
    let (i, j) = shared?;
    let other_a = if i == 0 { a.end } else { a.start };
    let other_b = if j == 2 { b.end } else { b.start };
    // Colinearity check via cross product distance.
    if (other_b - a.start).cross(na).abs() > Tolerance::COARSE.abs {
        return None;
    }
    Some(Line::new(other_a, other_b))
}

/// Join open polylines that share an endpoint (order may flip).
pub fn join_polylines(a: &Polyline, b: &Polyline, tol: Tolerance) -> Option<Polyline> {
    if a.closed || b.closed || a.vertices.len() < 2 || b.vertices.len() < 2 {
        return None;
    }
    let a0 = *a.vertices.first()?;
    let a1 = *a.vertices.last()?;
    let b0 = *b.vertices.first()?;
    let b1 = *b.vertices.last()?;
    let mut verts = a.vertices.clone();
    if a1.nearly_eq(b0, tol) {
        verts.extend(b.vertices.iter().skip(1).copied());
    } else if a1.nearly_eq(b1, tol) {
        verts.extend(b.vertices.iter().rev().skip(1).copied());
    } else if a0.nearly_eq(b1, tol) {
        let mut v: Vec<_> = b.vertices.clone();
        v.extend(a.vertices.iter().skip(1).copied());
        verts = v;
    } else if a0.nearly_eq(b0, tol) {
        let mut v: Vec<_> = b.vertices.iter().rev().copied().collect();
        v.extend(a.vertices.iter().skip(1).copied());
        verts = v;
    } else {
        return None;
    }
    Some(Polyline::new(verts))
}

/// Lengthen (or shorten) a line by `delta` at the end nearest `pick`.
/// Positive delta extends; negative shortens (clamped to keep min length).
pub fn lengthen_line(line: Line, pick: Vec2, delta: f64, tol: Tolerance) -> Option<Line> {
    if !delta.is_finite() || line.is_degenerate(tol) {
        return None;
    }
    let d_start = line.start.distance(pick);
    let d_end = line.end.distance(pick);
    let dir = line.delta().try_normalize(tol)?;
    if d_end <= d_start {
        let new_end = line.end + dir * delta;
        let out = Line::new(line.start, new_end);
        if out.length() < tol.abs {
            return None;
        }
        Some(out)
    } else {
        let new_start = line.start - dir * delta;
        let out = Line::new(new_start, line.end);
        if out.length() < tol.abs {
            return None;
        }
        Some(out)
    }
}

/// Stretch: if point is inside axis-aligned window [min,max], translate by `delta`.
pub fn stretch_point(p: Vec2, min: Vec2, max: Vec2, delta: Vec2) -> Vec2 {
    let lo = Vec2::new(min.x.min(max.x), min.y.min(max.y));
    let hi = Vec2::new(min.x.max(max.x), min.y.max(max.y));
    if p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y {
        p + delta
    } else {
        p
    }
}

/// Stretch a line: endpoints inside window move.
pub fn stretch_line(line: Line, min: Vec2, max: Vec2, delta: Vec2) -> Line {
    Line::new(
        stretch_point(line.start, min, max, delta),
        stretch_point(line.end, min, max, delta),
    )
}

/// Stretch polyline vertices inside window.
pub fn stretch_polyline(poly: &Polyline, min: Vec2, max: Vec2, delta: Vec2) -> Polyline {
    let vertices = poly
        .vertices
        .iter()
        .map(|p| stretch_point(*p, min, max, delta))
        .collect();
    Polyline {
        vertices,
        closed: poly.closed,
    }
}

/// 2-point align: map `src_from→src_to` onto `dst_from→dst_to` (translate + rotate + uniform scale).
/// Returns `(origin_src, translation_after_rot_scale, rotation, scale)`.
pub fn align_2point(
    src_from: Vec2,
    src_to: Vec2,
    dst_from: Vec2,
    dst_to: Vec2,
) -> Option<(Vec2, f64, f64)> {
    let s = src_to - src_from;
    let d = dst_to - dst_from;
    let sl = s.length();
    let dl = d.length();
    if sl < 1e-12 || dl < 1e-12 {
        return None;
    }
    let scale = dl / sl;
    let rot = d.angle() - s.angle();
    Some((src_from, rot, scale))
}

/// Apply align_2point transform to a point.
pub fn align_transform_point(
    p: Vec2,
    src_from: Vec2,
    rot: f64,
    scale: f64,
    dst_from: Vec2,
) -> Vec2 {
    let local = p - src_from;
    let scaled = local * scale;
    let rotated = rotate_point(scaled, Vec2::ZERO, rot);
    rotated + dst_from
}

/// Translate a line by delta (re-export helper style).
pub fn move_line(line: Line, delta: Vec2) -> Line {
    translate_line(line, delta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_math::Tolerance;

    #[test]
    fn rect_array_3x2() {
        let o = rectangular_array_offsets(3, 2, 10.0, 5.0);
        assert_eq!(o.len(), 5);
        assert!(o.contains(&Vec2::new(10.0, 0.0)));
        assert!(o.contains(&Vec2::new(20.0, 5.0)));
    }

    #[test]
    fn break_mid() {
        let l = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        let (a, b) = break_line_at(l, 0.5, Tolerance::DEFAULT).unwrap();
        assert!((a.length() - 5.0).abs() < 1e-9);
        assert!((b.length() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn join_collinear() {
        let a = Line::new(Vec2::ZERO, Vec2::new(5.0, 0.0));
        let b = Line::new(Vec2::new(5.0, 0.0), Vec2::new(10.0, 0.0));
        let j = join_collinear_lines(a, b, Tolerance::DEFAULT).unwrap();
        assert!((j.length() - 10.0).abs() < 1e-6);
    }

    #[test]
    fn lengthen_end() {
        let l = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        let n = lengthen_line(l, Vec2::new(10.0, 0.0), 2.0, Tolerance::DEFAULT).unwrap();
        assert!((n.length() - 12.0).abs() < 1e-9);
    }

    #[test]
    fn stretch_window() {
        let l = Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0));
        let s = stretch_line(l, Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 3.0));
        assert!(s.start.nearly_eq(Vec2::new(0.0, 3.0), Tolerance::DEFAULT));
        assert!(s.end.nearly_eq(Vec2::new(10.0, 0.0), Tolerance::DEFAULT));
    }

    #[test]
    fn align_scale_rotate() {
        let (origin, rot, scale) =
            align_2point(Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::ZERO, Vec2::new(0.0, 4.0)).unwrap();
        assert!((scale - 2.0).abs() < 1e-9);
        assert!((rot - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        let p = align_transform_point(Vec2::new(2.0, 0.0), origin, rot, scale, Vec2::ZERO);
        assert!(p.nearly_eq(Vec2::new(0.0, 4.0), Tolerance::DEFAULT));
    }
}

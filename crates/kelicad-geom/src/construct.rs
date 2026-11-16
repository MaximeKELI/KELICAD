//! Construction helpers inspired by LibreCAD / AutoCAD draw tools (MIT reimplementation).

use kelicad_math::{Tolerance, Vec2};

use crate::arc::Arc;
use crate::circle::Circle;
use crate::line::Line;
use crate::polyline::Polyline;

/// Circumcircle through three non-collinear points.
pub fn circle_from_3_points(a: Vec2, b: Vec2, c: Vec2, tol: Tolerance) -> Option<Circle> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if tol.is_zero(d) {
        return None;
    }
    let a2 = a.length_squared();
    let b2 = b.length_squared();
    let c2 = c.length_squared();
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    let center = Vec2::new(ux, uy);
    let radius = center.distance(a);
    if radius < tol.abs {
        return None;
    }
    Some(Circle::new(center, radius))
}

/// Circle from two diameter endpoints.
pub fn circle_from_2_points(a: Vec2, b: Vec2, tol: Tolerance) -> Option<Circle> {
    let center = a.lerp(b, 0.5);
    let radius = a.distance(b) * 0.5;
    if radius < tol.abs {
        None
    } else {
        Some(Circle::new(center, radius))
    }
}

/// Regular polygon (center + vertex).
pub fn regular_polygon_center_vertex(center: Vec2, vertex: Vec2, sides: usize) -> Option<Polyline> {
    let n = sides.max(3);
    let r = vertex.distance(center);
    if r < 1e-12 {
        return None;
    }
    let a0 = (vertex - center).angle();
    let mut vertices = Vec::with_capacity(n);
    for i in 0..n {
        let a = a0 + std::f64::consts::TAU * (i as f64) / (n as f64);
        vertices.push(center + Vec2::new(a.cos(), a.sin()) * r);
    }
    Some(Polyline {
        vertices,
        closed: true,
    })
}

/// Regular polygon from one side (two corners).
pub fn regular_polygon_side(a: Vec2, b: Vec2, sides: usize) -> Option<Polyline> {
    let n = sides.max(3);
    let edge = b - a;
    let len = edge.length();
    if len < 1e-12 {
        return None;
    }
    // Exterior angle.
    let turn = std::f64::consts::TAU / (n as f64);
    let mut vertices = vec![a, b];
    let mut cur = b;
    let mut dir = edge;
    for _ in 2..n {
        dir = dir.rotate(turn);
        cur += dir;
        // Renormalize length drift.
        let prev = vertices[vertices.len() - 1];
        let d = cur - prev;
        cur = prev + d.try_normalize(Tolerance::DEFAULT)? * len;
        vertices.push(cur);
    }
    Some(Polyline {
        vertices,
        closed: true,
    })
}

/// Angle bisector rays from the intersection of two lines (infinite).
/// Returns a segment of given `length` centered on the intersection when possible.
pub fn angle_bisector(a: Line, b: Line, length: f64, tol: Tolerance) -> Option<Line> {
    let da = a.delta().try_normalize(tol)?;
    let db = b.delta().try_normalize(tol)?;
    // Intersection of supporting lines.
    let denom = da.cross(db);
    if tol.is_zero(denom) {
        return None;
    }
    let t = (b.start - a.start).cross(db) / denom;
    let origin = a.start + da * t;
    let mut bis = da + db;
    if bis.length() < tol.abs {
        bis = da + (-db);
    }
    let dir = bis.try_normalize(tol)?;
    let half = length.abs() * 0.5;
    Some(Line::new(origin - dir * half, origin + dir * half))
}

/// Tangents from an external point to a circle (0, 1, or 2 lines).
pub fn tangents_point_circle(p: Vec2, c: Circle, tol: Tolerance) -> Vec<Line> {
    let v = p - c.center;
    let d = v.length();
    if d < c.radius + tol.abs {
        if (d - c.radius).abs() <= tol.abs {
            // On circle: single tangent perpendicular to radius.
            let dir = v.try_normalize(tol).map(|n| n.perp()).unwrap_or(Vec2::X);
            return vec![Line::new(p, p + dir)];
        }
        return vec![];
    }
    let ang = (c.radius / d).acos();
    let base = v.angle();
    let mut out = Vec::new();
    for sign in [-1.0, 1.0] {
        let a = base + sign * ang;
        let touch = c.center + Vec2::new(a.cos(), a.sin()) * c.radius;
        out.push(Line::new(p, touch));
    }
    out
}

/// External common tangents between two circles (up to 2).
pub fn tangents_circle_circle(a: Circle, b: Circle, tol: Tolerance) -> Vec<Line> {
    let diff = b.center - a.center;
    let d = diff.length();
    if d < tol.abs {
        return vec![];
    }
    // Homothety for external tangents (equal direction).
    let rdiff = a.radius - b.radius;
    if d.abs() <= rdiff.abs() + tol.abs {
        return vec![];
    }
    let dir = diff / d;
    let mut out = Vec::new();
    let sin_th = rdiff / d;
    let cos_th = (1.0 - sin_th * sin_th).max(0.0).sqrt();
    for sign in [-1.0, 1.0] {
        let nx = dir.x * sin_th - sign * dir.y * cos_th;
        let ny = dir.y * sin_th + sign * dir.x * cos_th;
        let n = Vec2::new(nx, ny);
        let p1 = a.center + n * a.radius;
        let p2 = b.center + n * b.radius;
        out.push(Line::new(p1, p2));
    }
    out
}

/// Fillet two line segments with radius `r`.
///
/// Returns `(trimmed_a, trimmed_b, arc)` when the lines can meet with a fillet.
pub fn fillet_lines(a: Line, b: Line, r: f64, tol: Tolerance) -> Option<(Line, Line, Arc)> {
    if r <= tol.abs {
        return None;
    }
    let da = a.delta().try_normalize(tol)?;
    let db = b.delta().try_normalize(tol)?;
    let denom = da.cross(db);
    if tol.is_zero(denom) {
        return None;
    }
    let t = (b.start - a.start).cross(db) / denom;
    let u = (b.start - a.start).cross(da) / denom;
    let corner = a.start + da * t;
    // Offset directions (inward relative to turn).
    let turn = da.cross(db);
    let n_a = if turn >= 0.0 { da.perp() } else { -da.perp() };
    let n_b = if turn >= 0.0 { db.perp() } else { -db.perp() };
    // Circle center = corner + offset along both normals (approx via angle bisector of normals).
    let half = (n_a + n_b).try_normalize(tol)?;
    // Distance from corner to center along bisector: r / sin(phi/2).
    let ang = da.dot(db).clamp(-1.0, 1.0).acos();
    let sin_half = (ang * 0.5).sin().abs().max(1e-12);
    let dist = r / sin_half;
    let center = corner + half * dist;
    // Touch points: project center onto lines.
    let ta = closest_on_infinite(a.start, da, center);
    let tb = closest_on_infinite(b.start, db, center);
    let a0 = (ta - center).angle();
    let a1 = (tb - center).angle();
    let clockwise = turn < 0.0;
    let arc = Arc {
        center,
        radius: r,
        start_angle: a0,
        end_angle: a1,
        clockwise,
    };
    // Trim: keep portions away from corner.
    let trim_a = trim_away_from_corner(a, corner, ta, tol)?;
    let trim_b = trim_away_from_corner(b, corner, tb, tol)?;
    let _ = (t, u); // intersection params kept for future validation
    Some((trim_a, trim_b, arc))
}

/// Chamfer two lines by distances `d1` along `a` and `d2` along `b` from the corner.
pub fn chamfer_lines(a: Line, b: Line, d1: f64, d2: f64, tol: Tolerance) -> Option<(Line, Line, Line)> {
    if d1 <= tol.abs || d2 <= tol.abs {
        return None;
    }
    let da = a.delta().try_normalize(tol)?;
    let db = b.delta().try_normalize(tol)?;
    let denom = da.cross(db);
    if tol.is_zero(denom) {
        return None;
    }
    let t = (b.start - a.start).cross(db) / denom;
    let corner = a.start + da * t;
    // Points along each line away from corner by d1/d2 — pick direction toward segment mid.
    let pa = point_along_away(a, corner, d1, tol)?;
    let pb = point_along_away(b, corner, d2, tol)?;
    let chamfer = Line::new(pa, pb);
    let trim_a = trim_away_from_corner(a, corner, pa, tol)?;
    let trim_b = trim_away_from_corner(b, corner, pb, tol)?;
    Some((trim_a, trim_b, chamfer))
}

fn closest_on_infinite(origin: Vec2, dir: Vec2, p: Vec2) -> Vec2 {
    let t = (p - origin).dot(dir);
    origin + dir * t
}

fn trim_away_from_corner(line: Line, corner: Vec2, keep_end: Vec2, tol: Tolerance) -> Option<Line> {
    let ds = line.start.distance(corner);
    let de = line.end.distance(corner);
    if ds <= de {
        // start nearer corner → keep end side
        Some(Line::new(keep_end, line.end))
    } else {
        Some(Line::new(line.start, keep_end))
    }
    .filter(|l| !l.is_degenerate(tol))
}

fn point_along_away(line: Line, corner: Vec2, dist: f64, tol: Tolerance) -> Option<Vec2> {
    let toward_start = line.start.distance(corner) > line.end.distance(corner);
    let dir = if toward_start {
        (line.start - corner).try_normalize(tol)?
    } else {
        (line.end - corner).try_normalize(tol)?
    };
    Some(corner + dir * dist)
}

/// Star polygon (center + outer vertex, `points` tips, `inner_ratio` ∈ (0,1)).
pub fn star_polygon(
    center: Vec2,
    outer_vertex: Vec2,
    points: usize,
    inner_ratio: f64,
) -> Option<Polyline> {
    let n = points.max(3);
    let r_out = outer_vertex.distance(center);
    if r_out < 1e-12 {
        return None;
    }
    let r_in = r_out * inner_ratio.clamp(0.05, 0.95);
    let a0 = (outer_vertex - center).angle();
    let mut vertices = Vec::with_capacity(n * 2);
    for i in 0..(n * 2) {
        let a = a0 + std::f64::consts::PI * (i as f64) / (n as f64);
        let r = if i % 2 == 0 { r_out } else { r_in };
        vertices.push(center + Vec2::new(a.cos(), a.sin()) * r);
    }
    Some(Polyline {
        vertices,
        closed: true,
    })
}

/// Capsule / stadium: segment `a`→`b` thickened by `radius` (polyline approx).
pub fn capsule(a: Vec2, b: Vec2, radius: f64, segments_per_cap: usize) -> Option<Polyline> {
    if radius < 1e-12 {
        return None;
    }
    let delta = b - a;
    let len = delta.length();
    if len < 1e-12 {
        // Degenerate → circle approx.
        return regular_polygon_center_vertex(a, a + Vec2::new(radius, 0.0), segments_per_cap.max(8) * 2);
    }
    let dir = delta / len;
    let nrm = dir.perp();
    let segs = segments_per_cap.max(4);
    let mut vertices = Vec::with_capacity(segs * 2 + 2);
    // Right side a→b
    vertices.push(a + nrm * radius);
    vertices.push(b + nrm * radius);
    // Cap at b (π semicircle)
    let base_b = (nrm).angle();
    for i in 1..=segs {
        let t = (i as f64) / (segs as f64);
        let ang = base_b + std::f64::consts::PI * t;
        vertices.push(b + Vec2::new(ang.cos(), ang.sin()) * radius);
    }
    // Left side b→a already covered by last cap point ≈ a - nrm*r via next
    vertices.push(a - nrm * radius);
    // Cap at a
    let base_a = (-nrm).angle();
    for i in 1..segs {
        let t = (i as f64) / (segs as f64);
        let ang = base_a + std::f64::consts::PI * t;
        vertices.push(a + Vec2::new(ang.cos(), ang.sin()) * radius);
    }
    Some(Polyline {
        vertices,
        closed: true,
    })
}

/// Arrow from `from` to `tip` with triangular head sized by `head_size`.
pub fn arrow(from: Vec2, tip: Vec2, head_size: f64) -> Option<Polyline> {
    let delta = tip - from;
    let len = delta.length();
    if len < 1e-12 {
        return None;
    }
    let dir = delta / len;
    let nrm = dir.perp();
    let hs = if head_size > 1e-12 {
        head_size.min(len * 0.45)
    } else {
        (len * 0.25).max(1e-3)
    };
    let base = tip - dir * hs;
    // Open polyline: shaft + chevron head outline as single stroke path.
    Some(Polyline {
        vertices: vec![
            from,
            base,
            base + nrm * (hs * 0.5),
            tip,
            base - nrm * (hs * 0.5),
            base,
        ],
        closed: false,
    })
}

/// Construction grid as axis-aligned lines covering `cols×rows` cells.
pub fn grid_lines(origin: Vec2, cols: usize, rows: usize, dx: f64, dy: f64) -> Vec<Line> {
    let cols = cols.max(1);
    let rows = rows.max(1);
    if dx.abs() < 1e-12 || dy.abs() < 1e-12 {
        return vec![];
    }
    let w = dx * cols as f64;
    let h = dy * rows as f64;
    let mut lines = Vec::with_capacity((cols + 1) + (rows + 1));
    for i in 0..=cols {
        let x = origin.x + dx * i as f64;
        lines.push(Line::new(
            Vec2::new(x, origin.y),
            Vec2::new(x, origin.y + h),
        ));
    }
    for j in 0..=rows {
        let y = origin.y + dy * j as f64;
        lines.push(Line::new(
            Vec2::new(origin.x, y),
            Vec2::new(origin.x + w, y),
        ));
    }
    let _ = w;
    lines
}

/// Revision cloud along an open/closed path (bulb arcs — LibreCAD/AutoCAD style).
pub fn revision_cloud(path: &[Vec2], bulge: f64, closed: bool) -> Option<Polyline> {
    if path.len() < 2 || bulge.abs() < 1e-9 {
        return None;
    }
    let mut verts = Vec::new();
    let n_seg = if closed { path.len() } else { path.len() - 1 };
    for i in 0..n_seg {
        let a = path[i];
        let b = path[(i + 1) % path.len()];
        let mid = a.lerp(b, 0.5);
        let delta = b - a;
        let len = delta.length();
        if len < 1e-12 {
            continue;
        }
        let nrm = delta.perp() / len;
        let tip = mid + nrm * bulge.abs();
        // Approximate each bulb with 5 points.
        for k in 0..=4 {
            let t = k as f64 / 4.0;
            let p = if t <= 0.5 {
                a.lerp(tip, t * 2.0)
            } else {
                tip.lerp(b, (t - 0.5) * 2.0)
            };
            if verts.last().map(|q: &Vec2| q.distance(p) > 1e-9).unwrap_or(true) {
                verts.push(p);
            }
        }
    }
    if verts.len() < 3 {
        return None;
    }
    Some(Polyline {
        vertices: verts,
        closed,
    })
}

/// Centerline between two points with end ticks (ISO-ish).
pub fn centerline(a: Vec2, b: Vec2, tick: f64) -> Option<Vec<Line>> {
    let delta = b - a;
    let len = delta.length();
    if len < 1e-12 {
        return None;
    }
    let dir = delta / len;
    let nrm = dir.perp();
    let t = tick.abs().max(1e-6);
    Some(vec![
        Line::new(a, b),
        Line::new(a - nrm * t, a + nrm * t),
        Line::new(b - nrm * t, b + nrm * t),
    ])
}

/// Table grid as lines (cols×rows cells from origin with cell size).
pub fn table_grid(origin: Vec2, cols: usize, rows: usize, cell_w: f64, cell_h: f64) -> Vec<Line> {
    grid_lines(origin, cols.max(1), rows.max(1), cell_w.abs().max(1e-6), cell_h.abs().max(1e-6))
}

/// Rectangular wipeout boundary (closed polyline).
pub fn wipeout_rect(a: Vec2, b: Vec2) -> Polyline {
    Polyline::rectangle(a, b)
}

/// Center mark cross for a circle (axes ± radius * factor).
pub fn center_mark(center: Vec2, radius: f64, factor: f64) -> Vec<Line> {
    let r = radius.abs().max(1e-9);
    let arm = r * factor.clamp(0.05, 2.0);
    vec![
        Line::new(
            Vec2::new(center.x - arm, center.y),
            Vec2::new(center.x + arm, center.y),
        ),
        Line::new(
            Vec2::new(center.x, center.y - arm),
            Vec2::new(center.x, center.y + arm),
        ),
    ]
}

/// Isosceles triangle given base corners `a`,`b` and apex height (signed via `height`).
pub fn isosceles_triangle(a: Vec2, b: Vec2, height: f64) -> Option<Polyline> {
    let mid = a.lerp(b, 0.5);
    let base = b - a;
    let len = base.length();
    if len < 1e-12 {
        return None;
    }
    let nrm = base.perp() / len;
    let apex = mid + nrm * height;
    Some(Polyline {
        vertices: vec![a, b, apex],
        closed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_3p_unit() {
        let c = circle_from_3_points(
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            Tolerance::DEFAULT,
        )
        .unwrap();
        assert!(c.center.nearly_eq(Vec2::ZERO, Tolerance::COARSE));
        assert!((c.radius - 1.0).abs() < 1e-6);
    }

    #[test]
    fn hexagon_has_six_sides() {
        let p = regular_polygon_center_vertex(Vec2::ZERO, Vec2::new(1.0, 0.0), 6).unwrap();
        assert_eq!(p.vertices.len(), 6);
        assert!(p.closed);
    }

    #[test]
    fn tangents_from_point() {
        let lines = tangents_point_circle(Vec2::new(5.0, 0.0), Circle::new(Vec2::ZERO, 1.0), Tolerance::DEFAULT);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn fillet_right_angle() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0));
        let (ta, tb, arc) = fillet_lines(a, b, 1.0, Tolerance::DEFAULT).unwrap();
        assert!((arc.radius - 1.0).abs() < 1e-6);
        assert!(!ta.is_degenerate(Tolerance::DEFAULT));
        assert!(!tb.is_degenerate(Tolerance::DEFAULT));
    }

    #[test]
    fn chamfer_right_angle() {
        let a = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        let b = Line::new(Vec2::new(0.0, 0.0), Vec2::new(0.0, 10.0));
        let (_ta, _tb, ch) = chamfer_lines(a, b, 1.0, 1.0, Tolerance::DEFAULT).unwrap();
        assert!((ch.length() - std::f64::consts::SQRT_2).abs() < 1e-3);
    }

    #[test]
    fn star_has_ten_vertices() {
        let s = star_polygon(Vec2::ZERO, Vec2::new(2.0, 0.0), 5, 0.4).unwrap();
        assert_eq!(s.vertices.len(), 10);
        assert!(s.closed);
    }

    #[test]
    fn capsule_closed() {
        let c = capsule(Vec2::ZERO, Vec2::new(4.0, 0.0), 0.5, 8).unwrap();
        assert!(c.closed);
        assert!(c.vertices.len() >= 8);
    }

    #[test]
    fn arrow_points_to_tip() {
        let a = arrow(Vec2::ZERO, Vec2::new(10.0, 0.0), 2.0).unwrap();
        assert!(a.vertices.iter().any(|p| p.nearly_eq(Vec2::new(10.0, 0.0), Tolerance::COARSE)));
    }

    #[test]
    fn grid_3x2() {
        let g = grid_lines(Vec2::ZERO, 3, 2, 1.0, 1.0);
        assert_eq!(g.len(), 4 + 3); // 4 vertical + 3 horizontal
    }
}

//! Trim / extend helpers for finite segments.

use kelicad_math::{Tolerance, Vec2};

use crate::intersect::{line_line, LineLineResult};
use crate::line::Line;

/// Trim `target` using a cutting segment: keep the portion on the side of
/// `keep_point` relative to the intersection (or unchanged if no hit).
pub fn trim_line_by_cut(
    target: Line,
    cut: Line,
    keep_point: Vec2,
    tol: Tolerance,
) -> Option<Line> {
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
pub fn extend_line_to(
    target: Line,
    boundary: Line,
    pick: Vec2,
    tol: Tolerance,
) -> Option<Line> {
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
        let ext = extend_line_to(target, boundary, Vec2::new(2.0, 0.0), Tolerance::DEFAULT).unwrap();
        assert!(ext.end.nearly_eq(Vec2::new(5.0, 0.0), Tolerance::DEFAULT));
    }
}

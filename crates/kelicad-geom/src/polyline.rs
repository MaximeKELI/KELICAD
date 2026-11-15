//! Lightweight polyline (straight segments; bulges deferred).

use kelicad_math::{Tolerance, Vec2};

use crate::aabb::Aabb;
use crate::line::Line;

/// Open or closed polyline made of straight segments.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    /// Vertices in order.
    pub vertices: Vec<Vec2>,
    /// If true, last segment connects back to the first vertex.
    pub closed: bool,
}

impl Polyline {
    /// Create an open polyline; needs at least one vertex.
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices,
            closed: false,
        }
    }

    /// Closed polygon ring.
    pub fn closed(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices,
            closed: true,
        }
    }

    /// Axis-aligned rectangle as a closed polyline.
    pub fn rectangle(corner: Vec2, opposite: Vec2) -> Self {
        let min = Vec2::new(corner.x.min(opposite.x), corner.y.min(opposite.y));
        let max = Vec2::new(corner.x.max(opposite.x), corner.y.max(opposite.y));
        Self::closed(vec![
            min,
            Vec2::new(max.x, min.y),
            max,
            Vec2::new(min.x, max.y),
        ])
    }

    /// True if fewer than two distinct points for an open poly, or &lt; 3 when closed.
    pub fn is_degenerate(&self, tol: Tolerance) -> bool {
        if self.vertices.len() < 2 {
            return true;
        }
        if self.closed && self.vertices.len() < 3 {
            return true;
        }
        let mut prev = self.vertices[0];
        let mut distinct = 1usize;
        for &v in &self.vertices[1..] {
            if !v.nearly_eq(prev, tol) {
                distinct += 1;
                prev = v;
            }
        }
        if self.closed {
            distinct < 3
        } else {
            distinct < 2
        }
    }

    /// Segment count (includes closing edge when closed).
    pub fn segment_count(&self) -> usize {
        if self.vertices.len() < 2 {
            return 0;
        }
        if self.closed {
            self.vertices.len()
        } else {
            self.vertices.len() - 1
        }
    }

    /// Finite segment at index.
    pub fn segment(&self, index: usize) -> Option<Line> {
        let n = self.segment_count();
        if index >= n {
            return None;
        }
        let a = self.vertices[index];
        let b = if self.closed && index + 1 == self.vertices.len() {
            self.vertices[0]
        } else {
            self.vertices[index + 1]
        };
        Some(Line::new(a, b))
    }

    /// Iterate segments.
    pub fn segments(&self) -> impl Iterator<Item = Line> + '_ {
        (0..self.segment_count()).filter_map(|i| self.segment(i))
    }

    /// Bounding box.
    pub fn aabb(&self) -> Aabb {
        self.vertices
            .iter()
            .fold(Aabb::EMPTY, |b, &p| b.expand(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_has_four_segments() {
        let p = Polyline::rectangle(Vec2::ZERO, Vec2::new(2.0, 3.0));
        assert!(p.closed);
        assert_eq!(p.segment_count(), 4);
    }
}

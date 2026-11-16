//! Snap resolution against a document.

use kelicad_doc::{Document, Geometry};
use kelicad_geom::intersect::{line_line, LineLineResult};
use kelicad_geom::{Circle, Line};
use kelicad_math::{Tolerance, Vec2};

use crate::kinds::{SnapKind, SnapModes};

/// Query parameters for a snap resolve.
#[derive(Debug, Clone, Copy)]
pub struct SnapQuery {
    /// Cursor position in world space.
    pub cursor: Vec2,
    /// Aperture radius in world units.
    pub aperture: f64,
    /// Optional base point (for perpendicular / polar).
    pub base: Option<Vec2>,
}

/// A resolved snap hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapHit {
    /// World position.
    pub point: Vec2,
    /// Snap kind.
    pub kind: SnapKind,
    /// Distance from cursor.
    pub distance: f64,
}

/// Collects and resolves object snaps.
#[derive(Debug, Clone)]
pub struct SnapEngine {
    /// Enabled modes.
    pub modes: SnapModes,
    /// Geometric tolerance.
    pub tol: Tolerance,
}

impl Default for SnapEngine {
    fn default() -> Self {
        Self {
            modes: SnapModes::default(),
            tol: Tolerance::DEFAULT,
        }
    }
}

impl SnapEngine {
    /// Resolve the best snap near `query`, or `None` if nothing in aperture.
    pub fn resolve(&self, doc: &Document, query: SnapQuery) -> Option<SnapHit> {
        let mut candidates = Vec::new();
        self.collect_entity_snaps(doc, query, &mut candidates);
        if self.modes.intersection {
            self.collect_intersections(doc, query, &mut candidates);
        }
        candidates
            .into_iter()
            .filter(|h| h.distance <= query.aperture)
            .min_by(|a, b| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.kind.priority().cmp(&b.kind.priority()))
            })
    }

    fn push(candidates: &mut Vec<SnapHit>, point: Vec2, kind: SnapKind, cursor: Vec2) {
        candidates.push(SnapHit {
            distance: point.distance(cursor),
            point,
            kind,
        });
    }

    fn collect_entity_snaps(&self, doc: &Document, query: SnapQuery, out: &mut Vec<SnapHit>) {
        for ent in doc.entities() {
            match &ent.geometry {
                Geometry::Line(l) => self.line_snaps(*l, query, out),
                Geometry::Circle(c) => self.circle_snaps(*c, query, out),
                Geometry::Arc(a) => {
                    if self.modes.endpoint {
                        Self::push(out, a.start_point(), SnapKind::Endpoint, query.cursor);
                        Self::push(out, a.end_point(), SnapKind::Endpoint, query.cursor);
                    }
                    if self.modes.center {
                        Self::push(out, a.center, SnapKind::Center, query.cursor);
                    }
                    if self.modes.nearest {
                        // Approximate nearest via supporting circle projection.
                        let v = query.cursor - a.center;
                        if let Some(n) = v.try_normalize(self.tol) {
                            let p = a.center + n * a.radius;
                            let ang = (p - a.center).angle();
                            if a.contains_angle(ang, self.tol) {
                                Self::push(out, p, SnapKind::Nearest, query.cursor);
                            }
                        }
                    }
                }
                Geometry::Polyline(p) => {
                    for seg in p.segments() {
                        self.line_snaps(seg, query, out);
                    }
                }
                Geometry::Point(p) => {
                    if self.modes.endpoint {
                        Self::push(out, *p, SnapKind::Endpoint, query.cursor);
                    }
                }
                Geometry::Text(t) => {
                    if self.modes.endpoint {
                        Self::push(out, t.position, SnapKind::Endpoint, query.cursor);
                    }
                }
                Geometry::MText(t) => {
                    if self.modes.endpoint {
                        Self::push(out, t.position, SnapKind::Endpoint, query.cursor);
                    }
                }
                Geometry::Dimension(d) => {
                    if self.modes.endpoint {
                        Self::push(out, d.def1, SnapKind::Endpoint, query.cursor);
                        Self::push(out, d.def2, SnapKind::Endpoint, query.cursor);
                    }
                    if self.modes.midpoint {
                        Self::push(
                            out,
                            d.def1.lerp(d.def2, 0.5),
                            SnapKind::Midpoint,
                            query.cursor,
                        );
                    }
                }
                Geometry::Hatch(h) => {
                    for &p in &h.boundary {
                        if self.modes.endpoint {
                            Self::push(out, p, SnapKind::Endpoint, query.cursor);
                        }
                    }
                }
                Geometry::Insert(i) => {
                    if self.modes.endpoint {
                        Self::push(out, i.position, SnapKind::Endpoint, query.cursor);
                    }
                }
                Geometry::Ellipse(e) => {
                    if self.modes.center {
                        Self::push(out, e.center, SnapKind::Center, query.cursor);
                    }
                    if self.modes.endpoint {
                        Self::push(out, e.center + e.major, SnapKind::Endpoint, query.cursor);
                    }
                    if self.modes.nearest {
                        for poly in [e.to_polyline(32)] {
                            for seg in poly.segments() {
                                self.line_snaps(seg, query, out);
                            }
                        }
                    }
                }
                Geometry::Spline(s) => {
                    let poly = s.to_polyline(8);
                    for seg in poly.segments() {
                        self.line_snaps(seg, query, out);
                    }
                }
            }
        }
    }

    fn line_snaps(&self, line: Line, query: SnapQuery, out: &mut Vec<SnapHit>) {
        if self.modes.endpoint {
            Self::push(out, line.start, SnapKind::Endpoint, query.cursor);
            Self::push(out, line.end, SnapKind::Endpoint, query.cursor);
        }
        if self.modes.midpoint {
            Self::push(out, line.point_at(0.5), SnapKind::Midpoint, query.cursor);
        }
        if self.modes.nearest {
            Self::push(
                out,
                line.closest_point(query.cursor),
                SnapKind::Nearest,
                query.cursor,
            );
        }
        if self.modes.perpendicular {
            if let Some(base) = query.base {
                let t = line.closest_param(base).clamp(0.0, 1.0);
                Self::push(out, line.point_at(t), SnapKind::Perpendicular, query.cursor);
            }
        }
    }

    fn circle_snaps(&self, circle: Circle, query: SnapQuery, out: &mut Vec<SnapHit>) {
        if self.modes.center {
            Self::push(out, circle.center, SnapKind::Center, query.cursor);
        }
        if self.modes.quadrant {
            for k in 0..4 {
                let ang = k as f64 * std::f64::consts::FRAC_PI_2;
                Self::push(
                    out,
                    circle.point_at_angle(ang),
                    SnapKind::Quadrant,
                    query.cursor,
                );
            }
        }
        if self.modes.nearest {
            let v = query.cursor - circle.center;
            if let Some(n) = v.try_normalize(self.tol) {
                Self::push(
                    out,
                    circle.center + n * circle.radius,
                    SnapKind::Nearest,
                    query.cursor,
                );
            }
        }
        if self.modes.perpendicular {
            if let Some(base) = query.base {
                let v = base - circle.center;
                if let Some(n) = v.try_normalize(self.tol) {
                    Self::push(
                        out,
                        circle.center + n * circle.radius,
                        SnapKind::Perpendicular,
                        query.cursor,
                    );
                }
            }
        }
    }

    fn collect_intersections(&self, doc: &Document, query: SnapQuery, out: &mut Vec<SnapHit>) {
        let lines: Vec<Line> = doc
            .entities()
            .flat_map(|e| match &e.geometry {
                Geometry::Line(l) => vec![*l],
                Geometry::Polyline(p) => p.segments().collect(),
                _ => Vec::new(),
            })
            .collect();
        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                if let LineLineResult::Point(p) = line_line(lines[i], lines[j], self.tol) {
                    Self::push(out, p, SnapKind::Intersection, query.cursor);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_doc::document::EntityRecord;
    use kelicad_doc::{EntityId, EntityKind, Geometry};
    use kelicad_geom::Line;
    use uuid::Uuid;

    #[test]
    fn snaps_to_endpoint() {
        let mut doc = Document::new();
        let id = EntityId::from_uuid(Uuid::from_u128(1));
        doc.upsert_entity(EntityRecord {
            id,
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(10.0, 0.0))),
            layer_id: doc.default_layer_id(),
        });
        let mut engine = SnapEngine::default();
        engine.modes.nearest = false;
        let hit = engine
            .resolve(
                &doc,
                SnapQuery {
                    cursor: Vec2::new(0.05, 0.02),
                    aperture: 1.0,
                    base: None,
                },
            )
            .expect("hit");
        assert_eq!(hit.kind, SnapKind::Endpoint);
        assert!(hit.point.nearly_eq(Vec2::ZERO, Tolerance::DEFAULT));
    }
}

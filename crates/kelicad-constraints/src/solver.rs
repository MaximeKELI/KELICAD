//! Iterative Gauss–Newton solver over line endpoints.

use std::collections::HashMap;

use kelicad_doc::{Document, EntityId, Geometry};
use kelicad_geom::Line;
use kelicad_math::{Tolerance, Vec2};
use kelicad_ops::Operation;

use crate::error::ConstraintError;
use crate::system::ConstraintSystem;
use crate::types::{Constraint, Endpoint};

/// Outcome of a solve attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolveStatus {
    /// Residuals below tolerance.
    Solved,
    /// Residuals reduced but free DOF remain (still usable).
    Underconstrained,
    /// Conflicting constraints; geometry left unchanged.
    Inconsistent,
}

/// Result of [`solve`].
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// High-level status.
    pub status: SolveStatus,
    /// Max absolute residual after solve (or before abort).
    pub residual: f64,
    /// Forward operations to apply (`SetGeometry` / `Batch`). Empty if inconsistent.
    pub ops: Vec<Operation>,
}

const MAX_ITERS: usize = 40;
const DAMP: f64 = 0.55;
const STEP_EPS: f64 = 1e-9;

/// Solve `system` against `doc`. Returns ops; does not mutate `doc`.
pub fn solve(system: &ConstraintSystem, doc: &Document) -> Result<SolveResult, ConstraintError> {
    if system.is_empty() {
        return Ok(SolveResult {
            status: SolveStatus::Underconstrained,
            residual: 0.0,
            ops: vec![],
        });
    }

    let mut lines: HashMap<EntityId, Line> = HashMap::new();
    for id in system.touched_entities() {
        let Some(ent) = doc.get_entity(id) else {
            return Err(ConstraintError::NotALine(id));
        };
        let Geometry::Line(line) = &ent.geometry else {
            return Err(ConstraintError::NotALine(id));
        };
        lines.insert(id, *line);
    }

    let constraints: Vec<_> = system.constraints().into_iter().map(|(_, c)| c).collect();
    let params = system.params().clone();
    let tol = Tolerance::COARSE;
    let initial_residual = max_residual(&constraints, &lines, &params);

    // Detect hard inconsistency: two Distance on same entity with different values.
    if conflicting_distances(&constraints, tol) {
        return Ok(SolveResult {
            status: SolveStatus::Inconsistent,
            residual: initial_residual,
            ops: vec![],
        });
    }
    // Horizontal + Vertical + Distance > 0 on same entity → zero-length vs positive.
    if conflicting_hv_distance(&constraints, tol) {
        return Ok(SolveResult {
            status: SolveStatus::Inconsistent,
            residual: initial_residual,
            ops: vec![],
        });
    }

    let mut residual;
    for _ in 0..MAX_ITERS {
        residual = max_residual(&constraints, &lines, &params);
        if residual < tol.abs {
            break;
        }
        let mut deltas: HashMap<EntityId, (Vec2, Vec2)> = HashMap::new();
        for c in &constraints {
            apply_projection(c, &lines, &mut deltas, DAMP, &params);
        }
        if deltas.is_empty() {
            break;
        }
        let mut moved = false;
        for (id, (ds, de)) in deltas {
            if let Some(line) = lines.get_mut(&id) {
                if ds.length() > STEP_EPS || de.length() > STEP_EPS {
                    line.start += ds;
                    line.end += de;
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }

    residual = max_residual(&constraints, &lines, &params);
    if residual > tol.abs * 50.0 {
        return Ok(SolveResult {
            status: SolveStatus::Inconsistent,
            residual,
            ops: vec![],
        });
    }

    let mut ops = Vec::new();
    for (id, line) in &lines {
        let Some(prev) = doc.get_entity(*id) else {
            continue;
        };
        let Geometry::Line(old) = &prev.geometry else {
            continue;
        };
        if !old.start.nearly_eq(line.start, tol) || !old.end.nearly_eq(line.end, tol) {
            ops.push(Operation::SetGeometry {
                id: *id,
                geometry: Geometry::Line(*line),
            });
        }
    }

    let status = if residual < tol.abs {
        if dof_hint(&constraints, lines.len()) > 0 {
            SolveStatus::Underconstrained
        } else {
            SolveStatus::Solved
        }
    } else if residual < tol.abs * 10.0 {
        SolveStatus::Underconstrained
    } else {
        SolveStatus::Inconsistent
    };

    if status == SolveStatus::Inconsistent {
        return Ok(SolveResult {
            status,
            residual,
            ops: vec![],
        });
    }

    Ok(SolveResult {
        status,
        residual,
        ops,
    })
}

fn conflicting_distances(constraints: &[Constraint], tol: Tolerance) -> bool {
    let mut by_entity: HashMap<EntityId, f64> = HashMap::new();
    for c in constraints {
        if let Constraint::Distance { entity, value } = c {
            if let Some(prev) = by_entity.get(entity) {
                if (prev - value).abs() > tol.abs {
                    return true;
                }
            } else {
                by_entity.insert(*entity, *value);
            }
        }
    }
    false
}

fn conflicting_hv_distance(constraints: &[Constraint], tol: Tolerance) -> bool {
    let mut h = std::collections::HashSet::new();
    let mut v = std::collections::HashSet::new();
    let mut dist: HashMap<EntityId, f64> = HashMap::new();
    for c in constraints {
        match c {
            Constraint::Horizontal { entity } => {
                h.insert(*entity);
            }
            Constraint::Vertical { entity } => {
                v.insert(*entity);
            }
            Constraint::Distance { entity, value } => {
                dist.insert(*entity, *value);
            }
            _ => {}
        }
    }
    for (id, value) in dist {
        if h.contains(&id) && v.contains(&id) && value > tol.abs {
            return true;
        }
    }
    false
}

fn dof_hint(constraints: &[Constraint], n_lines: usize) -> i32 {
    let vars = (n_lines * 4) as i32;
    let mut eqs = 0i32;
    for c in constraints {
        eqs += match c {
            Constraint::Coincident { .. } => 2,
            Constraint::Horizontal { .. }
            | Constraint::Vertical { .. }
            | Constraint::Parallel { .. }
            | Constraint::Perpendicular { .. }
            |             Constraint::EqualLength { .. }
            | Constraint::Distance { .. }
            | Constraint::Angle { .. }
            | Constraint::FormulaDistance { .. } => 1,
        };
    }
    vars - eqs
}

fn endpoint(line: Line, end: Endpoint) -> Vec2 {
    match end {
        Endpoint::Start => line.start,
        Endpoint::End => line.end,
    }
}

fn max_residual(
    constraints: &[Constraint],
    lines: &HashMap<EntityId, Line>,
    params: &HashMap<String, f64>,
) -> f64 {
    let mut m = 0.0f64;
    for c in constraints {
        for r in residuals(c, lines, params) {
            m = m.max(r.abs());
        }
    }
    m
}

fn residuals(
    c: &Constraint,
    lines: &HashMap<EntityId, Line>,
    params: &HashMap<String, f64>,
) -> Vec<f64> {
    match c {
        Constraint::Coincident { a, a_end, b, b_end } => {
            let pa = endpoint(*lines.get(a).expect("line"), *a_end);
            let pb = endpoint(*lines.get(b).expect("line"), *b_end);
            vec![pa.x - pb.x, pa.y - pb.y]
        }
        Constraint::Horizontal { entity } => {
            let l = lines.get(entity).expect("line");
            vec![l.end.y - l.start.y]
        }
        Constraint::Vertical { entity } => {
            let l = lines.get(entity).expect("line");
            vec![l.end.x - l.start.x]
        }
        Constraint::Parallel { a, b } => {
            let la = lines.get(a).expect("line");
            let lb = lines.get(b).expect("line");
            let da = la.delta();
            let db = lb.delta();
            vec![da.x * db.y - da.y * db.x]
        }
        Constraint::Perpendicular { a, b } => {
            let la = lines.get(a).expect("line");
            let lb = lines.get(b).expect("line");
            let da = la.delta();
            let db = lb.delta();
            vec![da.dot(db)]
        }
        Constraint::EqualLength { a, b } => {
            let la = lines.get(a).expect("line");
            let lb = lines.get(b).expect("line");
            vec![la.length() - lb.length()]
        }
        Constraint::Distance { entity, value } => {
            let l = lines.get(entity).expect("line");
            vec![l.length() - value]
        }
        Constraint::Angle { entity, degrees } => {
            let l = lines.get(entity).expect("line");
            let cur = l.delta().angle().to_degrees();
            // Wrap residual to (−180, 180]
            let mut d = cur - degrees;
            while d > 180.0 {
                d -= 360.0;
            }
            while d <= -180.0 {
                d += 360.0;
            }
            vec![d]
        }
        Constraint::FormulaDistance { entity, formula } => {
            let l = lines.get(entity).expect("line");
            let target = crate::expr::eval_formula(formula, params).unwrap_or(l.length());
            vec![l.length() - target]
        }
    }
}

fn apply_projection(
    c: &Constraint,
    lines: &HashMap<EntityId, Line>,
    deltas: &mut HashMap<EntityId, (Vec2, Vec2)>,
    damp: f64,
    params: &HashMap<String, f64>,
) {
    match c {
        Constraint::Coincident { a, a_end, b, b_end } => {
            let la = *lines.get(a).expect("line");
            let lb = *lines.get(b).expect("line");
            let pa = endpoint(la, *a_end);
            let pb = endpoint(lb, *b_end);
            let mid = Vec2::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5);
            add_endpoint_delta(deltas, *a, *a_end, (mid - pa) * damp);
            add_endpoint_delta(deltas, *b, *b_end, (mid - pb) * damp);
        }
        Constraint::Horizontal { entity } => {
            let l = *lines.get(entity).expect("line");
            let mid_y = (l.start.y + l.end.y) * 0.5;
            add_endpoint_delta(
                deltas,
                *entity,
                Endpoint::Start,
                Vec2::new(0.0, (mid_y - l.start.y) * damp),
            );
            add_endpoint_delta(
                deltas,
                *entity,
                Endpoint::End,
                Vec2::new(0.0, (mid_y - l.end.y) * damp),
            );
        }
        Constraint::Vertical { entity } => {
            let l = *lines.get(entity).expect("line");
            let mid_x = (l.start.x + l.end.x) * 0.5;
            add_endpoint_delta(
                deltas,
                *entity,
                Endpoint::Start,
                Vec2::new((mid_x - l.start.x) * damp, 0.0),
            );
            add_endpoint_delta(
                deltas,
                *entity,
                Endpoint::End,
                Vec2::new((mid_x - l.end.x) * damp, 0.0),
            );
        }
        Constraint::Parallel { a, b } => {
            // Rotate B's end around start to match A's direction, preserving length.
            let la = *lines.get(a).expect("line");
            let lb = *lines.get(b).expect("line");
            let da = la.delta();
            let len_a = da.length();
            let len_b = lb.length();
            if len_a < 1e-12 || len_b < 1e-12 {
                return;
            }
            let dir = da * (1.0 / len_a);
            let target_end = lb.start + dir * len_b;
            add_endpoint_delta(deltas, *b, Endpoint::End, (target_end - lb.end) * damp);
        }
        Constraint::Perpendicular { a, b } => {
            let la = *lines.get(a).expect("line");
            let lb = *lines.get(b).expect("line");
            let da = la.delta();
            let len_a = da.length();
            let len_b = lb.length();
            if len_a < 1e-12 || len_b < 1e-12 {
                return;
            }
            let dir = da * (1.0 / len_a);
            let perp = Vec2::new(-dir.y, dir.x);
            // Choose orientation closer to current B.
            let cur = lb.delta() * (1.0 / len_b);
            let target_dir = if cur.dot(perp) >= 0.0 { perp } else { -perp };
            let target_end = lb.start + target_dir * len_b;
            add_endpoint_delta(deltas, *b, Endpoint::End, (target_end - lb.end) * damp);
        }
        Constraint::EqualLength { a, b } => {
            let la = *lines.get(a).expect("line");
            let lb = *lines.get(b).expect("line");
            let target = (la.length() + lb.length()) * 0.5;
            scale_line_to(deltas, *a, la, target, damp);
            scale_line_to(deltas, *b, lb, target, damp);
        }
        Constraint::Distance { entity, value } => {
            let l = *lines.get(entity).expect("line");
            scale_line_to(deltas, *entity, l, *value, damp);
        }
        Constraint::Angle { entity, degrees } => {
            let l = *lines.get(entity).expect("line");
            let len = l.length();
            if len < 1e-12 {
                return;
            }
            let target = degrees.to_radians();
            let dir = Vec2::new(target.cos(), target.sin());
            let mid = l.point_at(0.5);
            let half = len * 0.5;
            let ns = mid - dir * half;
            let ne = mid + dir * half;
            add_endpoint_delta(deltas, *entity, Endpoint::Start, (ns - l.start) * damp);
            add_endpoint_delta(deltas, *entity, Endpoint::End, (ne - l.end) * damp);
        }
        Constraint::FormulaDistance { entity, formula } => {
            let l = *lines.get(entity).expect("line");
            if let Ok(target) = crate::expr::eval_formula(formula, params) {
                if target > 0.0 {
                    scale_line_to(deltas, *entity, l, target, damp);
                }
            }
        }
    }
}

fn scale_line_to(
    deltas: &mut HashMap<EntityId, (Vec2, Vec2)>,
    id: EntityId,
    line: Line,
    target_len: f64,
    damp: f64,
) {
    let cur = line.length();
    if cur < 1e-12 {
        // Degenerate: stretch along +X from midpoint.
        let mid = line.point_at(0.5);
        let half = target_len * 0.5;
        let ns = Vec2::new(mid.x - half, mid.y);
        let ne = Vec2::new(mid.x + half, mid.y);
        add_endpoint_delta(deltas, id, Endpoint::Start, (ns - line.start) * damp);
        add_endpoint_delta(deltas, id, Endpoint::End, (ne - line.end) * damp);
        return;
    }
    let scale = target_len / cur;
    let mid = line.point_at(0.5);
    let ns = mid + (line.start - mid) * scale;
    let ne = mid + (line.end - mid) * scale;
    add_endpoint_delta(deltas, id, Endpoint::Start, (ns - line.start) * damp);
    add_endpoint_delta(deltas, id, Endpoint::End, (ne - line.end) * damp);
}

fn add_endpoint_delta(
    deltas: &mut HashMap<EntityId, (Vec2, Vec2)>,
    id: EntityId,
    end: Endpoint,
    d: Vec2,
) {
    let entry = deltas.entry(id).or_insert((Vec2::ZERO, Vec2::ZERO));
    match end {
        Endpoint::Start => entry.0 += d,
        Endpoint::End => entry.1 += d,
    }
}

#[cfg(test)]
mod tests {
    use kelicad_doc::{EntityKind, EntityRecord, LayerId};
    use kelicad_ops::DocSession;
    use uuid::Uuid;

    use super::*;
    use crate::types::ConstraintId;

    fn line_entity(id: EntityId, layer: LayerId, start: Vec2, end: Vec2) -> EntityRecord {
        EntityRecord {
            id,
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(start, end)),
            layer_id: layer,
        }
    }

    fn eid(n: u128) -> EntityId {
        EntityId::from_uuid(Uuid::from_u128(n))
    }

    #[test]
    fn horizontal_and_distance() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let id = eid(1);
        session
            .apply(Operation::CreateEntity {
                record: line_entity(id, layer, Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0)),
            })
            .unwrap();

        let mut sys = ConstraintSystem::new();
        sys.add(Constraint::Horizontal { entity: id }).unwrap();
        sys.add(Constraint::Distance {
            entity: id,
            value: 5.0,
        })
        .unwrap();

        let result = solve(&sys, &session.document).unwrap();
        assert!(matches!(
            result.status,
            SolveStatus::Solved | SolveStatus::Underconstrained
        ));
        assert!(!result.ops.is_empty());
        session
            .apply(Operation::Batch {
                ops: result.ops.clone(),
            })
            .unwrap();
        let Geometry::Line(l) = &session.document.get_entity(id).unwrap().geometry else {
            panic!("expected line");
        };
        assert!((l.end.y - l.start.y).abs() < 1e-6);
        assert!((l.length() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn parallel_pair() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let a = eid(10);
        let b = eid(11);
        session
            .apply(Operation::CreateEntity {
                record: line_entity(a, layer, Vec2::ZERO, Vec2::new(4.0, 0.0)),
            })
            .unwrap();
        session
            .apply(Operation::CreateEntity {
                record: line_entity(b, layer, Vec2::new(0.0, 1.0), Vec2::new(3.0, 2.0)),
            })
            .unwrap();

        let mut sys = ConstraintSystem::new();
        sys.add(Constraint::Parallel { a, b }).unwrap();
        let result = solve(&sys, &session.document).unwrap();
        session.apply(Operation::Batch { ops: result.ops }).unwrap();
        let Geometry::Line(la) = &session.document.get_entity(a).unwrap().geometry else {
            panic!();
        };
        let Geometry::Line(lb) = &session.document.get_entity(b).unwrap().geometry else {
            panic!();
        };
        let cross = la.delta().x * lb.delta().y - la.delta().y * lb.delta().x;
        assert!(cross.abs() < 1e-5);
    }

    #[test]
    fn coincident_merge() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let a = eid(20);
        let b = eid(21);
        session
            .apply(Operation::CreateEntity {
                record: line_entity(a, layer, Vec2::ZERO, Vec2::new(2.0, 0.0)),
            })
            .unwrap();
        session
            .apply(Operation::CreateEntity {
                record: line_entity(b, layer, Vec2::new(2.5, 0.5), Vec2::new(4.0, 0.5)),
            })
            .unwrap();

        let mut sys = ConstraintSystem::new();
        sys.add(Constraint::Coincident {
            a,
            a_end: Endpoint::End,
            b,
            b_end: Endpoint::Start,
        })
        .unwrap();
        let result = solve(&sys, &session.document).unwrap();
        session.apply(Operation::Batch { ops: result.ops }).unwrap();
        let Geometry::Line(la) = &session.document.get_entity(a).unwrap().geometry else {
            panic!();
        };
        let Geometry::Line(lb) = &session.document.get_entity(b).unwrap().geometry else {
            panic!();
        };
        assert!(la.end.nearly_eq(lb.start, Tolerance::COARSE));
    }

    #[test]
    fn inconsistent_distances() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let id = eid(30);
        session
            .apply(Operation::CreateEntity {
                record: line_entity(id, layer, Vec2::ZERO, Vec2::new(1.0, 0.0)),
            })
            .unwrap();
        let mut sys = ConstraintSystem::new();
        let id1 = ConstraintId(Uuid::from_u128(1));
        let id2 = ConstraintId(Uuid::from_u128(2));
        sys.insert(
            id1,
            Constraint::Distance {
                entity: id,
                value: 2.0,
            },
        )
        .unwrap();
        sys.insert(
            id2,
            Constraint::Distance {
                entity: id,
                value: 5.0,
            },
        )
        .unwrap();
        let result = solve(&sys, &session.document).unwrap();
        assert_eq!(result.status, SolveStatus::Inconsistent);
        assert!(result.ops.is_empty());
    }

    #[test]
    fn inconsistent_hv_distance() {
        let mut session = DocSession::new();
        let layer = session.document.default_layer_id();
        let id = eid(40);
        session
            .apply(Operation::CreateEntity {
                record: line_entity(id, layer, Vec2::ZERO, Vec2::new(3.0, 4.0)),
            })
            .unwrap();
        let mut sys = ConstraintSystem::new();
        sys.add(Constraint::Horizontal { entity: id }).unwrap();
        sys.add(Constraint::Vertical { entity: id }).unwrap();
        sys.add(Constraint::Distance {
            entity: id,
            value: 5.0,
        })
        .unwrap();
        let result = solve(&sys, &session.document).unwrap();
        assert_eq!(result.status, SolveStatus::Inconsistent);
        assert!(result.ops.is_empty());
    }
}

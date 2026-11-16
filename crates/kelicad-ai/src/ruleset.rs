//! Declared compliance rulesets (deterministic; never LLM).

use kelicad_doc::{Document, Geometry};
use kelicad_geom::Line;
use kelicad_math::Tolerance;
use serde::{Deserialize, Serialize};

/// One finding from a ruleset run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckFinding {
    /// Rule id.
    pub rule: String,
    /// Severity.
    pub severity: String,
    /// Human message (EN).
    pub message: String,
    /// Related entity ids (strings).
    pub entity_ids: Vec<String>,
}

/// Result of running a ruleset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckReport {
    /// Ruleset id.
    pub ruleset: String,
    /// Findings.
    pub findings: Vec<CheckFinding>,
    /// True when no findings.
    pub ok: bool,
}

/// Rule definition (JSON).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleDef {
    /// Warn if document has zero entities.
    EmptyDocument,
    /// Open polyline with at least `min_vertices` verts.
    OpenPolyline {
        /// Minimum vertices.
        #[serde(default = "default_min_verts")]
        min_vertices: usize,
    },
    /// Line shorter than `min_length`.
    MinSegmentLength {
        /// Minimum length.
        min_length: f64,
    },
    /// Nearly duplicate lines (same endpoints within tolerance).
    DuplicateLine {
        /// Absolute distance tolerance.
        #[serde(default = "default_dup_tol")]
        tolerance: f64,
    },
    /// Layer name must be in whitelist.
    LayerWhitelist {
        /// Allowed layer names.
        allowed: Vec<String>,
    },
    /// Minimum clear distance between non-connected line midpoints (PMR-style).
    MinClearance {
        /// Required clearance.
        min_distance: f64,
    },
}

fn default_min_verts() -> usize {
    3
}
fn default_dup_tol() -> f64 {
    1e-6
}

/// Named ruleset.
#[derive(Debug, Clone, Deserialize)]
pub struct Ruleset {
    /// Id.
    pub id: String,
    /// Rules.
    pub rules: Vec<RuleDef>,
}

/// Built-in default ruleset JSON.
pub const DEFAULT_RULESET_JSON: &str = r#"{
  "id": "default",
  "rules": [
    { "type": "empty_document" },
    { "type": "open_polyline", "min_vertices": 3 },
    { "type": "min_segment_length", "min_length": 0.01 },
    { "type": "duplicate_line", "tolerance": 1e-6 }
  ]
}"#;

/// PMR-ish distance ruleset.
pub const PMR_RULESET_JSON: &str = r#"{
  "id": "pmr_distances",
  "rules": [
    { "type": "empty_document" },
    { "type": "min_clearance", "min_distance": 0.90 },
    { "type": "min_segment_length", "min_length": 0.05 }
  ]
}"#;

/// Load a built-in ruleset by name (`default`, `basic`, `pmr`, `pmr_distances`).
pub fn load_ruleset(name: &str) -> Result<Ruleset, String> {
    let json = match name {
        "default" | "basic" | "" => DEFAULT_RULESET_JSON,
        "pmr" | "pmr_distances" => PMR_RULESET_JSON,
        other => return Err(format!("unknown ruleset '{other}'")),
    };
    serde_json::from_str(json).map_err(|e| e.to_string())
}

/// Parse ruleset JSON.
pub fn parse_ruleset(json: &str) -> Result<Ruleset, String> {
    serde_json::from_str(json).map_err(|e| e.to_string())
}

/// Run ruleset against document.
pub fn run_ruleset(ruleset: &Ruleset, doc: &Document) -> CheckReport {
    let mut findings = Vec::new();
    for rule in &ruleset.rules {
        match rule {
            RuleDef::EmptyDocument => {
                if doc.entity_count() == 0 {
                    findings.push(CheckFinding {
                        rule: "empty_document".into(),
                        severity: "warning".into(),
                        message: "document is empty".into(),
                        entity_ids: vec![],
                    });
                }
            }
            RuleDef::OpenPolyline { min_vertices } => {
                for e in doc.entities() {
                    if let Geometry::Polyline(p) = &e.geometry {
                        if !p.closed && p.vertices.len() >= *min_vertices {
                            findings.push(CheckFinding {
                                rule: "open_polyline".into(),
                                severity: "warning".into(),
                                message: format!("open polyline with ≥{} vertices", min_vertices),
                                entity_ids: vec![e.id.to_string()],
                            });
                        }
                    }
                }
            }
            RuleDef::MinSegmentLength { min_length } => {
                for e in doc.entities() {
                    if let Geometry::Line(l) = &e.geometry {
                        if l.length() + f64::EPSILON < *min_length {
                            findings.push(CheckFinding {
                                rule: "min_segment_length".into(),
                                severity: "warning".into(),
                                message: format!("segment length {:.6} < {min_length}", l.length()),
                                entity_ids: vec![e.id.to_string()],
                            });
                        }
                    }
                }
            }
            RuleDef::DuplicateLine { tolerance } => {
                let tol = Tolerance::new(*tolerance);
                let lines: Vec<_> = doc
                    .entities()
                    .filter_map(|e| match &e.geometry {
                        Geometry::Line(l) => Some((e.id, *l)),
                        _ => None,
                    })
                    .collect();
                for i in 0..lines.len() {
                    for j in (i + 1)..lines.len() {
                        if lines_near_duplicate(lines[i].1, lines[j].1, tol) {
                            findings.push(CheckFinding {
                                rule: "duplicate_line".into(),
                                severity: "warning".into(),
                                message: "duplicate or coincident line pair".into(),
                                entity_ids: vec![lines[i].0.to_string(), lines[j].0.to_string()],
                            });
                        }
                    }
                }
            }
            RuleDef::LayerWhitelist { allowed } => {
                for e in doc.entities() {
                    if let Some(layer) = doc.get_layer(e.layer_id) {
                        if !allowed.iter().any(|a| a == &layer.name) {
                            findings.push(CheckFinding {
                                rule: "layer_whitelist".into(),
                                severity: "warning".into(),
                                message: format!("layer '{}' not in whitelist", layer.name),
                                entity_ids: vec![e.id.to_string()],
                            });
                        }
                    }
                }
            }
            RuleDef::MinClearance { min_distance } => {
                let lines: Vec<_> = doc
                    .entities()
                    .filter_map(|e| match &e.geometry {
                        Geometry::Line(l) => Some((e.id, *l)),
                        _ => None,
                    })
                    .collect();
                for i in 0..lines.len() {
                    for j in (i + 1)..lines.len() {
                        let mi = lines[i].1.point_at(0.5);
                        let mj = lines[j].1.point_at(0.5);
                        let d = mi.distance(mj);
                        if d + f64::EPSILON < *min_distance {
                            findings.push(CheckFinding {
                                rule: "min_clearance".into(),
                                severity: "warning".into(),
                                message: format!(
                                    "clearance {d:.3} < {min_distance} between line midpoints"
                                ),
                                entity_ids: vec![lines[i].0.to_string(), lines[j].0.to_string()],
                            });
                        }
                    }
                }
            }
        }
    }
    CheckReport {
        ruleset: ruleset.id.clone(),
        ok: findings.is_empty(),
        findings,
    }
}

fn lines_near_duplicate(a: Line, b: Line, tol: Tolerance) -> bool {
    (a.start.nearly_eq(b.start, tol) && a.end.nearly_eq(b.end, tol))
        || (a.start.nearly_eq(b.end, tol) && a.end.nearly_eq(b.start, tol))
}

#[cfg(test)]
mod tests {
    use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry};
    use kelicad_geom::{Line, Polyline};
    use kelicad_math::Vec2;
    use kelicad_ops::{DocSession, Operation};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn default_flags_open_poly_and_short() {
        let mut s = DocSession::new();
        let layer = s.document.default_layer_id();
        s.apply(Operation::CreateEntity {
            record: EntityRecord {
                id: EntityId::from_uuid(Uuid::from_u128(1)),
                kind: EntityKind::Polyline,
                geometry: Geometry::Polyline(Polyline {
                    vertices: vec![Vec2::ZERO, Vec2::X, Vec2::new(1.0, 1.0)],
                    closed: false,
                }),
                layer_id: layer,
            },
        })
        .unwrap();
        s.apply(Operation::CreateEntity {
            record: EntityRecord {
                id: EntityId::from_uuid(Uuid::from_u128(2)),
                kind: EntityKind::Line,
                geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(0.001, 0.0))),
                layer_id: layer,
                props: Default::default(),
        },
        })
        .unwrap();
        let rs = load_ruleset("default").unwrap();
        let report = run_ruleset(&rs, &s.document);
        assert!(!report.ok);
        assert!(report.findings.iter().any(|f| f.rule == "open_polyline"));
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "min_segment_length"));
    }
}

//! Rigorous AI ruleset / cleanup / safety tests.

use kelicad_doc::{EntityId, EntityKind, EntityRecord, Geometry, Layer, LayerId};
use kelicad_geom::Line;
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use serde_json::json;
use uuid::Uuid;

use crate::id_gen::SequentialIdGen;
use crate::ruleset::{load_ruleset, parse_ruleset, run_ruleset, RuleDef, Ruleset};
use crate::safety::requires_destructive_confirm;
use crate::tools::{execute_tool, ToolCall, ToolContext};

fn eid(n: u128) -> EntityId {
    EntityId::from_uuid(Uuid::from_u128(n))
}

fn add_line(s: &mut DocSession, id: u128, a: Vec2, b: Vec2) {
    let layer = s.document.default_layer_id();
    s.apply(Operation::CreateEntity {
        record: EntityRecord {
            id: eid(id),
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(a, b)),
            layer_id: layer,
            props: Default::default(),
        },
    })
    .unwrap();
}

#[test]
fn unknown_ruleset_errors() {
    assert!(load_ruleset("nope").is_err());
}

#[test]
fn pmr_flags_insufficient_clearance() {
    let mut s = DocSession::new();
    add_line(&mut s, 1, Vec2::ZERO, Vec2::new(1.0, 0.0));
    add_line(&mut s, 2, Vec2::new(0.0, 0.2), Vec2::new(1.0, 0.2));
    let rs = load_ruleset("pmr").unwrap();
    let report = run_ruleset(&rs, &s.document);
    assert!(!report.ok);
    assert!(report.findings.iter().any(|f| f.rule == "min_clearance"));
}

#[test]
fn duplicate_line_rule() {
    let mut s = DocSession::new();
    add_line(&mut s, 1, Vec2::ZERO, Vec2::X);
    add_line(&mut s, 2, Vec2::X, Vec2::ZERO); // reversed duplicate
    let rs = Ruleset {
        id: "dup".into(),
        rules: vec![RuleDef::DuplicateLine { tolerance: 1e-6 }],
    };
    let report = run_ruleset(&rs, &s.document);
    assert!(!report.ok);
    assert_eq!(report.findings.len(), 1);
}

#[test]
fn layer_whitelist_rule() {
    let mut s = DocSession::new();
    let walls = LayerId::from_uuid(Uuid::from_u128(7));
    s.apply(Operation::CreateLayer {
        layer: Layer::new(walls, "Walls", [255, 0, 0]),
    })
    .unwrap();
    s.apply(Operation::CreateEntity {
        record: EntityRecord {
            id: eid(1),
            kind: EntityKind::Line,
            geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::X)),
            layer_id: walls,
            props: Default::default(),
        },
    })
    .unwrap();
    let rs = Ruleset {
        id: "layers".into(),
        rules: vec![RuleDef::LayerWhitelist {
            allowed: vec!["0".into()],
        }],
    };
    let report = run_ruleset(&rs, &s.document);
    assert!(report.findings.iter().any(|f| f.rule == "layer_whitelist"));
}

#[test]
fn cleanup_removes_duplicate() {
    let mut s = DocSession::new();
    add_line(&mut s, 1, Vec2::ZERO, Vec2::new(5.0, 0.0));
    add_line(&mut s, 2, Vec2::ZERO, Vec2::new(5.0, 0.0));
    let ctx = ToolContext {
        session: &s,
        selection: &[],
        view_center: Vec2::ZERO,
        view_height: 10.0,
    };
    let call = ToolCall {
        name: "cleanup_drawing".into(),
        args: json!({}),
    };
    let mut ids = SequentialIdGen::new(1000);
    let result = execute_tool(&call, &ctx, &mut ids).unwrap();
    assert!(result.ops.iter().any(|op| matches!(
        op,
        Operation::DeleteEntity { id } if *id == eid(2) || *id == eid(1)
    )));
}

#[test]
fn auto_dimension_produces_ops() {
    let mut s = DocSession::new();
    add_line(&mut s, 1, Vec2::ZERO, Vec2::new(10.0, 0.0));
    let ctx = ToolContext {
        session: &s,
        selection: &[],
        view_center: Vec2::new(5.0, 0.0),
        view_height: 20.0,
    };
    let call = ToolCall {
        name: "auto_dimension".into(),
        args: json!({}),
    };
    let mut ids = SequentialIdGen::new(2000);
    let result = execute_tool(&call, &ctx, &mut ids).unwrap();
    assert!(!result.ops.is_empty());
}

#[test]
fn delete_requires_destructive_confirm() {
    assert!(!requires_destructive_confirm(1));
    assert!(!requires_destructive_confirm(20));
    assert!(requires_destructive_confirm(21));
}

#[test]
fn parse_ruleset_rejects_garbage() {
    assert!(parse_ruleset("{not json").is_err());
}

#[test]
fn empty_document_finding() {
    let s = DocSession::new();
    let rs = load_ruleset("default").unwrap();
    let report = run_ruleset(&rs, &s.document);
    assert!(report.findings.iter().any(|f| f.rule == "empty_document"));
}

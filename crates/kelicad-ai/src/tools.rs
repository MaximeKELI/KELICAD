//! Tool registry, JSON schema checks, and execution → ops / read results.

use std::collections::HashMap;
use std::f64::consts::PI;

use kelicad_doc::{
    BlockDef, DimKind, DimensionGeom, EntityId, EntityRecord, Geometry, HatchGeom, HatchPattern,
    InsertGeom, TextAlign, TextGeom,
};
use kelicad_geom::transform::offset_line;
use kelicad_geom::{Arc, Circle, Line, Polyline};
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use serde_json::{json, Value};

use crate::error::AiError;
use crate::id_gen::IdGen;
use crate::plan::PlanStep;
use crate::safety::requires_destructive_confirm;

/// Named tool call from a planner / LLM.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Tool name.
    pub name: String,
    /// Arguments object.
    pub args: Value,
}

/// Result of executing one tool against a session (may only plan writes).
#[derive(Debug, Clone)]
pub struct ToolExecResult {
    /// UI plan step.
    pub step: PlanStep,
    /// Forward operations (empty for reads).
    pub ops: Vec<Operation>,
    /// Ghost geometries.
    pub ghosts: Vec<Geometry>,
    /// Bindings: logical name → entity id (e.g. "last").
    pub bindings: HashMap<String, EntityId>,
}

/// Document / selection / view context for tools.
#[derive(Debug, Clone)]
pub struct ToolContext<'a> {
    /// Live session (read; writes go through planned ops).
    pub session: &'a DocSession,
    /// Current selection ids.
    pub selection: &'a [EntityId],
    /// View center.
    pub view_center: Vec2,
    /// View height in world units.
    pub view_height: f64,
}

/// Execute a validated tool call.
pub fn execute_tool(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    match call.name.as_str() {
        "query_entities" => read_query_entities(call, ctx),
        "get_selection" => read_get_selection(call, ctx),
        "measure" => read_measure(call, ctx),
        "list_layers" => read_list_layers(call, ctx),
        "list_blocks" => read_list_blocks(call, ctx),
        "get_view" => read_get_view(call, ctx),
        "summarize_document" => read_summarize(call, ctx),
        "draw_line" => write_draw_line(call, ctx, ids),
        "draw_polyline" => write_draw_polyline(call, ctx, ids),
        "draw_circle" => write_draw_circle(call, ctx, ids),
        "draw_arc" => write_draw_arc(call, ctx, ids),
        "draw_rectangle" => write_draw_rectangle(call, ctx, ids),
        "draw_text" => write_draw_text(call, ctx, ids),
        "add_dimension" => write_add_dimension(call, ctx, ids),
        "hatch_region" => write_hatch(call, ctx, ids),
        "move" => write_move(call, ctx),
        "copy" => write_copy(call, ctx, ids),
        "rotate" => write_rotate(call, ctx),
        "scale" => write_scale(call, ctx),
        "offset" => write_offset(call, ctx, ids),
        "set_layer" => write_set_layer(call, ctx),
        "create_block" => write_create_block(call, ctx, ids),
        "insert_block" => write_insert_block(call, ctx, ids),
        "delete_entities" => write_delete(call, ctx),
        "highlight" => control_highlight(call, ctx),
        "propose_plan" => control_propose_plan(call),
        "run_check" => control_run_check(call, ctx),
        other => Err(AiError::UnknownTool(other.to_string())),
    }
}

/// List registered tool names.
pub fn tool_names() -> &'static [&'static str] {
    &[
        "query_entities",
        "get_selection",
        "measure",
        "list_layers",
        "list_blocks",
        "get_view",
        "summarize_document",
        "draw_line",
        "draw_polyline",
        "draw_circle",
        "draw_arc",
        "draw_rectangle",
        "draw_text",
        "add_dimension",
        "hatch_region",
        "move",
        "copy",
        "rotate",
        "scale",
        "offset",
        "set_layer",
        "create_block",
        "insert_block",
        "delete_entities",
        "highlight",
        "propose_plan",
        "run_check",
    ]
}

fn step(
    tool: &str,
    args: &Value,
    fr: &str,
    en: &str,
    read_only: bool,
    read_result: Option<Value>,
) -> PlanStep {
    PlanStep {
        tool: tool.to_string(),
        args: args.clone(),
        summary_fr: fr.to_string(),
        summary_en: en.to_string(),
        read_only,
        read_result,
    }
}

fn req_array2(tool: &str, args: &Value, key: &str) -> Result<Vec2, AiError> {
    let arr = args
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("missing {key} [x,y]"),
        })?;
    if arr.len() != 2 {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("{key} must have 2 numbers"),
        });
    }
    let x = arr[0].as_f64().ok_or_else(|| AiError::InvalidArgs {
        tool: tool.into(),
        reason: format!("{key}[0] not a number"),
    })?;
    let y = arr[1].as_f64().ok_or_else(|| AiError::InvalidArgs {
        tool: tool.into(),
        reason: format!("{key}[1] not a number"),
    })?;
    Ok(Vec2::new(x, y))
}

fn opt_array2(args: &Value, key: &str) -> Option<Vec2> {
    let arr = args.get(key)?.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    Some(Vec2::new(arr[0].as_f64()?, arr[1].as_f64()?))
}

fn req_f64(tool: &str, args: &Value, key: &str) -> Result<f64, AiError> {
    args.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("missing number {key}"),
        })
}

fn req_str(tool: &str, args: &Value, key: &str) -> Result<String, AiError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("missing string {key}"),
        })
}

fn resolve_layer(ctx: &ToolContext<'_>, args: &Value) -> kelicad_doc::LayerId {
    if let Some(name) = args.get("layer").and_then(|v| v.as_str()) {
        for layer in ctx.session.document.layers() {
            if layer.name.eq_ignore_ascii_case(name) {
                return layer.id;
            }
        }
    }
    ctx.session.document.current_layer
}

fn parse_entity_ids(tool: &str, args: &Value, key: &str) -> Result<Vec<EntityId>, AiError> {
    let arr = args
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("missing {key} array"),
        })?;
    let mut out = Vec::new();
    for v in arr {
        let s = v.as_str().ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("{key} entries must be strings"),
        })?;
        let u = uuid::Uuid::parse_str(s).map_err(|_| AiError::InvalidArgs {
            tool: tool.into(),
            reason: format!("invalid uuid {s}"),
        })?;
        out.push(EntityId::from_uuid(u));
    }
    Ok(out)
}

fn entity_ids_or_selection(
    tool: &str,
    args: &Value,
    ctx: &ToolContext<'_>,
) -> Result<Vec<EntityId>, AiError> {
    if args.get("entity_ids").is_some() {
        parse_entity_ids(tool, args, "entity_ids")
    } else if !ctx.selection.is_empty() {
        Ok(ctx.selection.to_vec())
    } else {
        // Fall back to all entities when selection empty (common for AI plans).
        Ok(ctx.session.document.entities().map(|e| e.id).collect())
    }
}

fn create_entity_op(id: EntityId, layer: kelicad_doc::LayerId, geometry: Geometry) -> Operation {
    Operation::CreateEntity {
        record: EntityRecord {
            id,
            kind: geometry.kind(),
            geometry,
            layer_id: layer,
        },
    }
}

// —— Read tools ——

fn read_query_entities(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let kind_filter = call
        .args
        .get("kind")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase());
    let mut list = Vec::new();
    for e in ctx.session.document.entities() {
        let kind = format!("{:?}", e.kind).to_ascii_lowercase();
        if let Some(ref f) = kind_filter {
            if &kind != f {
                continue;
            }
        }
        list.push(json!({
            "id": e.id.to_string(),
            "kind": kind,
            "layer_id": e.layer_id.to_string(),
        }));
    }
    let result = json!({ "entities": list, "count": list.len() });
    Ok(ToolExecResult {
        step: step(
            &call.name,
            &call.args,
            &format!("{} entité(s)", list.len()),
            &format!("{} entit(y/ies)", list.len()),
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_get_selection(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let ids: Vec<String> = ctx.selection.iter().map(|id| id.to_string()).collect();
    let result = json!({ "selection": ids });
    Ok(ToolExecResult {
        step: step(
            &call.name,
            &call.args,
            &format!("sélection: {}", ids.len()),
            &format!("selection: {}", ids.len()),
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_measure(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "measure";
    let mode = call
        .args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("distance");
    let result = match mode {
        "distance" => {
            let a = req_array2(tool, &call.args, "a")?;
            let b = req_array2(tool, &call.args, "b")?;
            let d = a.distance(b);
            json!({ "mode": "distance", "value": d, "unit": "drawing" })
        }
        "entity" => {
            let ids = parse_entity_ids(tool, &call.args, "entity_ids")?;
            let id = ids.first().ok_or_else(|| AiError::InvalidArgs {
                tool: tool.into(),
                reason: "entity_ids empty".into(),
            })?;
            let ent = ctx
                .session
                .document
                .get_entity(*id)
                .ok_or_else(|| AiError::Execution(format!("entity {id} not found")))?;
            match &ent.geometry {
                Geometry::Line(l) => json!({"kind":"line","length": l.length()}),
                Geometry::Circle(c) => {
                    json!({"kind":"circle","radius": c.radius, "area": PI * c.radius * c.radius})
                }
                Geometry::Polyline(p) => {
                    let mut len = 0.0;
                    for i in 0..p.segment_count() {
                        if let Some(seg) = p.segment(i) {
                            len += seg.length();
                        }
                    }
                    json!({"kind":"polyline","length": len, "closed": p.closed})
                }
                Geometry::Dimension(d) => {
                    json!({"kind":"dimension","measured": d.measured_length(), "text": d.display_text()})
                }
                other => {
                    json!({"kind": format!("{:?}", other.kind()), "note": "no scalar measure"})
                }
            }
        }
        other => {
            return Err(AiError::InvalidArgs {
                tool: tool.into(),
                reason: format!("unknown mode {other}"),
            })
        }
    };
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            "mesure (noyau)",
            "measure (core)",
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_list_layers(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let layers: Vec<Value> = ctx
        .session
        .document
        .layers()
        .map(|l| {
            json!({
                "id": l.id.to_string(),
                "name": l.name,
                "color": l.color,
            })
        })
        .collect();
    let result = json!({ "layers": layers });
    Ok(ToolExecResult {
        step: step(
            &call.name,
            &call.args,
            &format!("{} calque(s)", layers.len()),
            &format!("{} layer(s)", layers.len()),
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_list_blocks(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let blocks: Vec<Value> = ctx
        .session
        .document
        .blocks()
        .map(|b| {
            json!({
                "id": b.id.to_string(),
                "name": b.name,
                "entity_count": b.entities.len(),
            })
        })
        .collect();
    let result = json!({ "blocks": blocks });
    Ok(ToolExecResult {
        step: step(
            &call.name,
            &call.args,
            &format!("{} bloc(s)", blocks.len()),
            &format!("{} block(s)", blocks.len()),
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_get_view(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let result = json!({
        "center": [ctx.view_center.x, ctx.view_center.y],
        "height": ctx.view_height,
    });
    Ok(ToolExecResult {
        step: step(&call.name, &call.args, "vue", "view", true, Some(result)),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn read_summarize(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for e in ctx.session.document.entities() {
        *by_kind
            .entry(format!("{:?}", e.kind).to_ascii_lowercase())
            .or_default() += 1;
    }
    let result = json!({
        "entity_count": ctx.session.document.entity_count(),
        "layer_count": ctx.session.document.layers().count(),
        "by_kind": by_kind,
        "current_layer": ctx.session.document.current_layer.to_string(),
    });
    Ok(ToolExecResult {
        step: step(
            &call.name,
            &call.args,
            "résumé du document",
            "document summary",
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

// —— Write tools ——

fn write_draw_line(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_line";
    let start = req_array2(tool, &call.args, "start")?;
    let end = req_array2(tool, &call.args, "end")?;
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Line(Line::new(start, end));
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!(
                "ligne ({:.3},{:.3})→({:.3},{:.3})",
                start.x, start.y, end.x, end.y
            ),
            &format!(
                "line ({:.3},{:.3})→({:.3},{:.3})",
                start.x, start.y, end.x, end.y
            ),
            false,
            None,
        ),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_draw_polyline(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_polyline";
    let pts = call
        .args
        .get("points")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: "missing points".into(),
        })?;
    let mut vertices = Vec::new();
    for p in pts {
        let arr = p.as_array().ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: "point must be [x,y]".into(),
        })?;
        if arr.len() != 2 {
            return Err(AiError::InvalidArgs {
                tool: tool.into(),
                reason: "point must be [x,y]".into(),
            });
        }
        vertices.push(Vec2::new(
            arr[0].as_f64().ok_or_else(|| AiError::InvalidArgs {
                tool: tool.into(),
                reason: "x not number".into(),
            })?,
            arr[1].as_f64().ok_or_else(|| AiError::InvalidArgs {
                tool: tool.into(),
                reason: "y not number".into(),
            })?,
        ));
    }
    if vertices.len() < 2 {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: "need ≥2 points".into(),
        });
    }
    let closed = call
        .args
        .get("closed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let poly = if closed {
        Polyline::closed(vertices)
    } else {
        Polyline::new(vertices)
    };
    let geom = Geometry::Polyline(poly);
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(tool, &call.args, "polyligne", "polyline", false, None),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_draw_circle(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_circle";
    let center = req_array2(tool, &call.args, "center")?;
    let radius = req_f64(tool, &call.args, "radius")?;
    if radius <= 0.0 {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: "radius must be > 0".into(),
        });
    }
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Circle(Circle::new(center, radius));
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("cercle r={radius:.3}"),
            &format!("circle r={radius:.3}"),
            false,
            None,
        ),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_draw_arc(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_arc";
    let center = req_array2(tool, &call.args, "center")?;
    let radius = req_f64(tool, &call.args, "radius")?;
    let start_angle = req_f64(tool, &call.args, "start_angle")?;
    let end_angle = req_f64(tool, &call.args, "end_angle")?;
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Arc(Arc::new(center, radius, start_angle, end_angle));
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(tool, &call.args, "arc", "arc", false, None),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_draw_rectangle(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_rectangle";
    let width = req_f64(tool, &call.args, "width")?;
    let height = req_f64(tool, &call.args, "height")?;
    if width <= 0.0 || height <= 0.0 {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: "width/height must be > 0".into(),
        });
    }
    let origin = opt_array2(&call.args, "origin").unwrap_or(Vec2::ZERO);
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let opposite = origin + Vec2::new(width, height);
    let geom = Geometry::Polyline(Polyline::rectangle(origin, opposite));
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("rectangle {width:.3}×{height:.3}"),
            &format!("rectangle {width:.3}×{height:.3}"),
            false,
            None,
        ),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_draw_text(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "draw_text";
    let position = req_array2(tool, &call.args, "position")?;
    let content = req_str(tool, &call.args, "content")?;
    let height = call
        .args
        .get("height")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.25);
    let rotation = call
        .args
        .get("rotation")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Text(TextGeom {
        position,
        height,
        rotation,
        content,
        align: TextAlign::Left,
    });
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(tool, &call.args, "texte", "text", false, None),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_add_dimension(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "add_dimension";
    let (def1, def2, dim_line) = if let (Some(a), Some(b)) = (
        opt_array2(&call.args, "def1"),
        opt_array2(&call.args, "def2"),
    ) {
        let dim_line = opt_array2(&call.args, "dim_line").unwrap_or_else(|| {
            let mid = (a + b) * 0.5;
            mid + Vec2::new(0.0, 0.5)
        });
        (a, b, dim_line)
    } else if let Some(eid) = call.args.get("entity_id").and_then(|v| v.as_str()) {
        let id =
            EntityId::from_uuid(
                uuid::Uuid::parse_str(eid).map_err(|_| AiError::InvalidArgs {
                    tool: tool.into(),
                    reason: "bad entity_id".into(),
                })?,
            );
        let ent = ctx
            .session
            .document
            .get_entity(id)
            .ok_or_else(|| AiError::Execution(format!("entity {id} not found")))?;
        match &ent.geometry {
            Geometry::Line(l) => {
                let mid = (l.start + l.end) * 0.5;
                (l.start, l.end, mid + Vec2::new(0.0, 0.5))
            }
            Geometry::Polyline(p) if p.vertices.len() >= 2 => {
                // Dimension first edge (width of rectangle).
                let a = p.vertices[0];
                let b = p.vertices[1];
                let mid = (a + b) * 0.5;
                let offset = call
                    .args
                    .get("offset")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5);
                (a, b, mid + Vec2::new(0.0, offset))
            }
            _ => {
                return Err(AiError::Execution(
                    "add_dimension: entity has no linear edge".into(),
                ))
            }
        }
    } else {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: "need def1/def2 or entity_id".into(),
        });
    };
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let dim = DimensionGeom {
        kind: DimKind::Linear,
        def1,
        def2,
        dim_line,
        assoc_a: None,
        assoc_b: None,
        text_override: None,
    };
    let measured = dim.measured_length();
    let geom = Geometry::Dimension(dim);
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("cote {measured:.3}"),
            &format!("dimension {measured:.3}"),
            false,
            Some(json!({"measured": measured})),
        ),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_hatch(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "hatch_region";
    let pts = call
        .args
        .get("boundary")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: "missing boundary".into(),
        })?;
    let mut boundary = Vec::new();
    for p in pts {
        let arr = p.as_array().ok_or_else(|| AiError::InvalidArgs {
            tool: tool.into(),
            reason: "boundary point [x,y]".into(),
        })?;
        boundary.push(Vec2::new(
            arr[0].as_f64().unwrap_or(0.0),
            arr[1].as_f64().unwrap_or(0.0),
        ));
    }
    if boundary.len() < 3 {
        return Err(AiError::InvalidArgs {
            tool: tool.into(),
            reason: "boundary needs ≥3 points".into(),
        });
    }
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Hatch(HatchGeom {
        boundary,
        pattern: HatchPattern::Solid,
    });
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(tool, &call.args, "hachure", "hatch", false, None),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_move(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "move";
    let delta = req_array2(tool, &call.args, "delta")?;
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut ops = Vec::new();
    let mut ghosts = Vec::new();
    for id in &ids {
        let Some(ent) = ctx.session.document.get_entity(*id) else {
            continue;
        };
        let new_geom = ent.geometry.translated(delta);
        ghosts.push(new_geom.clone());
        ops.push(Operation::SetGeometry {
            id: *id,
            geometry: new_geom,
        });
    }
    if ops.is_empty() {
        return Err(AiError::Execution("move: no entities".into()));
    }
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("déplacer Δ=({:.3},{:.3})", delta.x, delta.y),
            &format!("move Δ=({:.3},{:.3})", delta.x, delta.y),
            false,
            None,
        ),
        ops,
        ghosts,
        bindings: HashMap::new(),
    })
}

fn write_copy(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    id_gen: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "copy";
    let delta = req_array2(tool, &call.args, "delta")?;
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut ops = Vec::new();
    let mut ghosts = Vec::new();
    let mut last = None;
    for id in &ids {
        let Some(ent) = ctx.session.document.get_entity(*id) else {
            continue;
        };
        let new_id = id_gen.next_entity();
        let geom = ent.geometry.translated(delta);
        ghosts.push(geom.clone());
        ops.push(create_entity_op(new_id, ent.layer_id, geom));
        last = Some(new_id);
    }
    let mut bindings = HashMap::new();
    if let Some(id) = last {
        bindings.insert("last".into(), id);
    }
    if ops.is_empty() {
        return Err(AiError::Execution("copy: no entities".into()));
    }
    Ok(ToolExecResult {
        step: step(tool, &call.args, "copier", "copy", false, None),
        ops,
        ghosts,
        bindings,
    })
}

fn write_rotate(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "rotate";
    let origin = req_array2(tool, &call.args, "origin")?;
    let degrees = req_f64(tool, &call.args, "degrees")?;
    let radians = degrees.to_radians();
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut ops = Vec::new();
    let mut ghosts = Vec::new();
    for id in &ids {
        let Some(ent) = ctx.session.document.get_entity(*id) else {
            continue;
        };
        let new_geom = ent.geometry.rotated(origin, radians);
        ghosts.push(new_geom.clone());
        ops.push(Operation::SetGeometry {
            id: *id,
            geometry: new_geom,
        });
    }
    if ops.is_empty() {
        return Err(AiError::Execution("rotate: no entities".into()));
    }
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("rotation {degrees:.1}°"),
            &format!("rotate {degrees:.1}°"),
            false,
            None,
        ),
        ops,
        ghosts,
        bindings: HashMap::new(),
    })
}

fn write_scale(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "scale";
    let origin = req_array2(tool, &call.args, "origin")?;
    let factor = req_f64(tool, &call.args, "factor")?;
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut ops = Vec::new();
    let mut ghosts = Vec::new();
    for id in &ids {
        let Some(ent) = ctx.session.document.get_entity(*id) else {
            continue;
        };
        let new_geom = ent.geometry.scaled(origin, factor);
        ghosts.push(new_geom.clone());
        ops.push(Operation::SetGeometry {
            id: *id,
            geometry: new_geom,
        });
    }
    if ops.is_empty() {
        return Err(AiError::Execution("scale: no entities".into()));
    }
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("échelle ×{factor:.3}"),
            &format!("scale ×{factor:.3}"),
            false,
            None,
        ),
        ops,
        ghosts,
        bindings: HashMap::new(),
    })
}

fn write_offset(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    id_gen: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "offset";
    let distance = req_f64(tool, &call.args, "distance")?;
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut ops = Vec::new();
    let mut ghosts = Vec::new();
    let mut last = None;
    for id in &ids {
        let Some(ent) = ctx.session.document.get_entity(*id) else {
            continue;
        };
        let Geometry::Line(l) = &ent.geometry else {
            continue;
        };
        let Some(ol) = offset_line(*l, distance) else {
            continue;
        };
        let new_id = id_gen.next_entity();
        let geom = Geometry::Line(ol);
        ghosts.push(geom.clone());
        ops.push(create_entity_op(new_id, ent.layer_id, geom));
        last = Some(new_id);
    }
    if ops.is_empty() {
        return Err(AiError::Execution("offset: need line entities".into()));
    }
    let mut bindings = HashMap::new();
    if let Some(id) = last {
        bindings.insert("last".into(), id);
    }
    Ok(ToolExecResult {
        step: step(tool, &call.args, "offset", "offset", false, None),
        ops,
        ghosts,
        bindings,
    })
}

fn write_set_layer(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "set_layer";
    let name = req_str(tool, &call.args, "layer")?;
    let layer_id = ctx
        .session
        .document
        .layers()
        .find(|l| l.name.eq_ignore_ascii_case(&name))
        .map(|l| l.id);
    let mut ops = Vec::new();
    let layer_id = if let Some(id) = layer_id {
        id
    } else {
        // Create layer if missing — needs id gen; use nil-derived from name hash.
        return Err(AiError::Execution(format!(
            "layer '{name}' not found (create layers via document UI for M3)"
        )));
    };
    let entity_ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    for id in entity_ids {
        if ctx.session.document.get_entity(id).is_some() {
            ops.push(Operation::SetEntityLayer { id, layer_id });
        }
    }
    // Also set current layer.
    ops.push(Operation::SetCurrentLayer { id: layer_id });
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("calque {name}"),
            &format!("layer {name}"),
            false,
            None,
        ),
        ops,
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn write_create_block(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "create_block";
    let name = req_str(tool, &call.args, "name")?;
    let entity_ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let mut geoms = Vec::new();
    for id in &entity_ids {
        if let Some(ent) = ctx.session.document.get_entity(*id) {
            geoms.push(ent.geometry.clone());
        }
    }
    if geoms.is_empty() {
        return Err(AiError::Execution("create_block: no geometry".into()));
    }
    let block_id = ids.next_block();
    let def = BlockDef {
        id: block_id,
        name: name.clone(),
        origin: Vec2::ZERO,
        entities: geoms,
        attributes: vec![],
    };
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("bloc {name}"),
            &format!("block {name}"),
            false,
            None,
        ),
        ops: vec![Operation::UpsertBlock { def }],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn write_insert_block(
    call: &ToolCall,
    ctx: &ToolContext<'_>,
    ids: &mut dyn IdGen,
) -> Result<ToolExecResult, AiError> {
    let tool = "insert_block";
    let name = req_str(tool, &call.args, "name")?;
    let position = req_array2(tool, &call.args, "position")?;
    let block = ctx
        .session
        .document
        .block_by_name(&name)
        .ok_or_else(|| AiError::Execution(format!("block '{name}' not found")))?;
    let layer = resolve_layer(ctx, &call.args);
    let id = ids.next_entity();
    let geom = Geometry::Insert(InsertGeom {
        block_id: block.id,
        position,
        scale: Vec2::new(1.0, 1.0),
        rotation: 0.0,
        attribs: HashMap::new(),
    });
    let mut bindings = HashMap::new();
    bindings.insert("last".into(), id);
    Ok(ToolExecResult {
        step: step(tool, &call.args, "insert", "insert", false, None),
        ops: vec![create_entity_op(id, layer, geom.clone())],
        ghosts: vec![geom],
        bindings,
    })
}

fn write_delete(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let tool = "delete_entities";
    let ids = entity_ids_or_selection(tool, &call.args, ctx)?;
    let confirm = call
        .args
        .get("confirm_destructive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if requires_destructive_confirm(ids.len()) && !confirm {
        return Err(AiError::DestructiveBlocked(format!(
            "delete {} entities",
            ids.len()
        )));
    }
    let ops: Vec<_> = ids
        .iter()
        .filter(|id| ctx.session.document.get_entity(**id).is_some())
        .map(|id| Operation::DeleteEntity { id: *id })
        .collect();
    if ops.is_empty() {
        return Err(AiError::Execution("delete: nothing to delete".into()));
    }
    Ok(ToolExecResult {
        step: step(
            tool,
            &call.args,
            &format!("supprimer {}", ops.len()),
            &format!("delete {}", ops.len()),
            false,
            None,
        ),
        ops,
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn control_highlight(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let ids = if call.args.get("entity_ids").is_some() {
        parse_entity_ids("highlight", &call.args, "entity_ids")?
    } else {
        ctx.selection.to_vec()
    };
    let mut ghosts = Vec::new();
    for id in &ids {
        if let Some(ent) = ctx.session.document.get_entity(*id) {
            ghosts.push(ent.geometry.clone());
        }
    }
    Ok(ToolExecResult {
        step: step(
            "highlight",
            &call.args,
            &format!("surligner {}", ids.len()),
            &format!("highlight {}", ids.len()),
            true,
            Some(json!({"ids": ids.iter().map(|i| i.to_string()).collect::<Vec<_>>()})),
        ),
        ops: vec![],
        ghosts,
        bindings: HashMap::new(),
    })
}

fn control_propose_plan(call: &ToolCall) -> Result<ToolExecResult, AiError> {
    let steps = call.args.get("steps").cloned().unwrap_or_else(|| json!([]));
    Ok(ToolExecResult {
        step: step(
            "propose_plan",
            &call.args,
            "plan proposé",
            "proposed plan",
            true,
            Some(steps),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

fn control_run_check(call: &ToolCall, ctx: &ToolContext<'_>) -> Result<ToolExecResult, AiError> {
    let mut warnings = Vec::new();
    if ctx.session.document.entity_count() == 0 {
        warnings.push("document is empty");
    }
    for e in ctx.session.document.entities() {
        if let Geometry::Polyline(p) = &e.geometry {
            if !p.closed && p.vertices.len() >= 3 {
                warnings.push("open polyline with ≥3 vertices");
                break;
            }
        }
    }
    let result = json!({ "warnings": warnings, "ok": warnings.is_empty() });
    Ok(ToolExecResult {
        step: step(
            "run_check",
            &call.args,
            if warnings.is_empty() {
                "contrôle OK"
            } else {
                "avertissements"
            },
            if warnings.is_empty() {
                "check OK"
            } else {
                "warnings"
            },
            true,
            Some(result),
        ),
        ops: vec![],
        ghosts: vec![],
        bindings: HashMap::new(),
    })
}

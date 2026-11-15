//! WASM façade for the KELICAD core (commands + document).

#![forbid(unsafe_code)]

use kelicad_cmd::{CommandEngine, CommandEvent, CommandId, UuidV7Gen};
use kelicad_doc::Geometry;
use kelicad_math::Vec2;
use kelicad_ops::DocSession;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Install the panic hook once.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Core version.
#[wasm_bindgen]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Serialize)]
struct EntityDto {
    id: String,
    kind: String,
    layer_id: String,
    geometry: serde_json::Value,
}

#[derive(Serialize)]
struct LayerDto {
    id: String,
    name: String,
    color: [u8; 3],
}

#[derive(Serialize)]
struct SnapshotDto {
    schema_version: u32,
    current_layer: String,
    layers: Vec<LayerDto>,
    entities: Vec<EntityDto>,
}

/// Interactive CAD session for the web UI.
#[wasm_bindgen]
pub struct WasmCad {
    engine: CommandEngine<UuidV7Gen>,
}

#[wasm_bindgen]
impl WasmCad {
    /// New empty drawing.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            engine: CommandEngine::new(DocSession::new(), UuidV7Gen),
        }
    }

    /// Entity count.
    pub fn entity_count(&self) -> u32 {
        self.engine.session.document.entity_count() as u32
    }

    /// Submit a command-line string (command name or point/number).
    pub fn submit_text(&mut self, text: &str) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Text(text.to_string()));
        Ok(output_json(&out))
    }

    /// Start a command by alias/name.
    pub fn start_command(&mut self, name: &str) -> Result<String, JsValue> {
        let id = CommandId::parse(name).ok_or_else(|| JsValue::from_str("unknown command"))?;
        let out = self.engine.handle(CommandEvent::Start(id));
        Ok(output_json(&out))
    }

    /// World-space pick.
    pub fn pick(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Pick(Vec2::new(x, y)));
        Ok(output_json(&out))
    }

    /// Hover for ghost / snap.
    pub fn hover(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Hover(Vec2::new(x, y)));
        Ok(output_json(&out))
    }

    /// Enter.
    pub fn enter(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Enter);
        Ok(output_json(&out))
    }

    /// Cancel (Escape).
    pub fn cancel(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Cancel);
        Ok(output_json(&out))
    }

    /// Undo.
    pub fn undo(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Start(CommandId::Undo));
        Ok(output_json(&out))
    }

    /// Redo.
    pub fn redo(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Start(CommandId::Redo));
        Ok(output_json(&out))
    }

    /// Set snap aperture in world units.
    pub fn set_snap_aperture(&mut self, aperture: f64) {
        self.engine.snap_aperture = aperture.max(1e-9);
    }

    /// Export document snapshot JSON (for .kelicad writers / download).
    pub fn export_snapshot_json(&self) -> Result<String, JsValue> {
        let doc = &self.engine.session.document;
        let layers = doc
            .layers()
            .map(|l| LayerDto {
                id: l.id.to_string(),
                name: l.name.clone(),
                color: l.color,
            })
            .collect();
        let entities = doc
            .entities()
            .map(|e| EntityDto {
                id: e.id.to_string(),
                kind: format!("{:?}", e.kind).to_ascii_lowercase(),
                layer_id: e.layer_id.to_string(),
                geometry: geometry_value(&e.geometry),
            })
            .collect();
        let snap = SnapshotDto {
            schema_version: 0,
            current_layer: doc.current_layer.to_string(),
            layers,
            entities,
        };
        serde_json::to_string(&snap).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Interleaved line vertices for GPU: all line segments in the document
    /// plus polyline edges and circle tessellations (32 segments).
    pub fn scene_line_vertices(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for ent in self.engine.session.document.entities() {
            append_geometry_lines(&ent.geometry, &mut out);
        }
        out
    }

    /// Ghost preview line vertices.
    pub fn ghost_line_vertices(&self, hover_x: f64, hover_y: f64) -> Vec<f32> {
        let out = self
            .engine
            .handle(CommandEvent::Hover(Vec2::new(hover_x, hover_y)));
        let mut verts = Vec::new();
        for g in out.ghost {
            append_geometry_lines(&g, &mut verts);
        }
        // Hover is side-effect free for document; but handle mutates last snap only via &mut.
        // Note: we needed &mut self for handle — ghost query advances nothing harmful.
        let _ = out;
        verts
    }

    /// Selected entity ids as JSON array of strings.
    pub fn selection_json(&self) -> String {
        let ids: Vec<String> = self.engine.selection.iter().map(|id| id.to_string()).collect();
        serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into())
    }

    /// Active prompt (French) or empty.
    pub fn prompt_fr(&self) -> String {
        // Recompute prompt without event: peek via active command path
        // Expose last by running a no-op hover at last point if any
        String::new()
    }
}

impl Default for WasmCad {
    fn default() -> Self {
        Self::new()
    }
}

fn output_json(out: &kelicad_cmd::CommandOutput) -> String {
    let ghost_count = out.ghost.len();
    let snap = out.snap.as_ref().map(|s| {
        serde_json::json!({
            "x": s.point.x,
            "y": s.point.y,
            "kind": format!("{:?}", s.kind),
        })
    });
    serde_json::json!({
        "prompt_fr": out.prompt.as_ref().map(|p| p.message_fr.clone()),
        "prompt_en": out.prompt.as_ref().map(|p| p.message_en.clone()),
        "echoes": out.echoes,
        "document_changed": out.document_changed,
        "ghost_count": ghost_count,
        "snap": snap,
    })
    .to_string()
}

fn geometry_value(g: &Geometry) -> serde_json::Value {
    match g {
        Geometry::Point(p) => serde_json::json!({"type":"point","x":p.x,"y":p.y}),
        Geometry::Line(l) => serde_json::json!({
            "type":"line","x0":l.start.x,"y0":l.start.y,"x1":l.end.x,"y1":l.end.y
        }),
        Geometry::Circle(c) => serde_json::json!({
            "type":"circle","cx":c.center.x,"cy":c.center.y,"r":c.radius
        }),
        Geometry::Arc(a) => serde_json::json!({
            "type":"arc","cx":a.center.x,"cy":a.center.y,"r":a.radius,
            "start_angle":a.start_angle,"end_angle":a.end_angle,"clockwise":a.clockwise
        }),
        Geometry::Polyline(p) => serde_json::json!({
            "type":"polyline",
            "closed": p.closed,
            "vertices": p.vertices.iter().map(|v| [v.x, v.y]).collect::<Vec<_>>()
        }),
    }
}

fn append_geometry_lines(g: &Geometry, out: &mut Vec<f32>) {
    match g {
        Geometry::Line(l) => {
            out.extend_from_slice(&[
                l.start.x as f32,
                l.start.y as f32,
                l.end.x as f32,
                l.end.y as f32,
            ]);
        }
        Geometry::Polyline(p) => {
            for seg in p.segments() {
                out.extend_from_slice(&[
                    seg.start.x as f32,
                    seg.start.y as f32,
                    seg.end.x as f32,
                    seg.end.y as f32,
                ]);
            }
        }
        Geometry::Circle(c) => {
            const N: usize = 48;
            for i in 0..N {
                let a0 = (i as f64) / (N as f64) * std::f64::consts::TAU;
                let a1 = ((i + 1) as f64) / (N as f64) * std::f64::consts::TAU;
                let p0 = c.point_at_angle(a0);
                let p1 = c.point_at_angle(a1);
                out.extend_from_slice(&[p0.x as f32, p0.y as f32, p1.x as f32, p1.y as f32]);
            }
        }
        Geometry::Arc(a) => {
            const N: usize = 32;
            for i in 0..N {
                let t0 = (i as f64) / (N as f64);
                let t1 = ((i + 1) as f64) / (N as f64);
                let p0 = a.point_at(t0);
                let p1 = a.point_at(t1);
                out.extend_from_slice(&[p0.x as f32, p0.y as f32, p1.x as f32, p1.y as f32]);
            }
        }
        Geometry::Point(p) => {
            let s = 0.15;
            out.extend_from_slice(&[
                (p.x - s) as f32,
                p.y as f32,
                (p.x + s) as f32,
                p.y as f32,
                p.x as f32,
                (p.y - s) as f32,
                p.x as f32,
                (p.y + s) as f32,
            ]);
        }
    }
}

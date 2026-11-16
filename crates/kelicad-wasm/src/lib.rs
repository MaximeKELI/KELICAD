//! WASM façade for the KELICAD core (commands + document).

#![forbid(unsafe_code)]

use kelicad_ai::{AgentMode, DeterministicProvider, KeliAgent, UuidV7Gen as AiUuidV7Gen};
use kelicad_bim::{export_ifc, schedule_csv, BimElement};
use kelicad_cmd::{CommandEngine, CommandEvent, CommandId, UuidV7Gen};
use kelicad_constraints::{solve, Constraint, ConstraintSystem};
use kelicad_doc::{EntityId, EntityRecord, Geometry, Layer, LayerId};
use kelicad_geom::{Arc, Circle, Line, Polyline};
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use kelicad_solid::{derive_view, BuiltinSolidKernel, Profile2, SolidBody, SolidKernel, ViewKind};
use kelicad_sync::{
    ActorId, CommentAnchor, CommentId, HybridTime, OpEnvelope, PresenceState, Replica, WireMessage,
};
use kelicad_vectorize::vectorize_bytes;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
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
    keli: KeliAgent<DeterministicProvider, AiUuidV7Gen>,
    replica: Replica,
    sync_journal_cursor: usize,
    last_prompt_fr: String,
    constraints: ConstraintSystem,
    solids: Vec<SolidBody>,
    bim: Vec<BimElement>,
}

#[wasm_bindgen]
impl WasmCad {
    /// New empty drawing.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let actor = ActorId::new();
        Self {
            engine: CommandEngine::new(DocSession::new(), UuidV7Gen),
            keli: KeliAgent::new(DeterministicProvider, AiUuidV7Gen),
            replica: Replica::new(actor, "Anonyme", [80, 180, 220]),
            sync_journal_cursor: 0,
            last_prompt_fr: String::new(),
            constraints: ConstraintSystem::new(),
            solids: Vec::new(),
            bim: Vec::new(),
        }
    }

    fn remember(&mut self, out: &kelicad_cmd::CommandOutput) {
        if let Some(p) = &out.prompt {
            self.last_prompt_fr = p.message_fr.clone();
        } else if self.engine.active_command().is_none() {
            self.last_prompt_fr.clear();
        }
    }

    /// Entity count.
    pub fn entity_count(&self) -> u32 {
        self.engine.session.document.entity_count() as u32
    }

    /// Submit a command-line string (command name or point/number).
    pub fn submit_text(&mut self, text: &str) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Text(text.to_string()));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Start a command by alias/name.
    pub fn start_command(&mut self, name: &str) -> Result<String, JsValue> {
        let id = CommandId::parse(name).ok_or_else(|| JsValue::from_str("unknown command"))?;
        let out = self.engine.handle(CommandEvent::Start(id));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// World-space pick.
    pub fn pick(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Pick(Vec2::new(x, y)));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Hover for ghost / snap.
    pub fn hover(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Hover(Vec2::new(x, y)));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Enter.
    pub fn enter(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Enter);
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Cancel (Escape).
    pub fn cancel(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Cancel);
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Undo.
    pub fn undo(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Start(CommandId::Undo));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Redo.
    pub fn redo(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::Start(CommandId::Redo));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Set snap aperture in world units.
    pub fn set_snap_aperture(&mut self, aperture: f64) {
        self.engine.snap_aperture = aperture.max(1e-9);
    }

    /// Replace the session document from a snapshot JSON (empty journal).
    pub fn import_snapshot_json(&mut self, json: &str) -> Result<(), JsValue> {
        #[derive(Deserialize)]
        struct InSnap {
            current_layer: String,
            layers: Vec<InLayer>,
            entities: Vec<InEntity>,
        }
        #[derive(Deserialize)]
        struct InLayer {
            id: String,
            name: String,
            color: [u8; 3],
        }
        #[derive(Deserialize)]
        struct InEntity {
            id: String,
            layer_id: String,
            geometry: serde_json::Value,
        }

        let snap: InSnap =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut session = DocSession::new();
        for layer in &snap.layers {
            let id = LayerId::from_uuid(
                Uuid::parse_str(&layer.id).map_err(|e| JsValue::from_str(&e.to_string()))?,
            );
            if id != session.document.default_layer_id() {
                session
                    .apply(Operation::CreateLayer {
                        layer: Layer::new(id, layer.name.clone(), layer.color),
                    })
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
            } else {
                // Update default layer display attrs via upsert
                session
                    .document
                    .upsert_layer(Layer::new(id, layer.name.clone(), layer.color));
            }
        }
        for ent in &snap.entities {
            let id = EntityId::from_uuid(
                Uuid::parse_str(&ent.id).map_err(|e| JsValue::from_str(&e.to_string()))?,
            );
            let layer_id = LayerId::from_uuid(
                Uuid::parse_str(&ent.layer_id).map_err(|e| JsValue::from_str(&e.to_string()))?,
            );
            let geometry = geometry_from_value(&ent.geometry).map_err(|e| JsValue::from_str(&e))?;
            session
                .apply(Operation::CreateEntity {
                    record: EntityRecord {
                        id,
                        kind: geometry.kind(),
                        geometry,
                        layer_id,
                    },
                })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
        if let Ok(u) = Uuid::parse_str(&snap.current_layer) {
            let _ = session.document.set_current_layer(LayerId::from_uuid(u));
        }
        // Reset journal so undo doesn't walk the import batch as user history.
        session.journal = kelicad_ops::OpJournal::new();
        self.engine = CommandEngine::new(session, UuidV7Gen);
        self.last_prompt_fr.clear();
        Ok(())
    }

    /// Export model space as DXF ASCII.
    pub fn export_dxf(&self) -> Result<String, JsValue> {
        kelicad_io::export_dxf(&self.engine.session).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Import DXF ASCII, replacing the current document. Returns import report JSON.
    pub fn import_dxf(&mut self, text: &str) -> Result<String, JsValue> {
        let (session, report) =
            kelicad_io::import_dxf(text).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.engine = CommandEngine::new(session, UuidV7Gen);
        self.last_prompt_fr.clear();
        Ok(serde_json::json!({
            "imported": report.imported,
            "skipped": report.skipped,
            "warnings": report.warnings,
        })
        .to_string())
    }

    /// Export SVG.
    pub fn export_svg(&self, width: f64, height: f64) -> Result<String, JsValue> {
        kelicad_io::export_svg(&self.engine.session, width, height)
            .map_err(|e| JsValue::from_str(&e.to_string()))
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

    /// Ghost preview line vertices at world hover position.
    pub fn ghost_line_vertices(&mut self, hover_x: f64, hover_y: f64) -> Vec<f32> {
        let out = self
            .engine
            .handle(CommandEvent::Hover(Vec2::new(hover_x, hover_y)));
        self.remember(&out);
        let mut verts = Vec::new();
        for g in &out.ghost {
            append_geometry_lines(g, &mut verts);
        }
        verts
    }

    /// Selected entity ids as JSON array of strings.
    pub fn selection_json(&self) -> String {
        let ids: Vec<String> = self
            .engine
            .selection
            .iter()
            .map(|id| id.to_string())
            .collect();
        serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into())
    }

    /// Active prompt (French) or empty.
    pub fn prompt_fr(&self) -> String {
        self.last_prompt_fr.clone()
    }

    /// Ask KELI (Propose by default). Returns AskResult JSON.
    pub fn keli_ask(&mut self, utterance: &str) -> Result<String, JsValue> {
        self.sync_keli_selection();
        let result = self
            .keli
            .ask(&mut self.engine.session, utterance)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Accept pending KELI proposal (one undoable batch).
    pub fn keli_accept(&mut self) -> Result<String, JsValue> {
        self.keli
            .accept(&mut self.engine.session)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(r#"{"ok":true}"#.into())
    }

    /// Reject pending KELI proposal.
    pub fn keli_reject(&mut self) -> Result<String, JsValue> {
        self.keli
            .reject()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(r#"{"ok":true}"#.into())
    }

    /// Set KELI mode: "propose" | "apply".
    pub fn keli_set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        let m = match mode.to_ascii_lowercase().as_str() {
            "apply" | "appliquer" => AgentMode::Apply,
            "propose" | "proposer" => AgentMode::Propose,
            other => {
                return Err(JsValue::from_str(&format!("unknown mode {other}")));
            }
        };
        self.keli.set_mode(m);
        Ok(())
    }

    /// Pending proposal JSON or null.
    pub fn keli_proposal_json(&self) -> String {
        match self.keli.pending() {
            Some(p) => serde_json::to_string(&kelicad_ai::ProposalDto::from(p))
                .unwrap_or_else(|_| "null".into()),
            None => "null".into(),
        }
    }

    /// AI journal JSON array.
    pub fn keli_journal_json(&self) -> String {
        self.keli.journal.to_json().unwrap_or_else(|_| "[]".into())
    }

    /// Ghost vertices for pending KELI proposal (interleaved x0,y0,x1,y1).
    pub fn keli_ghost_vertices(&self) -> Vec<f32> {
        self.keli.ghost_vertices()
    }

    fn sync_keli_selection(&mut self) {
        self.keli.selection = self.engine.selection.iter().collect();
    }

    /// Set local collab identity (display name + RGB).
    pub fn sync_set_identity(&mut self, name: &str, r: u8, g: u8, b: u8) {
        self.replica.presence.name = name.to_string();
        self.replica.presence.color = [r, g, b];
    }

    /// Actor id string.
    pub fn sync_actor_id(&self) -> String {
        self.replica.actor_id.to_string()
    }

    /// Publish newly journaled local ops to the collab outbox. Returns JSON array of envelopes.
    pub fn sync_publish_local(&mut self) -> String {
        let applied = self.engine.session.journal.len_applied();
        if self.sync_journal_cursor > applied {
            self.sync_journal_cursor = applied;
        }
        let entries: Vec<_> = self
            .engine
            .session
            .journal
            .applied()
            .skip(self.sync_journal_cursor)
            .cloned()
            .collect();
        for entry in entries {
            self.replica.publish_already_applied(entry.op);
        }
        self.sync_journal_cursor = applied;
        self.replica.mirror_session(self.engine.session.clone());
        self.sync_drain_outbox()
    }

    /// Drain pending outbox envelopes as JSON array.
    pub fn sync_drain_outbox(&mut self) -> String {
        let out = self.replica.drain_outbox();
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
    }

    /// Integrate one remote envelope JSON. Returns status string.
    pub fn sync_integrate(&mut self, envelope_json: &str) -> Result<String, JsValue> {
        let env: OpEnvelope =
            serde_json::from_str(envelope_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        if env.actor_id == self.replica.actor_id {
            return Ok("self".into());
        }
        let res = self
            .replica
            .integrate(&env)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.engine.session = self.replica.session.clone();
        // Remote ops must not sit in local undo stack.
        self.engine.session.journal = kelicad_ops::OpJournal::new();
        self.sync_journal_cursor = 0;
        Ok(format!("{res:?}"))
    }

    /// Catch-up from snapshot ops JSON array.
    pub fn sync_catch_up(&mut self, ops_json: &str) -> Result<u32, JsValue> {
        let ops: Vec<OpEnvelope> =
            serde_json::from_str(ops_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let n = self
            .replica
            .catch_up(&ops)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.engine.session = self.replica.session.clone();
        self.engine.session.journal = kelicad_ops::OpJournal::new();
        self.sync_journal_cursor = 0;
        Ok(n as u32)
    }

    /// Hello / presence wire message JSON.
    pub fn sync_hello_json(&self) -> String {
        serde_json::to_string(&WireMessage::Hello {
            state: self.replica.presence.clone(),
        })
        .unwrap_or_else(|_| "{}".into())
    }

    /// Update local presence; returns Presence wire JSON.
    pub fn sync_set_presence(&mut self, x: f64, y: f64, has_cursor: bool) -> String {
        let cursor = if has_cursor {
            Some(Vec2::new(x, y))
        } else {
            None
        };
        let selection = self.engine.selection.iter().collect();
        let state = self.replica.set_presence(cursor, selection);
        serde_json::to_string(&WireMessage::Presence { state }).unwrap_or_else(|_| "{}".into())
    }

    /// Ingest peer presence JSON (`PresenceState` or WireMessage::Presence).
    pub fn sync_upsert_peer(&mut self, json: &str) -> Result<(), JsValue> {
        if let Ok(WireMessage::Presence { state }) = serde_json::from_str::<WireMessage>(json) {
            self.replica.upsert_peer(state);
            return Ok(());
        }
        let state: PresenceState =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.replica.upsert_peer(state);
        Ok(())
    }

    /// Peers presence as JSON array.
    pub fn sync_peers_json(&self) -> String {
        let peers: Vec<_> = self.replica.peers.values().cloned().collect();
        serde_json::to_string(&peers).unwrap_or_else(|_| "[]".into())
    }

    /// Add a point-anchored comment; returns comment JSON.
    pub fn sync_add_comment_at(&mut self, x: f64, y: f64, body: &str) -> String {
        let c = self.replica.add_comment(
            CommentAnchor::Point {
                position: Vec2::new(x, y),
            },
            body,
        );
        serde_json::to_string(&c).unwrap_or_else(|_| "{}".into())
    }

    /// Comments JSON array.
    pub fn sync_comments_json(&self) -> String {
        serde_json::to_string(&self.replica.comments.all()).unwrap_or_else(|_| "[]".into())
    }

    /// Resolve comment by id string.
    pub fn sync_resolve_comment(&mut self, id: &str) -> Result<bool, JsValue> {
        let u = Uuid::parse_str(id).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(self.replica.resolve_comment(CommentId(u)))
    }

    /// Upsert comment from JSON (remote).
    pub fn sync_upsert_comment_json(&mut self, json: &str) -> Result<(), JsValue> {
        let c: kelicad_sync::Comment =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let time = c.created;
        self.replica.comments.upsert(c, time);
        Ok(())
    }

    /// Save named version; returns restore point JSON.
    pub fn sync_save_version(&mut self, name: &str) -> Result<String, JsValue> {
        self.replica.mirror_session(self.engine.session.clone());
        let point = self
            .replica
            .save_version(name)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&point).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Versions JSON.
    pub fn sync_versions_json(&self) -> String {
        serde_json::to_string(&self.replica.versions).unwrap_or_else(|_| "[]".into())
    }

    /// Diff two version names → EntityDiff JSON.
    pub fn sync_diff_versions(&self, a: &str, b: &str) -> Result<String, JsValue> {
        let diff = self
            .replica
            .diff_versions(a, b)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&diff).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Document fingerprint for convergence checks.
    pub fn sync_fingerprint(&mut self) -> String {
        self.replica.mirror_session(self.engine.session.clone());
        self.replica.doc_fingerprint()
    }

    /// Export applied journal as a macro JSON script.
    pub fn macro_record_json(&self) -> String {
        let script = kelicad_plugin::record_macro(&self.engine.session);
        kelicad_plugin::macro_to_json(&script).unwrap_or_else(|_| "{}".into())
    }

    /// Add a constraint from JSON (`Constraint` serde). Returns constraint id.
    pub fn constraint_add_json(&mut self, json: &str) -> Result<String, JsValue> {
        let c: Constraint =
            serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let id = self
            .constraints
            .add(c)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(id.to_string())
    }

    /// Solve the constraint system and apply geometry ops (one undoable batch).
    pub fn constraint_solve(&mut self) -> Result<String, JsValue> {
        let result = solve(&self.constraints, &self.engine.session.document)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let n = result.ops.len();
        if !result.ops.is_empty() {
            self.engine
                .session
                .apply(Operation::Batch { ops: result.ops })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
        Ok(serde_json::json!({
            "status": result.status,
            "residual": result.residual,
            "ops": n,
        })
        .to_string())
    }

    /// Constraint system JSON.
    pub fn constraints_json(&self) -> String {
        serde_json::to_string(&self.constraints).unwrap_or_else(|_| "{}".into())
    }

    /// Vectorize SVG/PNG bytes; insert entities on the current layer. Returns count.
    pub fn vectorize_bytes(&mut self, data: &[u8], filename_hint: &str) -> Result<u32, JsValue> {
        let geoms = vectorize_bytes(data, Some(filename_hint))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let layer = self.engine.session.document.current_layer;
        let mut ops = Vec::new();
        for geom in geoms {
            let id = EntityId::from_uuid(Uuid::now_v7());
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: geom.kind(),
                    geometry: geom,
                    layer_id: layer,
                },
            });
        }
        let n = ops.len() as u32;
        if !ops.is_empty() {
            self.engine
                .session
                .apply(Operation::Batch { ops })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
        Ok(n)
    }

    /// Extrude selected (or first) closed polyline/rectangle to a solid. Returns JSON summary.
    pub fn solid_extrude_selected(&mut self, height: f64) -> Result<String, JsValue> {
        let profile = find_closed_profile(&self.engine.session.document, &self.engine.selection)
            .ok_or_else(|| JsValue::from_str("no closed polyline/rectangle to extrude"))?;
        let k = BuiltinSolidKernel;
        let body = k
            .extrude(&profile, height)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let elem = BimElement::new(kelicad_bim::BimKind::Wall, body.id)
            .with_prop("tag", format!("EX-{}", self.solids.len() + 1))
            .with_prop("height", format!("{height}"));
        let summary = serde_json::json!({
            "solid_id": body.id.to_string(),
            "bim_id": elem.id.to_string(),
            "aabb": {
                "min": [body.aabb.min.x, body.aabb.min.y, body.aabb.min.z],
                "max": [body.aabb.max.x, body.aabb.max.y, body.aabb.max.z],
            },
            "triangles": body.mesh.triangle_count(),
        });
        self.bim.push(elem);
        self.solids.push(body);
        Ok(summary.to_string())
    }

    /// Derive a 2D view from the last solid and insert geometries (undoable).
    pub fn view_derive_apply(&mut self, kind: &str) -> Result<u32, JsValue> {
        let body = self
            .solids
            .last()
            .ok_or_else(|| JsValue::from_str("no solid to derive view from"))?;
        let view = match kind.to_ascii_lowercase().as_str() {
            "plan" => ViewKind::Plan,
            "elevation_x" | "elevx" => ViewKind::ElevationX,
            "elevation_y" | "elevy" => ViewKind::ElevationY,
            "section" => ViewKind::Section,
            other => {
                return Err(JsValue::from_str(&format!("unknown view kind {other}")));
            }
        };
        let geoms = derive_view(body, view, None);
        let layer = self.engine.session.document.current_layer;
        let mut ops = Vec::new();
        for geom in geoms {
            let id = EntityId::from_uuid(Uuid::now_v7());
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: geom.kind(),
                    geometry: geom,
                    layer_id: layer,
                },
            });
        }
        let n = ops.len() as u32;
        if !ops.is_empty() {
            self.engine
                .session
                .apply(Operation::Batch { ops })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
        Ok(n)
    }

    /// BIM schedule CSV for current elements.
    pub fn bim_schedule_csv(&self) -> String {
        schedule_csv(&self.bim, &["material", "height", "thickness", "length"])
    }

    /// IFC subset export of current BIM + solids.
    pub fn ifc_export(&self) -> Result<String, JsValue> {
        export_ifc(&self.bim, &self.solids).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Solid count.
    pub fn solid_count(&self) -> u32 {
        self.solids.len() as u32
    }

    /// Handle an inbound wire message JSON (op/presence/comment/snapshot).
    pub fn sync_handle_wire(&mut self, text: &str) -> Result<String, JsValue> {
        let msg: WireMessage =
            serde_json::from_str(text).map_err(|e| JsValue::from_str(&e.to_string()))?;
        match msg {
            WireMessage::Snapshot {
                ops,
                comments,
                versions,
            } => {
                let n = self
                    .replica
                    .catch_up(&ops)
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                for c in comments {
                    let t = c.created;
                    self.replica.comments.upsert(c, t);
                }
                self.replica.versions = versions;
                self.engine.session = self.replica.session.clone();
                self.engine.session.journal = kelicad_ops::OpJournal::new();
                self.sync_journal_cursor = 0;
                Ok(format!("snapshot:{n}"))
            }
            WireMessage::Op { envelope } => self.sync_integrate(
                &serde_json::to_string(&envelope).map_err(|e| JsValue::from_str(&e.to_string()))?,
            ),
            WireMessage::Presence { state } => {
                self.replica.upsert_peer(state);
                Ok("presence".into())
            }
            WireMessage::CommentUpsert { comment } => {
                let t = comment.created;
                self.replica.comments.upsert(comment, t);
                Ok("comment".into())
            }
            WireMessage::CommentResolve {
                id,
                lamport,
                actor_id,
            } => {
                let u = Uuid::parse_str(&id).map_err(|e| JsValue::from_str(&e.to_string()))?;
                let actor = Uuid::parse_str(&actor_id)
                    .map(ActorId)
                    .unwrap_or(self.replica.actor_id);
                self.replica
                    .comments
                    .resolve(CommentId(u), HybridTime { lamport, actor });
                Ok("resolve".into())
            }
            WireMessage::VersionSave { point } => {
                self.replica.versions.push(point);
                Ok("version".into())
            }
            WireMessage::Hello { state } => {
                self.replica.upsert_peer(state);
                Ok("hello".into())
            }
            WireMessage::Ping => Ok("ping".into()),
        }
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

fn geometry_from_value(v: &serde_json::Value) -> Result<Geometry, String> {
    let ty = v
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "geometry.type missing".to_string())?;
    match ty {
        "point" => Ok(Geometry::Point(Vec2::new(num(v, "x")?, num(v, "y")?))),
        "line" => Ok(Geometry::Line(Line::new(
            Vec2::new(num(v, "x0")?, num(v, "y0")?),
            Vec2::new(num(v, "x1")?, num(v, "y1")?),
        ))),
        "circle" => Ok(Geometry::Circle(Circle::new(
            Vec2::new(num(v, "cx")?, num(v, "cy")?),
            num(v, "r")?,
        ))),
        "arc" => Ok(Geometry::Arc(Arc {
            center: Vec2::new(num(v, "cx")?, num(v, "cy")?),
            radius: num(v, "r")?,
            start_angle: num(v, "start_angle")?,
            end_angle: num(v, "end_angle")?,
            clockwise: v
                .get("clockwise")
                .and_then(|b| b.as_bool())
                .unwrap_or(false),
        })),
        "polyline" => {
            let closed = v.get("closed").and_then(|b| b.as_bool()).unwrap_or(false);
            let verts = v
                .get("vertices")
                .and_then(|a| a.as_array())
                .ok_or_else(|| "polyline.vertices".to_string())?;
            let mut points = Vec::new();
            for item in verts {
                let arr = item.as_array().ok_or_else(|| "vertex".to_string())?;
                points.push(Vec2::new(
                    arr[0].as_f64().ok_or_else(|| "vx".to_string())?,
                    arr[1].as_f64().ok_or_else(|| "vy".to_string())?,
                ));
            }
            Ok(Geometry::Polyline(if closed {
                Polyline::closed(points)
            } else {
                Polyline::new(points)
            }))
        }
        "text" => Ok(Geometry::Text(kelicad_doc::TextGeom {
            position: Vec2::new(num(v, "x")?, num(v, "y")?),
            height: num(v, "height").unwrap_or(2.5),
            rotation: num(v, "rotation").unwrap_or(0.0),
            content: v
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            align: kelicad_doc::TextAlign::Left,
            style_id: None,
            annotative: false,
})),
        "dimension" => Ok(Geometry::Dimension(kelicad_doc::DimensionGeom {
            kind: kelicad_doc::DimKind::Linear,
            def1: Vec2::new(num(v, "x1")?, num(v, "y1")?),
            def2: Vec2::new(num(v, "x2")?, num(v, "y2")?),
            dim_line: Vec2::new(num(v, "dx").unwrap_or(0.0), num(v, "dy").unwrap_or(0.0)),
            assoc_a: None,
            assoc_b: None,
            text_override: v
                .get("text_override")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string()),
                style_id: None,
                annotative: false,
                angle_p3: None,
})),
        "hatch" => {
            let verts = v
                .get("boundary")
                .and_then(|a| a.as_array())
                .ok_or_else(|| "hatch.boundary".to_string())?;
            let mut boundary = Vec::new();
            for item in verts {
                let arr = item.as_array().ok_or_else(|| "hv".to_string())?;
                boundary.push(Vec2::new(
                    arr[0].as_f64().unwrap_or(0.0),
                    arr[1].as_f64().unwrap_or(0.0),
                ));
            }
            Ok(Geometry::Hatch(kelicad_doc::HatchGeom::solid(boundary)))
        }
        "ellipse" => Ok(Geometry::Ellipse(kelicad_geom::Ellipse {
            center: Vec2::new(num(v, "cx")?, num(v, "cy")?),
            major: Vec2::new(num(v, "mjx")?, num(v, "mjy")?),
            ratio: num(v, "ratio").unwrap_or(1.0),
            start_angle: num(v, "start_angle").unwrap_or(0.0),
            end_angle: num(v, "end_angle").unwrap_or(0.0),
        })),
        "spline" => {
            let controls = v
                .get("controls")
                .and_then(|x| x.as_array())
                .ok_or_else(|| "controls".to_string())?;
            let mut pts = Vec::new();
            for item in controls {
                let arr = item.as_array().ok_or_else(|| "cp".to_string())?;
                pts.push(Vec2::new(
                    arr[0].as_f64().unwrap_or(0.0),
                    arr[1].as_f64().unwrap_or(0.0),
                ));
            }
            let degree = v.get("degree").and_then(|x| x.as_u64()).unwrap_or(3) as u8;
            let closed = v.get("closed").and_then(|x| x.as_bool()).unwrap_or(false);
            kelicad_geom::Spline::from_controls(pts, degree, closed)
                .map(Geometry::Spline)
                .ok_or_else(|| "spline controls".into())
        }
        other => Err(format!("unknown geometry type {other}")),
    }
}

fn num(v: &serde_json::Value, key: &str) -> Result<f64, String> {
    v.get(key)
        .and_then(|x| x.as_f64())
        .ok_or_else(|| format!("missing {key}"))
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
        Geometry::Text(t) => serde_json::json!({
            "type":"text","x":t.position.x,"y":t.position.y,"height":t.height,
            "rotation":t.rotation,"content":t.content
        }),
        Geometry::MText(t) => serde_json::json!({
            "type":"mtext","x":t.position.x,"y":t.position.y,"height":t.height,"content":t.content
        }),
        Geometry::Dimension(d) => serde_json::json!({
            "type":"dimension","x1":d.def1.x,"y1":d.def1.y,"x2":d.def2.x,"y2":d.def2.y,
            "dx":d.dim_line.x,"dy":d.dim_line.y,"text_override":d.text_override
        }),
        Geometry::Hatch(h) => serde_json::json!({
            "type":"hatch","boundary": h.boundary.iter().map(|v| [v.x,v.y]).collect::<Vec<_>>(),
            "scale": h.scale, "angle": h.angle
        }),
        Geometry::Ellipse(e) => serde_json::json!({
            "type":"ellipse","cx":e.center.x,"cy":e.center.y,
            "mjx":e.major.x,"mjy":e.major.y,"ratio":e.ratio,
            "start_angle":e.start_angle,"end_angle":e.end_angle
        }),
        Geometry::Spline(s) => serde_json::json!({
            "type":"spline","degree":s.degree,"closed":s.closed,
            "controls": s.control_points.iter().map(|v| [v.x,v.y]).collect::<Vec<_>>()
        }),
        Geometry::Insert(i) => serde_json::json!({
            "type":"insert","x":i.position.x,"y":i.position.y,"block_id":i.block_id.to_string()
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
        Geometry::Text(t) => {
            let s = 0.2;
            let p = t.position;
            out.extend_from_slice(&[(p.x - s) as f32, p.y as f32, (p.x + s) as f32, p.y as f32]);
        }
        Geometry::MText(t) => {
            let s = 0.2;
            let p = t.position;
            out.extend_from_slice(&[(p.x - s) as f32, p.y as f32, (p.x + s) as f32, p.y as f32]);
        }
        Geometry::Dimension(d) => {
            out.extend_from_slice(&[
                d.def1.x as f32,
                d.def1.y as f32,
                d.def2.x as f32,
                d.def2.y as f32,
                d.def1.x as f32,
                d.def1.y as f32,
                d.dim_line.x as f32,
                d.dim_line.y as f32,
                d.def2.x as f32,
                d.def2.y as f32,
                d.dim_line.x as f32,
                d.dim_line.y as f32,
            ]);
        }
        Geometry::Hatch(h) => {
            for seg in h.boundary.windows(2) {
                out.extend_from_slice(&[
                    seg[0].x as f32,
                    seg[0].y as f32,
                    seg[1].x as f32,
                    seg[1].y as f32,
                ]);
            }
        }
        Geometry::Ellipse(e) => {
            let poly = e.to_polyline(48);
            for seg in poly.segments() {
                out.extend_from_slice(&[
                    seg.start.x as f32,
                    seg.start.y as f32,
                    seg.end.x as f32,
                    seg.end.y as f32,
                ]);
            }
        }
        Geometry::Spline(s) => {
            let poly = s.to_polyline(10);
            for seg in poly.segments() {
                out.extend_from_slice(&[
                    seg.start.x as f32,
                    seg.start.y as f32,
                    seg.end.x as f32,
                    seg.end.y as f32,
                ]);
            }
        }
        Geometry::Insert(i) => {
            let s = 0.25;
            let p = i.position;
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

fn find_closed_profile(
    doc: &kelicad_doc::Document,
    selection: &kelicad_cmd::Selection,
) -> Option<Profile2> {
    let ids: Vec<EntityId> = if selection.is_empty() {
        doc.entities().map(|e| e.id).collect()
    } else {
        selection.to_vec()
    };
    for id in ids {
        let Some(ent) = doc.get_entity(id) else {
            continue;
        };
        if let Geometry::Polyline(p) = &ent.geometry {
            if p.closed && p.vertices.len() >= 3 {
                return Profile2::from_vertices(p.vertices.clone()).ok();
            }
        }
    }
    None
}

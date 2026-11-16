//! WASM façade for the KELICAD core (commands + document).

#![forbid(unsafe_code)]

use kelicad_ai::{
    suggest_impulses, AgentMode, DeterministicProvider, KeliAgent, UuidV7Gen as AiUuidV7Gen,
};
use kelicad_bim::{export_ifc, schedule_csv, BimElement};
use kelicad_cmd::{CommandEngine, CommandEvent, CommandId, UuidV7Gen};
use kelicad_constraints::{solve, Constraint, ConstraintSystem};
use kelicad_doc::{
    expand_insert, geometry_aabb, resolve_linetype, template_entities, BlockDef, BlockId, Document,
    DrawingUnits, EntityId, EntityRecord, Geometry, Layer, LayerId, TemplateKind,
};
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
    frozen: bool,
    locked: bool,
    no_plot: bool,
    current: bool,
}

#[derive(Serialize)]
struct SceneBatchDto {
    r: f32,
    g: f32,
    b: f32,
    verts: Vec<f32>,
}

#[derive(Serialize)]
struct BlockDto {
    id: String,
    name: String,
    origin: [f64; 2],
    entities: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct SnapshotDto {
    schema_version: u32,
    current_layer: String,
    layers: Vec<LayerDto>,
    entities: Vec<EntityDto>,
    #[serde(default)]
    blocks: Vec<BlockDto>,
    #[serde(default)]
    units: String,
    #[serde(default)]
    project_title: String,
    #[serde(default)]
    project_scale: String,
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

    /// Idle pick that toggles selection (Shift+click).
    pub fn pick_toggle(&mut self, x: f64, y: f64) -> Result<String, JsValue> {
        let out = self
            .engine
            .handle(CommandEvent::PickToggle(Vec2::new(x, y)));
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Window / crossing selection. `crossing` = intersect; `additive` keeps prior selection.
    pub fn select_rect(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        crossing: bool,
        additive: bool,
    ) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::SelectRect {
            a: Vec2::new(x0, y0),
            b: Vec2::new(x1, y1),
            crossing,
            additive,
        });
        self.remember(&out);
        Ok(output_json(&out))
    }

    /// Clear selection.
    pub fn select_clear(&mut self) -> Result<String, JsValue> {
        let out = self.engine.handle(CommandEvent::SelectClear);
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
            #[serde(default)]
            blocks: Vec<InBlock>,
            #[serde(default)]
            units: Option<String>,
            #[serde(default)]
            project_title: Option<String>,
            #[serde(default)]
            project_scale: Option<String>,
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
        #[derive(Deserialize)]
        struct InBlock {
            id: String,
            name: String,
            origin: [f64; 2],
            entities: Vec<serde_json::Value>,
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
        for blk in &snap.blocks {
            let id = BlockId::from_uuid(
                Uuid::parse_str(&blk.id).map_err(|e| JsValue::from_str(&e.to_string()))?,
            );
            let mut ents = Vec::new();
            for g in &blk.entities {
                ents.push(geometry_from_value(g).map_err(|e| JsValue::from_str(&e))?);
            }
            let def = BlockDef {
                id,
                name: blk.name.clone(),
                origin: Vec2::new(blk.origin[0], blk.origin[1]),
                entities: ents,
                attributes: vec![],
            };
            session
                .apply(Operation::UpsertBlock { def })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
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
                        props: Default::default(),
        },
                })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
        if let Ok(u) = Uuid::parse_str(&snap.current_layer) {
            let _ = session.document.set_current_layer(LayerId::from_uuid(u));
        }
        if let Some(u) = snap.units.as_deref().and_then(DrawingUnits::parse) {
            session.document.units = u;
        }
        if let Some(t) = snap.project_title {
            session.document.project_title = t;
        }
        if let Some(s) = snap.project_scale {
            session.document.project_scale = s;
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

    /// Export PDF bytes (WASM-safe lite exporter).
    pub fn export_pdf(&self) -> Result<Vec<u8>, JsValue> {
        kelicad_io::export_pdf_lite(&self.engine.session)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Export document snapshot JSON (for .kelicad writers / download).
    pub fn export_snapshot_json(&self) -> Result<String, JsValue> {
        let doc = &self.engine.session.document;
        let current = doc.current_layer;
        let layers = doc
            .layers()
            .map(|l| LayerDto {
                id: l.id.to_string(),
                name: l.name.clone(),
                color: l.color,
                frozen: l.flags.frozen,
                locked: l.flags.locked,
                no_plot: l.flags.no_plot,
                current: l.id == current,
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
        let blocks = doc
            .blocks()
            .map(|b| BlockDto {
                id: b.id.to_string(),
                name: b.name.clone(),
                origin: [b.origin.x, b.origin.y],
                entities: b.entities.iter().map(geometry_value).collect(),
            })
            .collect();
        let snap = SnapshotDto {
            schema_version: 0,
            current_layer: doc.current_layer.to_string(),
            layers,
            entities,
            blocks,
            units: doc.units.as_str().into(),
            project_title: doc.project_title.clone(),
            project_scale: doc.project_scale.clone(),
        };
        serde_json::to_string(&snap).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Layers JSON (for UI panel).
    pub fn layers_json(&self) -> String {
        let doc = &self.engine.session.document;
        let current = doc.current_layer;
        let layers: Vec<LayerDto> = doc
            .layers()
            .map(|l| LayerDto {
                id: l.id.to_string(),
                name: l.name.clone(),
                color: l.color,
                frozen: l.flags.frozen,
                locked: l.flags.locked,
                no_plot: l.flags.no_plot,
                current: l.id == current,
            })
            .collect();
        serde_json::to_string(&layers).unwrap_or_else(|_| "[]".into())
    }

    /// Set current layer by UUID string.
    pub fn set_current_layer(&mut self, layer_id: &str) -> Result<(), JsValue> {
        let u = Uuid::parse_str(layer_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let id = LayerId::from_uuid(u);
        self.engine
            .session
            .apply(Operation::SetCurrentLayer { id })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create a named layer (optional RGB).
    pub fn create_layer(&mut self, name: &str, r: u8, g: u8, b: u8) -> Result<String, JsValue> {
        let id = LayerId::from_uuid(Uuid::now_v7());
        let layer = Layer::new(id, name, [r, g, b]);
        self.engine
            .session
            .apply(Operation::CreateLayer { layer })
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(id.to_string())
    }

    /// Move selected entities onto a layer.
    pub fn set_selection_layer(&mut self, layer_id: &str) -> Result<u32, JsValue> {
        let u = Uuid::parse_str(layer_id).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let layer = LayerId::from_uuid(u);
        let ids: Vec<_> = self.engine.selection.iter().collect();
        let mut n = 0u32;
        for id in ids {
            self.engine
                .session
                .apply(Operation::SetEntityLayer { id, layer_id: layer })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            n += 1;
        }
        Ok(n)
    }

    /// Apply color / linetype / lineweight overrides to the selection.
    /// Empty strings keep the existing field; `BYLAYER` clears an override.
    pub fn set_selection_props(
        &mut self,
        color: &str,
        linetype: &str,
        lineweight: &str,
    ) -> Result<u32, JsValue> {
        let ids: Vec<_> = self.engine.selection.iter().collect();
        let mut n = 0u32;
        for id in ids {
            let Some(ent) = self.engine.session.document.get_entity(id) else {
                continue;
            };
            let mut props = ent.props.clone();
            if !color.is_empty() {
                props.color = if color.eq_ignore_ascii_case("BYLAYER") {
                    None
                } else {
                    parse_named_color(color)
                };
            }
            if !linetype.is_empty() {
                props.linetype = if linetype.eq_ignore_ascii_case("BYLAYER") {
                    None
                } else {
                    Some(linetype.to_ascii_uppercase())
                };
            }
            if !lineweight.is_empty() {
                props.lineweight = if lineweight == "-1"
                    || lineweight.eq_ignore_ascii_case("BYLAYER")
                {
                    None
                } else {
                    lineweight.parse::<u16>().ok()
                };
            }
            self.engine
                .session
                .apply(Operation::SetEntityProps { id, props })
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            n += 1;
        }
        Ok(n)
    }

    /// Toggle ORTHO (F8-style).
    pub fn set_ortho(&mut self, enabled: bool) {
        self.engine.ortho = enabled;
        let _ = self.engine.sysvars.set(
            "ORTHOMODE",
            kelicad_cmd::SysVarValue::Int(if enabled { 1 } else { 0 }),
        );
    }

    /// ORTHO state.
    pub fn ortho(&self) -> bool {
        self.engine.ortho
    }

    /// Toggle object snap master (F3-style).
    pub fn set_osnap(&mut self, enabled: bool) {
        self.engine.osnap_enabled = enabled;
        if enabled && self.engine.sysvars.get_int("OSMODE") == 0 {
            let _ = self
                .engine
                .sysvars
                .set("OSMODE", kelicad_cmd::SysVarValue::Int(4133));
        }
        if !enabled {
            let _ = self
                .engine
                .sysvars
                .set("OSMODE", kelicad_cmd::SysVarValue::Int(0));
        }
        self.engine.sync_from_sysvars();
    }

    /// OSNAP state.
    pub fn osnap(&self) -> bool {
        self.engine.osnap_enabled
    }

    /// Read a system variable (display string).
    pub fn get_sysvar(&self, name: &str) -> Result<String, JsValue> {
        self.engine
            .sysvars
            .get(name)
            .map(|v| v.display())
            .ok_or_else(|| JsValue::from_str("unknown sysvar"))
    }

    /// Set a system variable from a token (int/real/string).
    pub fn set_sysvar(&mut self, name: &str, value: &str) -> Result<(), JsValue> {
        let val = kelicad_cmd::parse_sysvar_token(value);
        self.engine
            .sysvars
            .set(name, val)
            .map_err(|e| JsValue::from_str(&e))?;
        self.engine.sync_from_sysvars();
        Ok(())
    }

    /// All sysvars as JSON object.
    pub fn sysvars_json(&self) -> String {
        serde_json::to_string(&self.engine.sysvars.to_json_map()).unwrap_or_else(|_| "{}".into())
    }

    /// Active command option chips JSON: `[{key,label},…]`.
    pub fn active_options_json(&self) -> String {
        let chips = self.engine.active_option_chips();
        let v: Vec<_> = chips
            .iter()
            .map(|c| serde_json::json!({"key": c.key, "label": c.label}))
            .collect();
        serde_json::to_string(&v).unwrap_or_else(|_| "[]".into())
    }

    /// Built-in command names JSON array.
    pub fn commands_catalog_json(&self) -> String {
        let names: Vec<_> = kelicad_cmd::CommandId::all()
            .iter()
            .map(|c| c.name())
            .collect();
        serde_json::to_string(&names).unwrap_or_else(|_| "[]".into())
    }

    /// Status bar JSON: layer, ortho, osnap, selection count, entity count.
    pub fn status_json(&self) -> String {
        let doc = &self.engine.session.document;
        let layer_name = doc
            .get_layer(doc.current_layer)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| "0".into());
        serde_json::json!({
            "layer": layer_name,
            "layer_id": doc.current_layer.to_string(),
            "ortho": self.engine.ortho,
            "osnap": self.engine.osnap_enabled,
            "grid": self.engine.sysvars.get_int("GRIDMODE") != 0,
            "polar": self.engine.sysvars.get_int("POLARMODE") != 0,
            "snap": self.engine.sysvars.get_int("SNAPMODE") != 0,
            "osmode": self.engine.sysvars.get_int("OSMODE"),
            "grid_unit": self.engine.sysvars.get_real("GRIDUNIT"),
            "snap_unit": self.engine.sysvars.get_real("SNAPUNIT"),
            "selection": self.engine.selection.len(),
            "entities": doc.entity_count(),
            "solids": self.solids.len(),
            "active": self.engine.active_command().map(|c| c.name()),
        })
        .to_string()
    }

    /// Interleaved line vertices for GPU: all line segments in the document
    /// plus polyline edges and circle tessellations (32 segments).
    pub fn scene_line_vertices(&self) -> Vec<f32> {
        let doc = &self.engine.session.document;
        let ltscale = self.engine.sysvars.get_real("LTSCALE").max(0.01);
        let mut out = Vec::new();
        for ent in doc.entities() {
            if let Some(layer) = doc.get_layer(ent.layer_id) {
                if layer.flags.frozen || layer.flags.off {
                    continue;
                }
            }
            let pattern = entity_linetype_pattern(doc, ent);
            append_geometry_lines(doc, &ent.geometry, &mut out, &pattern, ltscale);
        }
        out
    }

    /// Colored scene batches JSON for multi-pass GPU draw (layer / entity colors).
    /// Selected entities get a highlight batch (yellow) — AutoCAD/QCAD style.
    pub fn scene_batches_json(&self) -> String {
        use std::collections::{BTreeMap, HashSet};
        let doc = &self.engine.session.document;
        let ltscale = self.engine.sysvars.get_real("LTSCALE").max(0.01);
        let selected: HashSet<_> = self.engine.selection.iter().collect();
        let mut map: BTreeMap<[u8; 3], Vec<f32>> = BTreeMap::new();
        let mut sel_verts = Vec::new();
        let mut grip_verts = Vec::new();
        for ent in doc.entities() {
            if let Some(layer) = doc.get_layer(ent.layer_id) {
                if layer.flags.frozen || layer.flags.off {
                    continue;
                }
            }
            let pattern = entity_linetype_pattern(doc, ent);
            if selected.contains(&ent.id) {
                append_geometry_lines(doc, &ent.geometry, &mut sel_verts, &pattern, ltscale);
                append_entity_grips(doc, &ent.geometry, &mut grip_verts);
                continue;
            }
            let layer_color = doc
                .get_layer(ent.layer_id)
                .map(|l| l.color)
                .unwrap_or([140, 200, 255]);
            let color = ent.props.color.unwrap_or(layer_color);
            let bucket = map.entry(color).or_default();
            append_geometry_lines(doc, &ent.geometry, bucket, &pattern, ltscale);
        }
        let mut batches: Vec<SceneBatchDto> = map
            .into_iter()
            .map(|(c, verts)| SceneBatchDto {
                r: c[0] as f32 / 255.0,
                g: c[1] as f32 / 255.0,
                b: c[2] as f32 / 255.0,
                verts,
            })
            .collect();
        if !sel_verts.is_empty() {
            batches.push(SceneBatchDto {
                r: 1.0,
                g: 0.92,
                b: 0.2,
                verts: sel_verts,
            });
        }
        if !grip_verts.is_empty() {
            batches.push(SceneBatchDto {
                r: 1.0,
                g: 0.45,
                b: 0.1,
                verts: grip_verts,
            });
        }
        serde_json::to_string(&batches).unwrap_or_else(|_| "[]".into())
    }

    /// Reset to a new empty drawing (keeps collab identity).
    pub fn new_drawing(&mut self) {
        let ortho = self.engine.ortho;
        let osnap = self.engine.osnap_enabled;
        self.engine = CommandEngine::new(DocSession::new(), UuidV7Gen);
        self.engine.ortho = ortho;
        self.engine.osnap_enabled = osnap;
        self.last_prompt_fr.clear();
        self.solids.clear();
        self.bim.clear();
        self.constraints = ConstraintSystem::new();
    }

    /// Apply a paper template (`blank`/`a0`–`a4`/`mech`/`arch`). Returns entity count added.
    pub fn apply_template(&mut self, name: &str) -> Result<u32, JsValue> {
        let kind = TemplateKind::parse(name).ok_or_else(|| {
            JsValue::from_str("unknown template (blank|a0|a1|a2|a3|a4|mech|arch)")
        })?;
        if matches!(kind, TemplateKind::Blank) {
            self.new_drawing();
            return Ok(0);
        }
        let seeds = template_entities(kind);
        let mut layer_ids: std::collections::HashMap<String, LayerId> =
            std::collections::HashMap::new();
        // Keep existing layer 0; map by name.
        for l in self.engine.session.document.layers() {
            layer_ids.insert(l.name.clone(), l.id);
        }
        let mut ops = Vec::new();
        let mut n = 0u32;
        for seed in seeds {
            let layer_id = if let Some(id) = layer_ids.get(seed.layer) {
                *id
            } else {
                let id = LayerId::from_uuid(Uuid::now_v7());
                layer_ids.insert(seed.layer.to_string(), id);
                ops.push(Operation::CreateLayer {
                    layer: Layer::new(id, seed.layer, seed.layer_color),
                });
                id
            };
            let eid = EntityId::from_uuid(Uuid::now_v7());
            ops.push(Operation::CreateEntity {
                record: EntityRecord {
                    id: eid,
                    kind: seed.geometry.kind(),
                    geometry: seed.geometry,
                    layer_id,
                    props: Default::default(),
        },
            });
            n += 1;
        }
        self.engine
            .session
            .apply(Operation::Batch { ops })
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(n)
    }

    /// List block names as JSON.
    pub fn blocks_json(&self) -> String {
        let blocks: Vec<_> = self
            .engine
            .session
            .document
            .blocks()
            .map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "name": b.name,
                    "entity_count": b.entities.len(),
                })
            })
            .collect();
        serde_json::to_string(&blocks).unwrap_or_else(|_| "[]".into())
    }

    /// Ghost preview line vertices at world hover position.
    pub fn ghost_line_vertices(&mut self, hover_x: f64, hover_y: f64) -> Vec<f32> {
        let out = self
            .engine
            .handle(CommandEvent::Hover(Vec2::new(hover_x, hover_y)));
        self.remember(&out);
        let mut verts = Vec::new();
        let doc = &self.engine.session.document;
        let ltscale = self.engine.sysvars.get_real("LTSCALE").max(0.01);
        for g in &out.ghost {
            append_geometry_lines(doc, g, &mut verts, &[], ltscale);
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

    /// Detailed selection for properties panel.
    pub fn selection_detail_json(&self) -> String {
        let doc = &self.engine.session.document;
        let items: Vec<_> = self
            .engine
            .selection
            .iter()
            .filter_map(|id| {
                let ent = doc.get_entity(id)?;
                let layer = doc.get_layer(ent.layer_id).map(|l| l.name.clone());
                let aabb = geometry_aabb(&ent.geometry);
                Some(serde_json::json!({
                    "id": id.to_string(),
                    "kind": format!("{:?}", ent.kind).to_ascii_lowercase(),
                    "layer": layer,
                    "layer_id": ent.layer_id.to_string(),
                    "color": ent.props.color,
                    "linetype": ent.props.linetype,
                    "lineweight": ent.props.lineweight,
                    "bbox": {
                        "min": [aabb.min.x, aabb.min.y],
                        "max": [aabb.max.x, aabb.max.y],
                    },
                    "geometry": geometry_value(&ent.geometry),
                }))
            })
            .collect();
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".into())
    }

    /// Project properties JSON (units, title, scale).
    pub fn project_props_json(&self) -> String {
        let d = &self.engine.session.document;
        serde_json::json!({
            "units": d.units.as_str(),
            "title": d.project_title,
            "scale": d.project_scale,
        })
        .to_string()
    }

    /// Set project units (`mm`/`m`/`in`/`unitless`).
    pub fn set_project_units(&mut self, units: &str) -> Result<(), JsValue> {
        let u = DrawingUnits::parse(units).ok_or_else(|| JsValue::from_str("bad units"))?;
        self.engine.session.document.units = u;
        Ok(())
    }

    /// Set project title.
    pub fn set_project_title(&mut self, title: &str) {
        self.engine.session.document.project_title = title.to_string();
    }

    /// Set project scale label.
    pub fn set_project_scale(&mut self, scale: &str) {
        self.engine.session.document.project_scale = scale.to_string();
    }

    /// Templates list JSON.
    pub fn templates_json(&self) -> String {
        let list: Vec<_> = TemplateKind::all_named()
            .iter()
            .map(|k| {
                serde_json::json!({
                    "id": k.name(),
                    "name": k.name(),
                })
            })
            .collect();
        serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
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

    /// Contextual drawing impulses / proposals as JSON array.
    pub fn keli_impulses_json(&self) -> String {
        let sel = self.engine.selection.len();
        let imps = suggest_impulses(&self.engine.session, sel);
        serde_json::to_string(&imps).unwrap_or_else(|_| "[]".into())
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
                    props: Default::default(),
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
        self.push_solid(body, "EX", "height", &format!("{height}"))
    }

    /// Create a UV sphere solid.
    pub fn solid_sphere(&mut self, cx: f64, cy: f64, cz: f64, radius: f64) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .sphere(kelicad_math::Vec3::new(cx, cy, cz), radius, 24, 12)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "SPH", "radius", &format!("{radius}"))
    }

    /// Create a cone solid along +Z.
    pub fn solid_cone(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        height: f64,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .cone(Vec2::new(cx, cy), radius, height, 24)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "CONE", "height", &format!("{height}"))
    }

    /// Sweep selected closed profile along selected open polyline path.
    pub fn solid_sweep_selected(&mut self, thickness: f64) -> Result<String, JsValue> {
        let profile = find_closed_profile(&self.engine.session.document, &self.engine.selection)
            .ok_or_else(|| JsValue::from_str("need a closed profile in selection"))?;
        let path = find_open_path(&self.engine.session.document, &self.engine.selection)
            .ok_or_else(|| JsValue::from_str("need an open polyline path in selection"))?;
        let k = BuiltinSolidKernel;
        let body = k
            .sweep(&profile, &path, thickness)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "SWP", "thickness", &format!("{thickness}"))
    }

    /// Loft two closed profiles (first two closed polylines in selection) over height.
    pub fn solid_loft_selected(&mut self, height: f64) -> Result<String, JsValue> {
        let profiles = find_closed_profiles(&self.engine.session.document, &self.engine.selection, 2);
        if profiles.len() < 2 {
            return Err(JsValue::from_str("need 2 closed polylines to loft"));
        }
        let k = BuiltinSolidKernel;
        let body = k
            .loft(&profiles[0], &profiles[1], height, 6)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "LOFT", "height", &format!("{height}"))
    }

    /// Cylinder along +Z.
    pub fn solid_cylinder(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        height: f64,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .cylinder(Vec2::new(cx, cy), radius, height, 32)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "CYL", "height", &format!("{height}"))
    }

    /// Torus.
    pub fn solid_torus(
        &mut self,
        cx: f64,
        cy: f64,
        cz: f64,
        major: f64,
        minor: f64,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .torus(
                kelicad_math::Vec3::new(cx, cy, cz),
                major,
                minor,
                32,
                16,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "TOR", "major", &format!("{major}"))
    }

    /// Regular pyramid.
    pub fn solid_pyramid(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        height: f64,
        sides: u32,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .pyramid(Vec2::new(cx, cy), radius, height, sides.max(3) as usize)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "PYR", "height", &format!("{height}"))
    }

    /// Regular prism.
    pub fn solid_prism(
        &mut self,
        cx: f64,
        cy: f64,
        radius: f64,
        height: f64,
        sides: u32,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .prism(Vec2::new(cx, cy), radius, height, sides.max(3) as usize)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "PRISM", "height", &format!("{height}"))
    }

    /// Hollow pipe (outer − inner cylinder).
    pub fn solid_pipe(
        &mut self,
        cx: f64,
        cy: f64,
        outer: f64,
        inner: f64,
        height: f64,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let body = k
            .pipe(Vec2::new(cx, cy), outer, inner, height, 32)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "PIPE", "height", &format!("{height}"))
    }

    /// Revolve selected closed profile around +Y by degrees.
    pub fn solid_revolve_selected(&mut self, degrees: f64) -> Result<String, JsValue> {
        let profile = find_closed_profile(&self.engine.session.document, &self.engine.selection)
            .ok_or_else(|| JsValue::from_str("need a closed profile to revolve"))?;
        let k = BuiltinSolidKernel;
        let body = k
            .revolve(&profile, degrees, 36)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.push_solid(body, "REV", "degrees", &format!("{degrees}"))
    }

    fn push_solid(
        &mut self,
        body: SolidBody,
        tag_prefix: &str,
        prop_key: &str,
        prop_val: &str,
    ) -> Result<String, JsValue> {
        let elem = BimElement::new(kelicad_bim::BimKind::Wall, body.id)
            .with_prop("tag", format!("{tag_prefix}-{}", self.solids.len() + 1))
            .with_prop(prop_key, prop_val);
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
                    props: Default::default(),
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

    /// All solid meshes as JSON for the 3D viewport (positions xyz interleaved + indices).
    pub fn solids_meshes_json(&self) -> String {
        let meshes: Vec<_> = self
            .solids
            .iter()
            .map(|b| {
                let positions: Vec<f32> = b
                    .mesh
                    .positions
                    .iter()
                    .flat_map(|p| [p.x as f32, p.y as f32, p.z as f32])
                    .collect();
                serde_json::json!({
                    "id": b.id.to_string(),
                    "positions": positions,
                    "indices": b.mesh.indices,
                    "triangles": b.mesh.triangle_count(),
                    "aabb": {
                        "min": [b.aabb.min.x, b.aabb.min.y, b.aabb.min.z],
                        "max": [b.aabb.max.x, b.aabb.max.y, b.aabb.max.z],
                    },
                })
            })
            .collect();
        serde_json::to_string(&meshes).unwrap_or_else(|_| "[]".into())
    }

    /// Clear all solids (and linked BIM stubs created with them).
    pub fn solids_clear(&mut self) {
        self.solids.clear();
        self.bim.clear();
    }

    /// Boolean on the last two solids: `union` | `difference` | `intersection`.
    /// Replaces them with the result (keeps earlier solids).
    pub fn solid_boolean(&mut self, op: &str) -> Result<String, JsValue> {
        if self.solids.len() < 2 {
            return Err(JsValue::from_str("need ≥2 solids for boolean"));
        }
        let b = self.solids.pop().unwrap();
        let a = self.solids.pop().unwrap();
        let k = BuiltinSolidKernel;
        let body = match op.trim().to_ascii_lowercase().as_str() {
            "union" | "u" | "+" => k.boolean_union(&a, &b),
            "difference" | "diff" | "sub" | "-" => k.boolean_difference(&a, &b),
            "intersection" | "intersect" | "i" | "*" => k.boolean_intersection(&a, &b),
            other => {
                self.solids.push(a);
                self.solids.push(b);
                return Err(JsValue::from_str(&format!(
                    "unknown boolean op {other} (union|difference|intersection)"
                )));
            }
        }
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
        // Drop last two BIM entries if present (best-effort pairing).
        if self.bim.len() >= 2 {
            self.bim.truncate(self.bim.len() - 2);
        }
        self.push_solid(body, "BOOL", "op", op)
    }

    /// Create a demo box solid at origin (quick 3D smoke).
    pub fn solid_box(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        w: f64,
        d: f64,
        h: f64,
    ) -> Result<String, JsValue> {
        let k = BuiltinSolidKernel;
        let profile = Profile2::rectangle(Vec2::new(x, y), w.max(1e-6), d.max(1e-6))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut body = k
            .extrude(&profile, h.max(1e-6))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        if z.abs() > 1e-12 {
            body.mesh = body.mesh.translated(kelicad_math::Vec3::new(0.0, 0.0, z));
            body.aabb = body.mesh.aabb();
        }
        self.push_solid(body, "BOX", "size", &format!("{w}x{d}x{h}"))
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
        Geometry::Leader(l) => serde_json::json!({
            "type":"leader","vertices": l.vertices.iter().map(|v| [v.x,v.y]).collect::<Vec<_>>(),
            "text": l.text, "text_height": l.text_height
        }),
        Geometry::Ray(r) => serde_json::json!({
            "type":"ray","ox":r.origin.x,"oy":r.origin.y,"dx":r.direction.x,"dy":r.direction.y
        }),
        Geometry::XLine(x) => serde_json::json!({
            "type":"xline","x":x.point.x,"y":x.point.y,"dx":x.direction.x,"dy":x.direction.y
        }),
        Geometry::Xref(x) => serde_json::json!({
            "type":"xref","x":x.position.x,"y":x.position.y,"xref_id":x.xref_id.to_string()
        }),
    }
}

fn entity_linetype_pattern(doc: &Document, ent: &EntityRecord) -> Vec<f64> {
    let name = ent
        .props
        .linetype
        .as_deref()
        .or_else(|| doc.get_layer(ent.layer_id).map(|l| l.linetype.as_str()))
        .unwrap_or("CONTINUOUS");
    resolve_linetype(name).pattern
}

fn push_seg(out: &mut Vec<f32>, a: Vec2, b: Vec2) {
    out.extend_from_slice(&[a.x as f32, a.y as f32, b.x as f32, b.y as f32]);
}

fn push_linetype_seg(out: &mut Vec<f32>, a: Vec2, b: Vec2, pattern: &[f64], scale: f64) {
    if pattern.is_empty() {
        push_seg(out, a, b);
        return;
    }
    let v = b - a;
    let len = v.length();
    if len < 1e-12 {
        return;
    }
    let dir = v * (1.0 / len);
    let mut dist = 0.0;
    let mut i = 0usize;
    while dist < len - 1e-12 {
        let raw = pattern[i % pattern.len()];
        i += 1;
        let mut seg_len = raw.abs() * scale.max(1e-6);
        if raw.abs() < 1e-12 {
            seg_len = 0.08 * scale.max(1e-6);
        }
        let end = (dist + seg_len).min(len);
        if raw >= 0.0 {
            push_seg(out, a + dir * dist, a + dir * end);
        }
        dist = end;
    }
}

fn append_grip_square(out: &mut Vec<f32>, p: Vec2, s: f64) {
    let hx = s * 0.5;
    let a = Vec2::new(p.x - hx, p.y - hx);
    let b = Vec2::new(p.x + hx, p.y - hx);
    let c = Vec2::new(p.x + hx, p.y + hx);
    let d = Vec2::new(p.x - hx, p.y + hx);
    push_seg(out, a, b);
    push_seg(out, b, c);
    push_seg(out, c, d);
    push_seg(out, d, a);
}

fn append_entity_grips(doc: &Document, g: &Geometry, out: &mut Vec<f32>) {
    let s = 0.18;
    match g {
        Geometry::Line(l) => {
            append_grip_square(out, l.start, s);
            append_grip_square(out, l.end, s);
            append_grip_square(out, l.start.lerp(l.end, 0.5), s);
        }
        Geometry::Circle(c) => {
            append_grip_square(out, c.center, s);
            append_grip_square(out, c.center + Vec2::new(c.radius, 0.0), s);
        }
        Geometry::Arc(a) => {
            append_grip_square(out, a.center, s);
            append_grip_square(out, a.start_point(), s);
            append_grip_square(out, a.end_point(), s);
        }
        Geometry::Polyline(p) => {
            for v in &p.vertices {
                append_grip_square(out, *v, s);
            }
        }
        Geometry::Insert(i) => {
            append_grip_square(out, i.position, s);
            if let Some(def) = doc.get_block(i.block_id) {
                let box_ = expand_insert(def, i)
                    .iter()
                    .fold(kelicad_geom::Aabb::EMPTY, |acc, eg| {
                        acc.union(geometry_aabb(eg))
                    });
                if box_.min.x.is_finite() {
                    append_grip_square(out, box_.min, s);
                    append_grip_square(out, box_.max, s);
                    append_grip_square(out, Vec2::new(box_.min.x, box_.max.y), s);
                    append_grip_square(out, Vec2::new(box_.max.x, box_.min.y), s);
                }
            }
        }
        other => {
            let b = geometry_aabb(other);
            if b.min.x.is_finite() && b.min.x <= b.max.x {
                append_grip_square(out, b.center(), s);
            }
        }
    }
}

fn append_geometry_lines(
    doc: &Document,
    g: &Geometry,
    out: &mut Vec<f32>,
    pattern: &[f64],
    ltscale: f64,
) {
    match g {
        Geometry::Line(l) => {
            push_linetype_seg(out, l.start, l.end, pattern, ltscale);
        }
        Geometry::Polyline(p) => {
            for seg in p.segments() {
                push_linetype_seg(out, seg.start, seg.end, pattern, ltscale);
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
            // Stick-font baseline + height box (readable without true glyph raster).
            let h = t.height.max(0.15);
            let w = (t.content.chars().count().max(1) as f64) * h * 0.55;
            let p = t.position;
            let (c, s) = t.rotation.sin_cos();
            let tip = Vec2::new(p.x + w * c, p.y + w * s);
            let up = Vec2::new(p.x - h * s, p.y + h * c);
            push_seg(out, p, tip);
            push_seg(out, p, up);
            // Tick marks per character.
            for i in 1..=t.content.chars().count().min(24) {
                let t0 = (i as f64) / (t.content.chars().count().max(1) as f64);
                let along = p.lerp(tip, t0);
                let mark = Vec2::new(along.x - h * 0.35 * s, along.y + h * 0.35 * c);
                push_seg(out, along, mark);
            }
        }
        Geometry::MText(t) => {
            let h = t.height.max(0.15);
            let lines = t.content.lines().count().max(1);
            let w = t
                .width
                .max((t.content.chars().count().max(1) as f64) * h * 0.45);
            let p = t.position;
            push_seg(out, p, Vec2::new(p.x + w, p.y));
            push_seg(out, p, Vec2::new(p.x, p.y + h * lines as f64));
            push_seg(
                out,
                Vec2::new(p.x + w, p.y),
                Vec2::new(p.x + w, p.y + h * lines as f64),
            );
        }
        Geometry::Dimension(d) => {
            // Extension lines + dimension line + arrowheads (AutoCAD-ish).
            push_seg(out, d.def1, d.dim_line);
            push_seg(out, d.def2, d.dim_line);
            // Parallel dim line through dim_line spanning projected ends.
            let dir = (d.def2 - d.def1).try_normalize(kelicad_math::Tolerance::DEFAULT)
                .unwrap_or(Vec2::X);
            let nrm = dir.perp();
            let offset = (d.dim_line - d.def1).dot(nrm);
            let a = d.def1 + nrm * offset;
            let b = d.def2 + nrm * offset;
            push_seg(out, a, b);
            let ah = a.distance(b).max(1e-6) * 0.08;
            let ah = ah.clamp(0.05, 0.4);
            let tip_a = a;
            let tip_b = b;
            push_seg(out, tip_a, tip_a + dir * ah + nrm * ah * 0.4);
            push_seg(out, tip_a, tip_a + dir * ah - nrm * ah * 0.4);
            push_seg(out, tip_b, tip_b - dir * ah + nrm * ah * 0.4);
            push_seg(out, tip_b, tip_b - dir * ah - nrm * ah * 0.4);
            // Text tick at midpoint.
            let mid = a.lerp(b, 0.5);
            push_seg(out, mid, mid + nrm * ah * 1.2);
        }
        Geometry::Hatch(h) => {
            for seg in h.boundary.windows(2) {
                push_seg(out, seg[0], seg[1]);
            }
            if let (Some(first), Some(last)) = (h.boundary.first(), h.boundary.last()) {
                if first.distance(*last) > 1e-9 {
                    push_seg(out, *last, *first);
                }
            }
            // Diagonal fill pattern (ANSI31-ish cheat).
            if h.boundary.len() >= 3 {
                let min_x = h.boundary.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                let max_x = h.boundary.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                let min_y = h.boundary.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                let max_y = h.boundary.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                let step = ((max_x - min_x) + (max_y - min_y)).max(1.0) / 12.0;
                let mut x = min_x - (max_y - min_y);
                while x < max_x + (max_y - min_y) {
                    push_seg(
                        out,
                        Vec2::new(x, min_y),
                        Vec2::new(x + (max_y - min_y), max_y),
                    );
                    x += step;
                }
            }
            for island in &h.islands {
                for seg in island.windows(2) {
                    push_seg(out, seg[0], seg[1]);
                }
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
            if let Some(def) = doc.get_block(i.block_id) {
                for eg in expand_insert(def, i) {
                    append_geometry_lines(doc, &eg, out, pattern, ltscale);
                }
            } else {
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

        Geometry::Leader(l) => {
            for w in l.vertices.windows(2) {
                out.extend_from_slice(&[
                    w[0].x as f32, w[0].y as f32, w[1].x as f32, w[1].y as f32,
                ]);
            }
        }
        Geometry::Ray(r) => {
            let tip = r.point_at(1000.0);
            out.extend_from_slice(&[
                r.origin.x as f32, r.origin.y as f32, tip.x as f32, tip.y as f32,
            ]);
        }
        Geometry::XLine(x) => {
            let a = x.point - x.direction * 1000.0;
            let b = x.point + x.direction * 1000.0;
            out.extend_from_slice(&[a.x as f32, a.y as f32, b.x as f32, b.y as f32]);
        }
        Geometry::Xref(x) => {
            let s = 0.25f64;
            let p = x.position;
            out.extend_from_slice(&[
                (p.x - s) as f32, p.y as f32, (p.x + s) as f32, p.y as f32,
            ]);
        }

    }
}

fn find_closed_profile(
    doc: &kelicad_doc::Document,
    selection: &kelicad_cmd::Selection,
) -> Option<Profile2> {
    find_closed_profiles(doc, selection, 1).into_iter().next()
}

fn find_closed_profiles(
    doc: &kelicad_doc::Document,
    selection: &kelicad_cmd::Selection,
    limit: usize,
) -> Vec<Profile2> {
    let ids: Vec<EntityId> = if selection.is_empty() {
        doc.entities().map(|e| e.id).collect()
    } else {
        selection.to_vec()
    };
    let mut out = Vec::new();
    for id in ids {
        if out.len() >= limit {
            break;
        }
        let Some(ent) = doc.get_entity(id) else {
            continue;
        };
        if let Geometry::Polyline(p) = &ent.geometry {
            if p.closed && p.vertices.len() >= 3 {
                if let Ok(prof) = Profile2::from_vertices(p.vertices.clone()) {
                    out.push(prof);
                }
            }
        }
    }
    out
}

fn find_open_path(
    doc: &kelicad_doc::Document,
    selection: &kelicad_cmd::Selection,
) -> Option<Vec<Vec2>> {
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
            if !p.closed && p.vertices.len() >= 2 {
                return Some(p.vertices.clone());
            }
        }
    }
    None
}

fn parse_named_color(raw: &str) -> Option<[u8; 3]> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "red" | "1" => Some([255, 0, 0]),
        "yellow" | "2" => Some([255, 255, 0]),
        "green" | "3" => Some([0, 255, 0]),
        "cyan" | "4" => Some([0, 255, 255]),
        "blue" | "5" => Some([0, 0, 255]),
        "magenta" | "6" => Some([255, 0, 255]),
        "white" | "7" => Some([255, 255, 255]),
        other => {
            let parts: Vec<_> = other.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    parts[0].trim().parse::<u8>(),
                    parts[1].trim().parse::<u8>(),
                    parts[2].trim().parse::<u8>(),
                ) {
                    return Some([r, g, b]);
                }
            }
            None
        }
    }
}

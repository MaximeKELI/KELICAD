//! KELICAD desktop shell — native `.kelicad` IO via Tauri commands.

use std::path::PathBuf;

use kelicad_cmd::{CommandEngine, UuidV7Gen};
use kelicad_doc::{
    BlockDef, BlockId, Document, DrawingUnits, EntityId, EntityRecord, Geometry, Layer, LayerId,
};
use kelicad_io::{load_document, save_document};
use kelicad_math::Vec2;
use kelicad_ops::{DocSession, Operation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct NativeIoResult {
    path: String,
    snapshot_json: String,
    entities: u32,
    blocks: u32,
}

#[derive(Debug, Deserialize)]
struct SnapshotIn {
    current_layer: String,
    layers: Vec<LayerIn>,
    entities: Vec<EntityIn>,
    #[serde(default)]
    blocks: Vec<BlockIn>,
    #[serde(default)]
    units: Option<String>,
    #[serde(default)]
    project_title: Option<String>,
    #[serde(default)]
    project_scale: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LayerIn {
    id: String,
    name: String,
    color: [u8; 3],
}

#[derive(Debug, Deserialize)]
struct EntityIn {
    id: String,
    layer_id: String,
    geometry: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct BlockIn {
    id: String,
    name: String,
    origin: [f64; 2],
    entities: Vec<serde_json::Value>,
}

fn session_from_snapshot(json: &str) -> Result<DocSession, String> {
    let snap: SnapshotIn =
        serde_json::from_str(json).map_err(|e| format!("snapshot parse: {e}"))?;
    let mut session = DocSession::new();
    for layer in &snap.layers {
        let id = LayerId::from_uuid(
            Uuid::parse_str(&layer.id).map_err(|e| e.to_string())?,
        );
        if id != session.document.default_layer_id() {
            session
                .apply(Operation::CreateLayer {
                    layer: Layer::new(id, layer.name.clone(), layer.color),
                })
                .map_err(|e| e.to_string())?;
        } else {
            session
                .document
                .upsert_layer(Layer::new(id, layer.name.clone(), layer.color));
        }
    }
    for blk in &snap.blocks {
        let id = BlockId::from_uuid(Uuid::parse_str(&blk.id).map_err(|e| e.to_string())?);
        let mut ents = Vec::new();
        for g in &blk.entities {
            ents.push(geometry_from_value(g)?);
        }
        session
            .apply(Operation::UpsertBlock {
                def: BlockDef {
                    id,
                    name: blk.name.clone(),
                    origin: Vec2::new(blk.origin[0], blk.origin[1]),
                    entities: ents,
                    attributes: vec![],
                },
            })
            .map_err(|e| e.to_string())?;
    }
    for ent in &snap.entities {
        let id = EntityId::from_uuid(Uuid::parse_str(&ent.id).map_err(|e| e.to_string())?);
        let layer_id =
            LayerId::from_uuid(Uuid::parse_str(&ent.layer_id).map_err(|e| e.to_string())?);
        let geometry = geometry_from_value(&ent.geometry)?;
        session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: geometry.kind(),
                    geometry,
                    layer_id,
                },
            })
            .map_err(|e| e.to_string())?;
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
    session.journal = kelicad_ops::OpJournal::new();
    Ok(session)
}

fn snapshot_from_session(session: &DocSession) -> Result<String, String> {
    let doc = &session.document;
    let layers: Vec<_> = doc
        .layers()
        .map(|l| {
            serde_json::json!({
                "id": l.id.to_string(),
                "name": l.name,
                "color": l.color,
                "frozen": l.flags.frozen,
                "locked": l.flags.locked,
                "no_plot": l.flags.no_plot,
                "current": l.id == doc.current_layer,
            })
        })
        .collect();
    let entities: Vec<_> = doc
        .entities()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "kind": format!("{:?}", e.kind).to_ascii_lowercase(),
                "layer_id": e.layer_id.to_string(),
                "geometry": geometry_to_value(&e.geometry),
            })
        })
        .collect();
    let blocks: Vec<_> = doc
        .blocks()
        .map(|b| {
            serde_json::json!({
                "id": b.id.to_string(),
                "name": b.name,
                "origin": [b.origin.x, b.origin.y],
                "entities": b.entities.iter().map(geometry_to_value).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "schema_version": 0,
        "current_layer": doc.current_layer.to_string(),
        "layers": layers,
        "entities": entities,
        "blocks": blocks,
        "units": doc.units.as_str(),
        "project_title": doc.project_title,
        "project_scale": doc.project_scale,
    }))
    .map_err(|e| e.to_string())
}

fn geometry_to_value(g: &Geometry) -> serde_json::Value {
    // Round-trip via kelicad-io JSON string.
    match kelicad_io::geometry_json::geometry_to_json(g) {
        Ok(s) => serde_json::from_str(&s).unwrap_or(serde_json::Value::Null),
        Err(_) => serde_json::Value::Null,
    }
}

fn geometry_from_value(v: &serde_json::Value) -> Result<Geometry, String> {
    let s = serde_json::to_string(v).map_err(|e| e.to_string())?;
    kelicad_io::geometry_json::geometry_from_json(&s).map_err(|e| e.to_string())
}

/// Open a native `.kelicad` file and return a web-compatible snapshot JSON.
#[tauri::command]
fn open_kelicad(path: String) -> Result<NativeIoResult, String> {
    let path = PathBuf::from(path);
    let session = load_document(&path).map_err(|e| e.to_string())?;
    let snapshot_json = snapshot_from_session(&session)?;
    Ok(NativeIoResult {
        path: path.display().to_string(),
        entities: session.document.entity_count() as u32,
        blocks: session.document.blocks().count() as u32,
        snapshot_json,
    })
}

/// Save snapshot JSON to a native `.kelicad` path.
#[tauri::command]
fn save_kelicad(path: String, snapshot_json: String) -> Result<NativeIoResult, String> {
    let path = PathBuf::from(path);
    let session = session_from_snapshot(&snapshot_json)?;
    save_document(&path, &session).map_err(|e| e.to_string())?;
    Ok(NativeIoResult {
        path: path.display().to_string(),
        entities: session.document.entity_count() as u32,
        blocks: session.document.blocks().count() as u32,
        snapshot_json,
    })
}

/// Smoke: empty document entity count (proves core linkage).
#[tauri::command]
fn core_ping() -> String {
    let session = DocSession::new();
    let _eng = CommandEngine::new(session, UuidV7Gen);
    let _doc = Document::new();
    format!("kelicad-desktop ok entities={}", _eng.session.document.entity_count())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            open_kelicad,
            save_kelicad,
            core_ping
        ])
        .run(tauri::generate_context!())
        .expect("error while running KELICAD desktop");
}

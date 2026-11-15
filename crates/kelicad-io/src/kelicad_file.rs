//! Save / load `.kelicad` SQLite documents (schema v0).

use std::path::Path;

use kelicad_doc::document::EntityRecord;
use kelicad_doc::{Document, EntityId, Layer, LayerFlags, LayerId};
use kelicad_ops::DocSession;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::error::IoError;
use crate::geometry_json::{geometry_from_json, geometry_to_json};

/// Schema version written by this crate.
pub const SCHEMA_VERSION: u32 = 0;

/// Persist the current document state (materialized entities + layers).
///
/// M1 writes a snapshot of the derived document. Full oplog round-trip lands
/// when command ops are serialized (M1.1+).
pub fn save_document(path: &Path, session: &DocSession) -> Result<(), IoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    let tx = conn.transaction()?;
    init_schema(&tx)?;
    tx.execute_batch("DELETE FROM entities; DELETE FROM layers; DELETE FROM meta;")?;

    upsert_meta(&tx, "format", "kelicad")?;
    upsert_meta(&tx, "schema_version", &SCHEMA_VERSION.to_string())?;
    let doc_id = Uuid::now_v7();
    upsert_meta(&tx, "doc_id", &doc_id.to_string())?;
    upsert_meta(&tx, "tolerance_abs", "1e-9")?;

    for layer in session.document.layers() {
        tx.execute(
            "INSERT INTO layers (id, name, color_r, color_g, color_b, lineweight, frozen, locked, no_plot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                layer.id.as_uuid().as_bytes().as_slice(),
                layer.name,
                layer.color[0],
                layer.color[1],
                layer.color[2],
                layer.lineweight,
                layer.flags.frozen as i32,
                layer.flags.locked as i32,
                layer.flags.no_plot as i32,
            ],
        )?;
    }

    for ent in session.document.entities() {
        let geom = geometry_to_json(&ent.geometry)?;
        tx.execute(
            "INSERT INTO entities (id, kind, layer_id, geometry_json, deleted)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![
                ent.id.as_uuid().as_bytes().as_slice(),
                kind_str(ent.kind),
                ent.layer_id.as_uuid().as_bytes().as_slice(),
                geom,
            ],
        )?;
    }

    upsert_meta(
        &tx,
        "current_layer",
        &session.document.current_layer.as_uuid().to_string(),
    )?;

    tx.commit()?;
    Ok(())
}

/// Load a document into a fresh session (empty journal).
pub fn load_document(path: &Path) -> Result<DocSession, IoError> {
    let conn = Connection::open(path)?;
    let format: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'format'", [], |r| {
            r.get(0)
        })
        .map_err(|_| IoError::Format("missing meta.format".into()))?;
    if format != "kelicad" {
        return Err(IoError::Format(format!("unexpected format {format}")));
    }
    let schema: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| IoError::Format("missing schema_version".into()))?;
    if schema.parse::<u32>().unwrap_or(u32::MAX) != SCHEMA_VERSION {
        return Err(IoError::Format(format!(
            "unsupported schema_version {schema}"
        )));
    }

    let mut session = DocSession {
        document: Document::new(),
        journal: kelicad_ops::OpJournal::new(),
    };

    // Load layers
    {
        let mut stmt = conn.prepare(
            "SELECT id, name, color_r, color_g, color_b, lineweight, frozen, locked, no_plot FROM layers",
        )?;
        let rows = stmt.query_map([], |row| {
            let id_blob: Vec<u8> = row.get(0)?;
            Ok((
                id_blob,
                row.get::<_, String>(1)?,
                [
                    row.get::<_, u8>(2)?,
                    row.get::<_, u8>(3)?,
                    row.get::<_, u8>(4)?,
                ],
                row.get::<_, u16>(5)?,
                LayerFlags {
                    frozen: row.get::<_, i32>(6)? != 0,
                    locked: row.get::<_, i32>(7)? != 0,
                    no_plot: row.get::<_, i32>(8)? != 0,
                },
            ))
        })?;

        // Build fresh document without assuming only default layer.
        let mut fresh = Document::new();
        let mut seen_default = false;
        for row in rows {
            let (id_blob, name, color, lineweight, flags) = row?;
            let uuid = Uuid::from_slice(&id_blob)?;
            let id = LayerId::from_uuid(uuid);
            if id == fresh.default_layer_id() {
                seen_default = true;
            }
            let mut layer = Layer::new(id, name, color);
            layer.lineweight = lineweight;
            layer.flags = flags;
            fresh.upsert_layer(layer);
        }
        if !seen_default {
            return Err(IoError::Format("default layer 0 missing".into()));
        }
        session.document = fresh;
    }

    {
        let mut stmt =
            conn.prepare("SELECT id, layer_id, geometry_json FROM entities WHERE deleted = 0")?;
        let rows = stmt.query_map([], |row| {
            let id_blob: Vec<u8> = row.get(0)?;
            let layer_blob: Vec<u8> = row.get(1)?;
            let geom: String = row.get(2)?;
            Ok((id_blob, layer_blob, geom))
        })?;
        for row in rows {
            let (id_blob, layer_blob, geom_s) = row?;
            let id = EntityId::from_uuid(Uuid::from_slice(&id_blob)?);
            let layer_id = LayerId::from_uuid(Uuid::from_slice(&layer_blob)?);
            let geometry = geometry_from_json(&geom_s)?;
            session.document.upsert_entity(EntityRecord {
                id,
                kind: geometry.kind(),
                geometry,
                layer_id,
            });
        }
    }

    if let Ok(cur) = conn.query_row(
        "SELECT value FROM meta WHERE key = 'current_layer'",
        [],
        |r| r.get::<_, String>(0),
    ) {
        if let Ok(u) = Uuid::parse_str(&cur) {
            let _ = session.document.set_current_layer(LayerId::from_uuid(u));
        }
    }

    Ok(session)
}

fn kind_str(kind: kelicad_doc::EntityKind) -> &'static str {
    match kind {
        kelicad_doc::EntityKind::Point => "point",
        kelicad_doc::EntityKind::Line => "line",
        kelicad_doc::EntityKind::Arc => "arc",
        kelicad_doc::EntityKind::Circle => "circle",
        kelicad_doc::EntityKind::Polyline => "polyline",
        kelicad_doc::EntityKind::Text => "text",
        kelicad_doc::EntityKind::MText => "mtext",
        kelicad_doc::EntityKind::Dimension => "dimension",
        kelicad_doc::EntityKind::Hatch => "hatch",
        kelicad_doc::EntityKind::Insert => "insert",
    }
}

fn init_schema(conn: &Connection) -> Result<(), IoError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS layers (
            id BLOB PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            color_r INTEGER NOT NULL,
            color_g INTEGER NOT NULL,
            color_b INTEGER NOT NULL,
            lineweight INTEGER NOT NULL DEFAULT 0,
            frozen INTEGER NOT NULL DEFAULT 0,
            locked INTEGER NOT NULL DEFAULT 0,
            no_plot INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS entities (
            id BLOB PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            layer_id BLOB NOT NULL,
            geometry_json TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS oplog (
            seq INTEGER PRIMARY KEY NOT NULL,
            op_id BLOB NOT NULL,
            ts INTEGER NOT NULL,
            actor TEXT,
            payload TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY NOT NULL,
            seq_upto INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            blob BLOB NOT NULL
        );
        "#,
    )?;
    Ok(())
}

fn upsert_meta(conn: &Connection, key: &str, value: &str) -> Result<(), IoError> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kelicad_doc::{EntityKind, Geometry};
    use kelicad_geom::Line;
    use kelicad_math::Vec2;
    use kelicad_ops::Operation;
    use tempfile::NamedTempFile;

    #[test]
    fn round_trip_line() {
        let mut session = DocSession::new();
        let id = EntityId::from_uuid(Uuid::from_u128(42));
        session
            .apply(Operation::CreateEntity {
                record: EntityRecord {
                    id,
                    kind: EntityKind::Line,
                    geometry: Geometry::Line(Line::new(Vec2::ZERO, Vec2::new(3.0, 4.0))),
                    layer_id: session.document.default_layer_id(),
                },
            })
            .unwrap();

        let file = NamedTempFile::new().unwrap();
        let path = file.path().with_extension("kelicad");
        save_document(&path, &session).unwrap();
        let loaded = load_document(&path).unwrap();
        assert_eq!(loaded.document.entity_count(), 1);
        let ent = loaded.document.get_entity(id).unwrap();
        match &ent.geometry {
            Geometry::Line(l) => {
                assert_eq!(l.end, Vec2::new(3.0, 4.0));
            }
            _ => panic!("expected line"),
        }
    }
}

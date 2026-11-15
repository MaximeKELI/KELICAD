//! Named restore points and entity-set diffs.

use std::collections::{BTreeMap, BTreeSet};

use kelicad_doc::{EntityId, Geometry};
use kelicad_ops::DocSession;
use serde::{Deserialize, Serialize};

/// Named restore point metadata + snapshot JSON of entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RestorePoint {
    /// User-facing name.
    pub name: String,
    /// Fingerprint of applied op ids (sorted join).
    pub oplog_fingerprint: String,
    /// Snapshot JSON (entity list).
    pub snapshot_json: String,
}

/// Diff between two entity sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityDiff {
    /// Added ids.
    pub added: Vec<EntityId>,
    /// Removed ids.
    pub removed: Vec<EntityId>,
    /// Changed ids (same id, different geometry/layer).
    pub changed: Vec<EntityId>,
}

/// Fingerprint from a list of op id strings.
pub fn fingerprint(op_ids: &[String]) -> String {
    let mut v = op_ids.to_vec();
    v.sort();
    v.join(",")
}

/// Serialize a simple entity snapshot for restore points.
pub fn snapshot_session(session: &DocSession) -> Result<String, serde_json::Error> {
    let mut entities: Vec<_> = session
        .document
        .entities()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "layer_id": e.layer_id,
                "kind": e.kind,
                "geometry": e.geometry,
            })
        })
        .collect();
    entities.sort_by(|a, b| {
        a["id"]
            .as_str()
            .unwrap_or("")
            .cmp(b["id"].as_str().unwrap_or(""))
    });
    serde_json::to_string(&serde_json::json!({ "entities": entities }))
}

/// Diff two snapshots produced by [`snapshot_session`].
pub fn diff_snapshots(a_json: &str, b_json: &str) -> Result<EntityDiff, serde_json::Error> {
    let a: SnapshotFile = serde_json::from_str(a_json)?;
    let b: SnapshotFile = serde_json::from_str(b_json)?;
    let map_a: BTreeMap<EntityId, SnapEnt> =
        a.entities.into_iter().map(|e| (e.id, e)).collect();
    let map_b: BTreeMap<EntityId, SnapEnt> =
        b.entities.into_iter().map(|e| (e.id, e)).collect();
    let keys_a: BTreeSet<_> = map_a.keys().copied().collect();
    let keys_b: BTreeSet<_> = map_b.keys().copied().collect();
    let added: Vec<_> = keys_b.difference(&keys_a).copied().collect();
    let removed: Vec<_> = keys_a.difference(&keys_b).copied().collect();
    let mut changed = Vec::new();
    for id in keys_a.intersection(&keys_b) {
        if map_a.get(id) != map_b.get(id) {
            changed.push(*id);
        }
    }
    Ok(EntityDiff {
        added,
        removed,
        changed,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SnapshotFile {
    entities: Vec<SnapEnt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SnapEnt {
    id: EntityId,
    layer_id: kelicad_doc::LayerId,
    kind: kelicad_doc::EntityKind,
    geometry: Geometry,
}

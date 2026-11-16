//! Last-writer-wins target keys and registry.

use std::collections::HashMap;

use kelicad_doc::{BlockId, EntityId, LayerId, LayoutId, XrefId};
use kelicad_ops::Operation;
use serde::{Deserialize, Serialize};

use crate::envelope::HybridTime;

/// Conflict target for LWW.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKey {
    /// Entity create/update/delete.
    Entity(EntityId),
    /// Layer create/delete.
    Layer(LayerId),
    /// Current layer pointer.
    CurrentLayer,
    /// Block definition.
    Block(BlockId),
    /// Paper layout.
    Layout(LayoutId),
    /// External reference definition.
    Xref(XrefId),
}

/// Extract LWW targets touched by an operation.
pub fn targets_of(op: &Operation) -> Vec<TargetKey> {
    match op {
        Operation::CreateEntity { record } => vec![TargetKey::Entity(record.id)],
        Operation::SetGeometry { id, .. }
        | Operation::SetEntityLayer { id, .. }
        | Operation::SetEntityProps { id, .. } => {
            vec![TargetKey::Entity(*id)]
        }
        Operation::DeleteEntity { id } => vec![TargetKey::Entity(*id)],
        Operation::CreateLayer { layer } | Operation::UpdateLayer { layer } => {
            vec![TargetKey::Layer(layer.id)]
        }
        Operation::DeleteLayer { id } => vec![TargetKey::Layer(*id)],
        Operation::SetCurrentLayer { .. } => vec![TargetKey::CurrentLayer],
        Operation::UpsertBlock { def } => vec![TargetKey::Block(def.id)],
        Operation::DeleteBlock { id } => vec![TargetKey::Block(*id)],
        Operation::UpsertLayout { layout } => vec![TargetKey::Layout(layout.id)],
        Operation::DeleteLayout { id } => vec![TargetKey::Layout(*id)],
        Operation::UpsertXref { def } => vec![TargetKey::Xref(def.id)],
        Operation::DeleteXref { id } => vec![TargetKey::Xref(*id)],
        Operation::Batch { ops } => {
            let mut keys = Vec::new();
            for child in ops {
                keys.extend(targets_of(child));
            }
            keys
        }
    }
}

/// LWW win table + entity tombstones.
#[derive(Debug, Clone, Default)]
pub struct LwwTable {
    wins: HashMap<TargetKey, HybridTime>,
    /// Deleted entity ids with winning delete time.
    tombstones: HashMap<EntityId, HybridTime>,
}

impl LwwTable {
    /// Empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// True if `time` wins over existing (or no prior).
    pub fn would_apply(&self, key: &TargetKey, time: HybridTime) -> bool {
        match self.wins.get(key) {
            None => true,
            Some(prev) => time >= *prev,
        }
    }

    /// True if every target of `op` would apply under `time`.
    pub fn would_apply_op(&self, op: &Operation, time: HybridTime) -> bool {
        targets_of(op).iter().all(|k| self.would_apply(k, time))
    }

    /// Record wins for an applied op.
    pub fn record(&mut self, op: &Operation, time: HybridTime) {
        match op {
            Operation::Batch { ops } => {
                for child in ops {
                    self.record(child, time);
                }
            }
            Operation::DeleteEntity { id } => {
                self.wins.insert(TargetKey::Entity(*id), time);
                self.tombstones.insert(*id, time);
            }
            Operation::CreateEntity { record } => {
                self.wins.insert(TargetKey::Entity(record.id), time);
                self.tombstones.remove(&record.id);
            }
            other => {
                for key in targets_of(other) {
                    self.wins.insert(key, time);
                }
            }
        }
    }

    /// True if a create at `time` is blocked by a newer tombstone.
    pub fn is_tombstoned(&self, id: EntityId, time: HybridTime) -> bool {
        self.tombstones.get(&id).is_some_and(|t| *t > time)
    }
}

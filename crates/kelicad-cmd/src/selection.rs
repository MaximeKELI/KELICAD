//! Entity selection set.

use std::collections::BTreeSet;

use kelicad_doc::EntityId;

/// Ordered selection (stable iteration).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    ids: BTreeSet<EntityId>,
}

impl Selection {
    /// Empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of selected entities.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Clear.
    pub fn clear(&mut self) {
        self.ids.clear();
    }

    /// Replace with a single id.
    pub fn set_single(&mut self, id: EntityId) {
        self.ids.clear();
        self.ids.insert(id);
    }

    /// Toggle membership.
    pub fn toggle(&mut self, id: EntityId) {
        if !self.ids.remove(&id) {
            self.ids.insert(id);
        }
    }

    /// Add id.
    pub fn add(&mut self, id: EntityId) {
        self.ids.insert(id);
    }

    /// Remove id.
    pub fn remove(&mut self, id: EntityId) {
        self.ids.remove(&id);
    }

    /// Contains?
    pub fn contains(&self, id: EntityId) -> bool {
        self.ids.contains(&id)
    }

    /// Iterate ids.
    pub fn iter(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.ids.iter().copied()
    }

    /// Collect as vec.
    pub fn to_vec(&self) -> Vec<EntityId> {
        self.ids.iter().copied().collect()
    }
}

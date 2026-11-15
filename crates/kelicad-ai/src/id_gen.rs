//! Injectable entity id generation for stable AI plans.

use kelicad_doc::{BlockId, EntityId, LayerId};
use uuid::Uuid;

/// Generates stable ids for planned entities.
pub trait IdGen {
    /// Next entity id.
    fn next_entity(&mut self) -> EntityId;
    /// Next block id.
    fn next_block(&mut self) -> BlockId;
    /// Next layer id.
    fn next_layer(&mut self) -> LayerId;
}

/// Deterministic sequential UUID generator (tests / evals).
#[derive(Debug, Clone)]
pub struct SequentialIdGen {
    next: u128,
}

impl SequentialIdGen {
    /// Start at `start` (non-zero recommended).
    pub fn new(start: u128) -> Self {
        Self { next: start }
    }
}

impl Default for SequentialIdGen {
    fn default() -> Self {
        Self::new(1)
    }
}

impl IdGen for SequentialIdGen {
    fn next_entity(&mut self) -> EntityId {
        let id = EntityId::from_uuid(Uuid::from_u128(self.next));
        self.next = self.next.wrapping_add(1);
        id
    }

    fn next_block(&mut self) -> BlockId {
        let id = BlockId::from_uuid(Uuid::from_u128(self.next));
        self.next = self.next.wrapping_add(1);
        id
    }

    fn next_layer(&mut self) -> LayerId {
        let id = LayerId::from_uuid(Uuid::from_u128(self.next));
        self.next = self.next.wrapping_add(1);
        id
    }
}

/// UUID v7 generator (desktop / wasm with `js` feature on uuid).
#[derive(Debug, Default, Clone)]
pub struct UuidV7Gen;

impl IdGen for UuidV7Gen {
    fn next_entity(&mut self) -> EntityId {
        EntityId::from_uuid(Uuid::now_v7())
    }

    fn next_block(&mut self) -> BlockId {
        BlockId::from_uuid(Uuid::now_v7())
    }

    fn next_layer(&mut self) -> LayerId {
        LayerId::from_uuid(Uuid::now_v7())
    }
}

//! Injectable entity id generation (no global clock in the core).

use kelicad_doc::EntityId;
use uuid::Uuid;

/// Generates stable entity ids.
pub trait IdGen {
    /// Next entity id.
    fn next(&mut self) -> EntityId;
}

/// Deterministic sequential UUID generator for tests.
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
    fn next(&mut self) -> EntityId {
        let id = EntityId::from_uuid(Uuid::from_u128(self.next));
        self.next = self.next.wrapping_add(1);
        id
    }
}

/// UUID v7 generator using the system clock (desktop / wasm with `js`).
#[derive(Debug, Default, Clone)]
pub struct UuidV7Gen;

impl IdGen for UuidV7Gen {
    fn next(&mut self) -> EntityId {
        EntityId::from_uuid(Uuid::now_v7())
    }
}

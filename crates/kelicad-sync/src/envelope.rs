//! Stamped operation envelopes for sync.

use kelicad_ops::Operation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Replica / actor identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub Uuid);

impl ActorId {
    /// Random actor id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Deterministic actor for tests.
    pub fn from_u128(v: u128) -> Self {
        Self(Uuid::from_u128(v))
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ActorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Unique operation id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpId(pub Uuid);

impl OpId {
    /// New op id.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Deterministic.
    pub fn from_u128(v: u128) -> Self {
        Self(Uuid::from_u128(v))
    }
}

impl Default for OpId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OpId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Lamport + actor clock for LWW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HybridTime {
    /// Lamport counter.
    pub lamport: u64,
    /// Actor that minted this time.
    pub actor: ActorId,
}

/// Wire envelope carrying one operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpEnvelope {
    /// Unique id (idempotency).
    pub op_id: OpId,
    /// Authoring replica.
    pub actor_id: ActorId,
    /// Lamport clock at authoring.
    pub lamport: u64,
    /// CAD operation payload.
    pub payload: Operation,
}

impl OpEnvelope {
    /// Hybrid time for this envelope.
    pub fn time(&self) -> HybridTime {
        HybridTime {
            lamport: self.lamport,
            actor: self.actor_id,
        }
    }
}

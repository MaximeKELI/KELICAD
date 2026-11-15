//! Sync replica: local create + remote integrate with LWW.

use std::collections::{HashMap, HashSet};

use kelicad_ops::{DocSession, Operation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::comments::{Comment, CommentId, CommentStore};
use crate::envelope::{ActorId, HybridTime, OpEnvelope, OpId};
use crate::lww::LwwTable;
use crate::presence::PresenceState;
use crate::version::{fingerprint, snapshot_session, EntityDiff, RestorePoint};

/// Sync errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyncError {
    /// Document apply failed.
    #[error("apply failed: {0}")]
    Apply(String),
    /// JSON error.
    #[error("json: {0}")]
    Json(String),
}

/// Result of integrating an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrateResult {
    /// Applied to document.
    Applied,
    /// Already had this op_id.
    Duplicate,
    /// Lost LWW race.
    SkippedLww,
    /// Blocked by tombstone.
    SkippedTombstone,
}

/// Collaborative replica wrapping a [`DocSession`].
#[derive(Debug, Clone)]
pub struct Replica {
    /// Actor id for this replica.
    pub actor_id: ActorId,
    /// Lamport clock.
    pub lamport: u64,
    /// Document session.
    pub session: DocSession,
    /// Applied op ids.
    applied: HashSet<UuidSet>,
    /// LWW table.
    lww: LwwTable,
    /// Outbox for network send.
    pub outbox: Vec<OpEnvelope>,
    /// Full log (for catch-up / fingerprint).
    pub log: Vec<OpEnvelope>,
    /// Local presence.
    pub presence: PresenceState,
    /// Peer presence by peer id.
    pub peers: HashMap<ActorId, PresenceState>,
    /// Comments.
    pub comments: CommentStore,
    /// Named restore points.
    pub versions: Vec<RestorePoint>,
}

/// Wrapper so HashSet works with OpId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UuidSet(uuid::Uuid);

impl Replica {
    /// New replica with empty document.
    pub fn new(actor_id: ActorId, name: impl Into<String>, color: [u8; 3]) -> Self {
        let mut presence = PresenceState::new(name, color);
        presence.peer_id = actor_id;
        Self {
            actor_id,
            lamport: 0,
            session: DocSession::new(),
            applied: HashSet::new(),
            lww: LwwTable::new(),
            outbox: Vec::new(),
            log: Vec::new(),
            presence,
            peers: HashMap::new(),
            comments: CommentStore::new(),
            versions: Vec::new(),
        }
    }

    /// Tick Lamport for a local event.
    fn tick(&mut self) -> u64 {
        self.lamport += 1;
        self.lamport
    }

    /// Observe remote Lamport.
    fn observe(&mut self, remote: u64) {
        if remote > self.lamport {
            self.lamport = remote;
        }
    }

    /// Apply a local operation: journal + envelope outbox.
    pub fn commit_local(&mut self, op: Operation) -> Result<OpEnvelope, SyncError> {
        let lamport = self.tick();
        let envelope = OpEnvelope {
            op_id: OpId::new(),
            actor_id: self.actor_id,
            lamport,
            payload: op.clone(),
        };
        self.session
            .apply(op)
            .map_err(|e| SyncError::Apply(e.to_string()))?;
        self.lww.record(&envelope.payload, envelope.time());
        self.applied.insert(UuidSet(envelope.op_id.0));
        self.log.push(envelope.clone());
        self.outbox.push(envelope.clone());
        Ok(envelope)
    }

    /// Drain outbox.
    pub fn drain_outbox(&mut self) -> Vec<OpEnvelope> {
        std::mem::take(&mut self.outbox)
    }

    /// Integrate a remote (or replayed) envelope.
    pub fn integrate(&mut self, envelope: &OpEnvelope) -> Result<IntegrateResult, SyncError> {
        if self.applied.contains(&UuidSet(envelope.op_id.0)) {
            return Ok(IntegrateResult::Duplicate);
        }
        self.observe(envelope.lamport);
        let time = envelope.time();

        if let Operation::CreateEntity { record } = &envelope.payload {
            if self.lww.is_tombstoned(record.id, time) {
                self.applied.insert(UuidSet(envelope.op_id.0));
                return Ok(IntegrateResult::SkippedTombstone);
            }
        }

        if !self.lww.would_apply_op(&envelope.payload, time) {
            self.applied.insert(UuidSet(envelope.op_id.0));
            return Ok(IntegrateResult::SkippedLww);
        }

        self.session
            .apply_remote(&envelope.payload)
            .map_err(|e| SyncError::Apply(e.to_string()))?;
        self.lww.record(&envelope.payload, time);
        self.applied.insert(UuidSet(envelope.op_id.0));
        self.log.push(envelope.clone());
        Ok(IntegrateResult::Applied)
    }

    /// Catch up from a list of envelopes (e.g. snapshot from server).
    pub fn catch_up(&mut self, envelopes: &[OpEnvelope]) -> Result<usize, SyncError> {
        let mut n = 0;
        let mut sorted = envelopes.to_vec();
        sorted.sort_by_key(|e| (e.lamport, e.actor_id));
        for env in &sorted {
            if matches!(self.integrate(env)?, IntegrateResult::Applied) {
                n += 1;
            }
        }
        Ok(n)
    }

    /// Update local presence and return it for broadcast.
    pub fn set_presence(
        &mut self,
        cursor: Option<kelicad_math::Vec2>,
        selection: Vec<kelicad_doc::EntityId>,
    ) -> PresenceState {
        self.presence.cursor = cursor;
        self.presence.selection = selection;
        self.presence.clone()
    }

    /// Ingest peer presence.
    pub fn upsert_peer(&mut self, state: PresenceState) {
        if state.peer_id != self.actor_id {
            self.peers.insert(state.peer_id, state);
        }
    }

    /// Add a comment locally.
    pub fn add_comment(
        &mut self,
        anchor: crate::comments::CommentAnchor,
        body: impl Into<String>,
    ) -> Comment {
        let lamport = self.tick();
        let time = HybridTime {
            lamport,
            actor: self.actor_id,
        };
        let comment = Comment {
            id: CommentId::new(),
            anchor,
            body: body.into(),
            author: self.actor_id,
            created: time,
            resolved: false,
        };
        self.comments.upsert(comment.clone(), time);
        comment
    }

    /// Resolve comment.
    pub fn resolve_comment(&mut self, id: CommentId) -> bool {
        let lamport = self.tick();
        self.comments.resolve(
            id,
            HybridTime {
                lamport,
                actor: self.actor_id,
            },
        )
    }

    /// Save a named restore point.
    pub fn save_version(&mut self, name: impl Into<String>) -> Result<RestorePoint, SyncError> {
        let ids: Vec<String> = self.log.iter().map(|e| e.op_id.to_string()).collect();
        let point = RestorePoint {
            name: name.into(),
            oplog_fingerprint: fingerprint(&ids),
            snapshot_json: snapshot_session(&self.session)
                .map_err(|e| SyncError::Json(e.to_string()))?,
        };
        self.versions.push(point.clone());
        Ok(point)
    }

    /// Diff two restore points by name.
    pub fn diff_versions(&self, a: &str, b: &str) -> Result<EntityDiff, SyncError> {
        let pa = self
            .versions
            .iter()
            .find(|v| v.name == a)
            .ok_or_else(|| SyncError::Json(format!("version {a} not found")))?;
        let pb = self
            .versions
            .iter()
            .find(|v| v.name == b)
            .ok_or_else(|| SyncError::Json(format!("version {b} not found")))?;
        crate::version::diff_snapshots(&pa.snapshot_json, &pb.snapshot_json)
            .map_err(|e| SyncError::Json(e.to_string()))
    }

    /// Stable document fingerprint for convergence tests.
    pub fn doc_fingerprint(&self) -> String {
        snapshot_session(&self.session).unwrap_or_default()
    }
}

/// Wire-friendly replica status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaStatus {
    /// Actor.
    pub actor_id: ActorId,
    /// Lamport.
    pub lamport: u64,
    /// Entity count.
    pub entity_count: usize,
    /// Log length.
    pub log_len: usize,
}

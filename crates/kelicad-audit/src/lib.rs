//! Hash-chained append-only audit log (ADR 0012).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Genesis previous hash (64 zero hex chars).
pub const GENESIS_PREV: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One audit event in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Actor subject (JWT `sub` or `anonymous`).
    pub actor: String,
    /// Action name (e.g. `room.create`).
    pub action: String,
    /// Resource id / path.
    pub resource: String,
    /// Unix timestamp seconds.
    pub ts: u64,
    /// Previous event hash (hex).
    pub prev_hash: String,
    /// This event hash (hex).
    pub hash: String,
    /// Optional JSON detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

/// Append-only log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditLog {
    events: Vec<AuditEvent>,
}

impl AuditLog {
    /// Empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterate events.
    pub fn events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Append an event; returns the stored event.
    pub fn append(
        &mut self,
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        ts: u64,
        detail: Option<serde_json::Value>,
    ) -> AuditEvent {
        let prev_hash = self
            .events
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| GENESIS_PREV.to_string());
        let actor = actor.into();
        let action = action.into();
        let resource = resource.into();
        let hash = hash_event(&actor, &action, &resource, ts, &prev_hash, &detail);
        let ev = AuditEvent {
            actor,
            action,
            resource,
            ts,
            prev_hash,
            hash,
            detail,
        };
        self.events.push(ev.clone());
        ev
    }

    /// Verify the entire chain.
    pub fn verify_chain(&self) -> Result<(), String> {
        let mut expected_prev = GENESIS_PREV.to_string();
        for (i, ev) in self.events.iter().enumerate() {
            if ev.prev_hash != expected_prev {
                return Err(format!("event {i}: prev_hash mismatch"));
            }
            let recomputed = hash_event(
                &ev.actor,
                &ev.action,
                &ev.resource,
                ev.ts,
                &ev.prev_hash,
                &ev.detail,
            );
            if recomputed != ev.hash {
                return Err(format!("event {i}: hash mismatch"));
            }
            expected_prev = ev.hash.clone();
        }
        Ok(())
    }

    /// Events for a given actor.
    pub fn for_actor(&self, actor: &str) -> Vec<AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Remove all events for actor (GDPR delete). Re-hash remaining chain.
    pub fn purge_actor(&mut self, actor: &str) {
        let kept: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.actor != actor)
            .cloned()
            .collect();
        self.events.clear();
        for ev in kept {
            self.append(ev.actor, ev.action, ev.resource, ev.ts, ev.detail);
        }
    }

    /// Serialize as JSONL.
    pub fn to_jsonl(&self) -> String {
        self.events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn hash_event(
    actor: &str,
    action: &str,
    resource: &str,
    ts: u64,
    prev_hash: &str,
    detail: &Option<serde_json::Value>,
) -> String {
    let detail_s = detail.as_ref().map(|d| d.to_string()).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(actor.as_bytes());
    hasher.update(b"|");
    hasher.update(action.as_bytes());
    hasher.update(b"|");
    hasher.update(resource.as_bytes());
    hasher.update(b"|");
    hasher.update(ts.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(detail_s.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_verifies_and_detects_tamper() {
        let mut log = AuditLog::new();
        log.append("alice", "room.create", "r1", 1, None);
        log.append("alice", "ops.append", "r1", 2, None);
        log.append("bob", "room.create", "r2", 3, None);
        assert!(log.verify_chain().is_ok());
        log.events[1].action = "hacked".into();
        assert!(log.verify_chain().is_err());
    }

    #[test]
    fn purge_actor_rebuilds_chain() {
        let mut log = AuditLog::new();
        log.append("alice", "a", "r", 1, None);
        log.append("bob", "b", "r", 2, None);
        log.append("alice", "c", "r", 3, None);
        log.purge_actor("alice");
        assert_eq!(log.len(), 1);
        assert_eq!(log.events()[0].actor, "bob");
        assert!(log.verify_chain().is_ok());
    }
}

#[cfg(test)]
mod rigorous;

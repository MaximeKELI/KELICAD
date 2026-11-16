//! Rigorous audit-chain integrity tests.

use super::*;

#[test]
fn empty_log_verifies() {
    let log = AuditLog::new();
    assert!(log.is_empty());
    assert!(log.verify_chain().is_ok());
}

#[test]
fn first_event_links_genesis() {
    let mut log = AuditLog::new();
    let ev = log.append("alice", "login", "sys", 42, None);
    assert_eq!(ev.prev_hash, GENESIS_PREV);
    assert_eq!(ev.hash.len(), 64);
    assert!(log.verify_chain().is_ok());
}

#[test]
fn detail_affects_hash() {
    let mut a = AuditLog::new();
    let mut b = AuditLog::new();
    a.append(
        "alice",
        "act",
        "r",
        1,
        Some(serde_json::json!({"k": 1})),
    );
    b.append(
        "alice",
        "act",
        "r",
        1,
        Some(serde_json::json!({"k": 2})),
    );
    assert_ne!(a.events()[0].hash, b.events()[0].hash);
}

#[test]
fn tampered_prev_hash_detected() {
    let mut log = AuditLog::new();
    log.append("a", "x", "r", 1, None);
    log.append("b", "y", "r", 2, None);
    log.events[1].prev_hash = GENESIS_PREV.to_string();
    assert!(log.verify_chain().is_err());
}

#[test]
fn for_actor_filters() {
    let mut log = AuditLog::new();
    log.append("alice", "a", "r", 1, None);
    log.append("bob", "b", "r", 2, None);
    log.append("alice", "c", "r", 3, None);
    assert_eq!(log.for_actor("alice").len(), 2);
    assert_eq!(log.for_actor("carol").len(), 0);
}

#[test]
fn purge_all_actors_empties() {
    let mut log = AuditLog::new();
    log.append("alice", "a", "r", 1, None);
    log.append("alice", "b", "r", 2, None);
    log.purge_actor("alice");
    assert!(log.is_empty());
    assert!(log.verify_chain().is_ok());
}

#[test]
fn jsonl_roundtrip_line_count() {
    let mut log = AuditLog::new();
    log.append("a", "x", "r", 1, None);
    log.append("b", "y", "r", 2, Some(serde_json::json!({"n": 1})));
    let jsonl = log.to_jsonl();
    assert_eq!(jsonl.lines().count(), 2);
    for line in jsonl.lines() {
        let ev: AuditEvent = serde_json::from_str(line).expect("jsonl line");
        assert!(!ev.hash.is_empty());
    }
}

#[test]
fn chain_hashes_are_deterministic() {
    let mut a = AuditLog::new();
    let mut b = AuditLog::new();
    for i in 0..5u64 {
        a.append("u", "act", "res", i, None);
        b.append("u", "act", "res", i, None);
    }
    assert_eq!(a.events().len(), b.events().len());
    for (ea, eb) in a.events().iter().zip(b.events().iter()) {
        assert_eq!(ea.hash, eb.hash);
        assert_eq!(ea.prev_hash, eb.prev_hash);
    }
}

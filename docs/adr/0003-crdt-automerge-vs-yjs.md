# ADR 0003 — CRDT candidate evaluation (Automerge vs Yjs)

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

Milestone M4 requires real-time co-editing with offline merge. The product
stores an immutable operation log; a CRDT (or CRDT-backed op sync) must
converge after concurrent edits and network partitions. Two leading options
evaluated were **Automerge** and **Yjs**.

## Decision

**Neither Automerge nor Yjs for the CAD document.** Ship a **native
operation-log sync** in `kelicad-sync` (see ADR 0008):

- Replicas exchange stamped `Operation` envelopes (`op_id`, `actor_id`, Lamport).
- Concurrent writes resolve with **last-writer-wins per target** ordered by
  `(lamport, actor_id)`.
- Presence and comments use the same WebSocket as side channels.

### Why not Automerge / Yjs (for now)

| Criterion | Finding |
|-----------|---------|
| Op mapping | Free-form CRDT docs poorly match typed `Operation` / geometry |
| Rust + WASM | Extra dependency and binary size without CAD-aware semantics |
| Convergence | Achievable with deterministic LWW on the existing journal |
| Presence | Custom awareness over WS is enough for M4 |

Automerge/Yjs remain candidates if a future milestone needs generic nested
JSON CRDT or a third-party ecosystem; revisit then.

## Consequences

- `kelicad-sync` implements Replica + LWW + presence/comments.
- No Automerge/Yjs dependency in the workspace for M4.
- Soft locks / per-layer locking deferred.

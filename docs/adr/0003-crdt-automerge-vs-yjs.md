# ADR 0003 — CRDT candidate evaluation (Automerge vs Yjs)

- **Status:** Proposed (no decision yet)
- **Date:** 2026-11-15
- **Deciders:** pending prototype results

## Context

Milestone M4 requires real-time co-editing with offline merge. The product
stores an immutable operation log; a CRDT (or CRDT-backed op sync) must
converge after concurrent edits and network partitions. Two leading options
are **Automerge** and **Yjs**.

This ADR records **evaluation criteria and a prototype plan only**. No library
is selected until the comparison runs.

## Evaluation criteria

| Criterion | What we measure |
|-----------|-----------------|
| Convergence | Deterministic merge of concurrent geometry ops |
| Op mapping | Fit with our `Operation` journal (not free-form JSON blobs) |
| Rust story | First-class Rust crate vs JS-primary + FFI |
| WASM size | Incremental cost in the web bundle |
| Perf | Merge latency for 10k–100k ops; memory |
| Presence | Cursors / awareness support or clean extension point |
| License | MIT/Apache-2.0 preferred |
| Maintenance | Release cadence, adopters, governance |

## Prototype plan

1. Model a minimal document slice (points + lines + layer id) as ops.
2. Implement the same scenario twice: Automerge (Rust) and Yjs (JS or y-crdt).
3. Run scripted concurrent edits (A inserts, B moves, offline A, reconnect).
4. Record: LOC glue, binary/bundle delta, merge time, conflict UX notes.
5. Publish results in a follow-up ADR (`0003b` or status → Accepted) and choose.

## Non-goals (for this ADR)

- Choosing Automerge or Yjs.
- Designing soft locks / per-layer locking (separate ADR when M4 starts).

## Consequences (until decided)

- `kelicad-sync` remains a stub; no CRDT dependency is pinned in the core.
- Local single-user undo/redo via `kelicad-ops` stays authoritative for M0–M3.

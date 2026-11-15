# ADR 0008 — Collaboration sync protocol (M4)

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

M4 requires real-time co-edit, offline catch-up, presence, and comments.
ADR 0003 selects a native operation-log approach.

## Decision

### Wire format (JSON text over WebSocket)

```json
{ "type": "op", "envelope": { "op_id": "...", "actor_id": "...", "lamport": 12, "payload": { ... } } }
{ "type": "presence", "state": { "peer_id": "...", "name": "...", "color": [r,g,b], "cursor": [x,y], "selection": [] } }
{ "type": "comment", "action": "upsert"|"resolve", "comment": { ... } }
{ "type": "hello", "peer_id": "...", "name": "...", "color": [r,g,b] }
{ "type": "snapshot", "ops": [ /* OpEnvelope[] */ ], "comments": [ /* Comment[] */ ] }
{ "type": "version_save", "name": "...", "snapshot_json": "..." }
{ "type": "ping" }
```

`payload` is the JSON encoding of `kelicad_ops::Operation`.

### Ordering / LWW

- Each replica owns an `actor_id` (UUID) and a Lamport clock.
- On local commit: `lamport = max(local, known) + 1`.
- Conflict target keys: entity id, layer id, block id, layout id, or `current_layer`.
- Winner: higher `(lamport, actor_id)` lexicographic.
- Deletes install tombstones so late creates cannot resurrect without a newer create.
- Integrate is idempotent on `op_id`.

### Identity (M4)

No OIDC. Peers identify with `peer_id` + display `name` + color. Rooms are
unguessable UUIDs created via `POST /rooms`.

## Consequences

- Gateway can be in-memory for M4.
- Local undo is not shared; remote ops do not enter the local undo stack.

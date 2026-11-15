# ADR 0004 — Native `.kelicad` format on SQLite

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

The native file must be open, inspectable, migratable, and able to hold an
operation journal plus periodic snapshots for fast open. CAD files are often
large; random access to layers, assets, and snapshots matters.

## Decision

Define `.kelicad` as a **SQLite database** file with a versioned schema
(`meta`, `entities`, `layers`, `blocks`, `styles`, `oplog`, `snapshots`,
`assets`, `comments`, …). Schema version + migrations are mandatory. The
human-readable specification lives in `formats/kelicad-spec.md` and is written
**before** the I/O implementation ships.

Source of truth at runtime remains the in-memory op journal; the file is a
durable encoding of ops and derived snapshots.

## Consequences

- One file format for desktop and (via sql.js / OPFS) browser persistence.
- Tooling can use standard SQLite clients for forensics and migrations.
- Writers must never corrupt the DB: transactional saves, backup-on-write later.

## Alternatives considered

- Custom binary / protobuf blob: rejected (weaker inspectability).
- JSON / NDJSON only: rejected for large drawings and partial reads.
- LMDB: rejected (weaker ubiquity and browser story vs SQLite/sql.js).

# ADR 0005 — Browser `.kelicad` writes via sql.js

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

ADR 0004 defines `.kelicad` as SQLite. Native I/O uses `rusqlite` (`kelicad-io`).
The browser WASM target cannot link bundled SQLite easily in M1.

## Decision

- **Native:** `kelicad-io` + rusqlite (authoritative round-trip tests).
- **Web M1:** export a JSON snapshot from `kelicad-wasm`, then materialize a
  SQLite `.kelicad` file with **sql.js** (BSD-style) in the web app.

## Consequences

- Same schema v0 for files produced in browser and desktop.
- sql.js loads its wasm from CDN in M1; later pin/vendor for offline.
- Full oplog persistence in-file remains a follow-up.

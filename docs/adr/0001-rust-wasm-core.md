# ADR 0001 — Rust core with WASM delivery

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

KELICAD must ship one deterministic CAD core on browser, desktop, CLI, and
server (conversion, AI tooling, tests). The core owns geometry, document
mutation, and commands. UI stacks differ per platform.

## Decision

Implement the core in **Rust** (edition 2021+) and expose it to the web via
**wasm-bindgen / wasm-pack**. Desktop and CLI link the same crates natively.
Default crate policy: `#![forbid(unsafe_code)]`; any `unsafe` requires an ADR
or module-level justification.

## Consequences

- One algorithm surface for intersections, ops, and undo across platforms.
- WASM binary size and JS glue become first-class budgets (see §21).
- TypeScript UI talks to the core only through `kelicad-wasm` (web) or Tauri
  commands (desktop) — never by reimplementing document mutation.

## Alternatives considered

- TypeScript-only geometry: rejected (perf, numeric robustness, shared server).
- Separate native and web cores: rejected (divergence risk).

# KELICAD — Product brief (living)

This file summarizes the product charter for contributors. Full milestone
criteria live in project docs; ADRs record structural decisions.

## Product

- **Name:** KELICAD
- **AI assistant:** KELI
- **Native extension:** `.kelicad`
- **Package prefix:** `kelicad-`
- **Default UI locale:** French (+ English i18n from day one)
- **Code / comments / identifiers:** English

## Goals vs AutoCAD strengths

Precision, command line, layers, blocks, dimensioning, standards — plus:

| Axis | Target |
|------|--------|
| Platforms | One core: browser WASM, desktop, tablet |
| Collaboration | Real-time co-edit, comments, presence, history/branches |
| AI | KELI: draw, edit, query, check, vectorize in natural language |
| Performance | 1M entities fluid; GPU render; near-instant open |
| Openness | Documented format, public API, sandboxed plugins, native DXF |
| Ergonomics | Command palette, dynamic input, ghost preview, touch/stylus |
| Traceability | Git-like plan versioning, full audit |

## Stack (frozen)

Rust core (`#![forbid(unsafe_code)]` unless justified), wgpu, WASM, React +
Vite + Zustand + Tailwind + Radix, Tauri 2, SQLite, CRDT (ADR 0003), Axum
backend later.

## Layer rules

UI → commands → extensions → document → geom → math.  
All document mutation via `Operation`. Core is I/O-free and clock-free.

## Milestone in progress

**M5 — Plugins & API.** See ADR 0009 (`kelicad-plugin`, CLI, OpenAPI). Prior: M0–M4
delivered.

# KELICAD

KELICAD is an open CAD platform for architecture, engineering, mechanical,
civil, and electrical design. One core runs in the browser (WASM), on desktop
(Tauri), and in services. The AI assistant is **KELI**. Native files use the
`.kelicad` extension.

Default UI language: **French**, with full English i18n. Source identifiers and
technical comments are in English.

## Status

Milestone **M8 — DWG, entreprise, marketplace**: pluggable DWG bridge, gateway
JWT/RBAC, hash-chained audit, GDPR export/delete, signed plugin catalog, on-prem
compose skeleton (ADR 0012).

## Repository layout

```
crates/     Rust core (math → geom → doc → ops → … → wasm)
apps/       web (React + Vite), desktop (Tauri), cli
services/   backend (later milestones)
sdk/        plugin SDKs
formats/    .kelicad specification + test corpus
docs/adr/   Architecture Decision Records
```

## Prerequisites

- Rust stable ≥ 1.75 (`rustup`)
- Node.js ≥ 20 + pnpm 9 (`corepack enable`)
- For the web GPU demo: a browser with WebGPU

## Quick start

```bash
# Rust core
cargo test
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Web app
pnpm install
pnpm --filter @kelicad/web dev
```

## License

MIT — see [LICENSE](LICENSE).

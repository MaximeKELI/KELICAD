# ADR 0002 — wgpu as the rendering backend

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

The canvas must hit ~60 FPS with large entity counts, support WebGPU in the
browser, and map to Vulkan / Metal / DX12 on desktop without maintaining three
render paths.

## Decision

Use **wgpu** as the sole GPU abstraction for M0+. Line shaders, picking, and
tile culling live in `kelicad-render` (and a thin M0 path in `kelicad-wasm` /
the web app until that crate lands fully).

## Consequences

- Web requires WebGPU-capable browsers; graceful fallback messaging is required.
- Shader code is shared; platform differences are confined to surface setup.
- Performance budgets (pan/zoom, picking) are measured against wgpu pipelines.

## Alternatives considered

- Canvas2D / SVG: rejected for 1M-entity target.
- Direct WebGL + separate native APIs: rejected (duplication).
- `glow` / raw GL: rejected (weaker desktop path uniformity).

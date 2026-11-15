# M0 render benchmark — 100 000 lines

## Goal

Validate that the M0 web canvas can display **100 000** line segments with
interactive **pan/zoom** and report FPS in-UI, targeting ~**60 FPS** on recent
desktop GPUs (budget §21).

## Implementation (M0)

| Piece | Location |
|-------|----------|
| Vertex generation (TS, mirrors Rust helper) | `apps/web/src/demoLines.ts` |
| Rust generator (WASM façade) | `kelicad-wasm::generate_demo_line_vertices` |
| WebGPU line-list renderer | `apps/web/src/gpu/lineRenderer.ts` |
| UI + FPS counter | `apps/web/src/App.tsx`, `CanvasViewport.tsx` |

M0 uses the **browser WebGPU API** directly in TypeScript. This is the same
GPU path that **wgpu** targets on the web (ADR 0002). Shared Rust `wgpu`
pipelines move into `kelicad-render` as that crate lands (post-M0).

## How to run

```bash
pnpm install
pnpm --filter @kelicad/web dev
```

Open the URL (default `http://localhost:5173`) in a WebGPU-capable browser
(Chrome/Edge recommended). Confirm:

1. Header shows `Lignes 100 000`.
2. FPS readout stabilizes near 60 on capable hardware while idle or panning.
3. Mouse drag pans; wheel zooms.

## Recording results

| Machine | Browser | FPS (idle) | FPS (pan) | Notes |
|---------|---------|------------|-----------|-------|
| _fill in_ | | | | |

## CI note

Automated GPU FPS is not asserted in GitHub Actions (no guaranteed GPU).
Unit tests cover deterministic vertex generation only (`demoLines.test.ts`).

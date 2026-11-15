# ADR 0010 — 2D constraints and deterministic vectorize (M6)

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

M6 requires a 2D constraint solver, compliance/cleanup/auto-dimension tools,
and vectorization of SVG/raster into editable entities. Critical checks must
remain deterministic in core (charter §14.4); vision models stay out of the
browser core.

## Decision

1. **`kelicad-constraints`:** constraint graph + iterative Gauss–Newton solver
   over line endpoints. Solver returns `Operation::SetGeometry` / `Batch`; it
   never mutates `Document` directly.
2. **Constraint kinds (M6):** `Coincident`, `Horizontal`, `Vertical`,
   `Parallel`, `Perpendicular`, `EqualLength`, `Distance` (driving). Status is
   `Solved`, `Underconstrained`, or `Inconsistent`.
3. **`kelicad-vectorize`:** deterministic SVG path/primitives → geometries;
   PNG threshold + marching-squares contours → polylines. No external vision API.
4. **`kelicad-ai`:** declared JSON rulesets for `run_check`; tools
   `cleanup_drawing` and `auto_dimension` emit ops via the existing Propose path.
5. **Gateway** exposes `POST /v1/vectorize`; CLI and WASM bind the same crate.

## Consequences

- Sketch UX drag-handles and full circle/tangent solvers are deferred.
- Cloud ML vectorization can wrap the same entity DTO later without changing
  the document mutation path.

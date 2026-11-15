# ADR 0011 — SolidKernel and BIM subset (M7)

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

M7 requires a 3D solid kernel, BIM objects, IFC, derived 2D views, and
schedules (charter §18). The stack names OpenCASCADE behind a replaceable
`SolidKernel`, but OCC is unsuitable for default WASM/CI.

## Decision

1. **`SolidKernel` trait** in `kelicad-solid`: extrude, revolve, primitives,
   boolean union/difference, section, AABB. Implementations are swappable.
2. **`BuiltinSolidKernel`:** pure-Rust triangle-mesh kernel (WASM-safe) for
   M7. OpenCASCADE may later implement the same trait behind a feature flag.
3. **`kelicad-bim`:** Wall/Slab/Door/Window/Stair elements referencing solids,
   CSV schedules, AABB collisions, IFC2x3-style STEP subset export/import.
4. **Derived views** project meshes to 2D `Geometry` and apply via existing
   `Operation` batches (associative regeneration is re-derive + replace).

## Consequences

- No OCC system dependency in default CI/WASM builds.
- IFC coverage is a deliberate subset (placements + typed products), not a
  full schema validator.
- PBR materials and full IFC4X3 remain deferred.

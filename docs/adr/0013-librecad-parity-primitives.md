# ADR 0013 — LibreCAD / AutoCAD-style primitives (MIT reimplementation)

## Status

Accepted (2026-11-16)

## Context

Public CAD references surveyed:

- **LibreCAD** (GitHub, GPLv2) — ellipse, spline, polygon, fillet/chamfer, hatch patterns, rich construction tools
- **FreeCAD TechDraw PAT** — AutoCAD-compatible `.pat` hatch definitions
- **AutoCAD** — no public source; only documented formats (DXF, PAT)

KELICAD is MIT-licensed; we must **not** copy GPL LibreCAD sources into the tree. Algorithms were reimplemented from public geometry / DXF / PAT specifications after inspecting LibreCAD headers for *feature surface* only (`/tmp/cad-ref`, not vendored).

## Decision

Add first-class:

1. `Ellipse`, `Spline` geometry (+ DXF/SVG/JSON/WASM)
2. Construction helpers: circle 3-point, regular polygon, tangents, fillet, chamfer, bisector
3. AutoCAD-compatible `.pat` parser + built-in MIT patterns (`ANSI31`, `NET`, …) and hatch segment generation
4. Commands: `POLYGON`, `ELLIPSE`, `SPLINE`, `CIRCLE3P`, `FILLET`, `CHAMFER`

## Consequences

- Document model and all `Geometry` match sites grew two variants.
- Hatch gains `scale` / `angle` (serde defaults for old files).

## Follow-up (implemented)

- Layer linetypes (`CONTINUOUS`, `DASHED`, …) + dash tessellation
- `.pat` file load (`assets/pat/kelicad.pat`) + CLI `hatch`
- MText DXF codes (`\P`, `\H`, `\L`, `\C`, …)
- Shapefile import (Point / PolyLine / Polygon) + CLI `shp`
- Commands `TANGENTPC` / `TANGENTCC`

Still deferred: custom linetype tables in DXF TABLES, DBF attributes for SHP, real ODA DWG worker.

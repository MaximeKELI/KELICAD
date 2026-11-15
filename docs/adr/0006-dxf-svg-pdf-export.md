# ADR 0006 — DXF native + SVG/PDF export crates

- **Status:** Accepted
- **Date:** 2026-11-15

## Context

M2 requires DXF import/export with an import report, plus SVG and PDF export.
DWG remains deferred to ODA (M8).

## Decision

1. **DXF:** original Rust parser/writer in `kelicad-io` (R12-oriented entities
   first: LINE, CIRCLE, ARC, LWPOLYLINE, TEXT, INSERT, HATCH solid/simple,
   DIMENSION linear). No Autodesk code or proprietary resources.
2. **SVG:** original writer in `kelicad-io` (paths + text).
3. **PDF:** `printpdf` 0.7.x (MIT) for vector pages; pin exact version.

## Consequences

- Full DXF fidelity grows with the entity matrix; ImportReport lists skips.
- PDF text uses built-in PDF fonts until SHX/TTF embedding lands.
- Test corpus under `formats/corpus/dxf/` (synthetic now; expand to 50 real
  files as they are licensed for the repo).

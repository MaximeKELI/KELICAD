# ADR 0014 — Parity gap closure (modify / styles / XREF / DWG worker)

## Status

Accepted (2026-11-16)

## Context

Ultra-rigorous comparison vs AutoCAD / QCAD / LibreCAD / FreeCAD showed KELICAD strong on WASM/collab/AI but weak on drafting depth, styles, DWG, XREF, paper/plot, and parametric formulas.

## Decision

Close gaps with MIT-clean implementations (no GPL copy):

### P0
1. **Modify commands**: `ARRAYRECT`, `ARRAYPOLAR`, `STRETCH`, `BREAK`, `JOIN`, `EXPLODE`, `LENGTHEN`, `ALIGN` (+ geom helpers in `kelicad-geom::modify`).
2. **Styles**: `TextStyle` / `DimStyle` tables on `Document`; text/dim entities carry optional `style_id` + `annotative`.
3. **DWG worker**: `apps/dwg-worker` implements `KELICAD_DWG_WORKER` protocol with proxy fallback; refuse raw AC* DWG until ODA backend plugged in.

### P1
4. **Dimensions**: `DIMALIGNED`, `DIMRADIUS`, `DIMDIAMETER`, `DIMANGULAR`, `LEADER`.
5. **XREF / PDF underlay**: `XrefDef` + `XrefInsertGeom` + commands `XREF` / `PDFATTACH`.
6. **Layouts**: `PlotStyle`, viewport `annotation_scale`, paper helpers A3/Letter/landscape.

### P2
7. **Constraints**: `Angle`, `FormulaDistance` + expression evaluator + named params.
8. **Solids**: `boolean_intersection` + majority-vertex triangle filter.
9. **BIM**: Column / Beam / Roof / Space IFC types.

### P3
10. **UI**: expanded command palette aliases in web app.

## Consequences

- `Geometry` / `EntityKind` grew Leader, Ray, XLine, Xref — all IO/snap/wasm match sites updated.
- XREF definitions currently upsert outside the op journal (entity insert is journaled); follow-up: `UpsertXref` operation.
- Real Autodesk DWG still requires a licensed ODA (or equivalent) binary behind the same worker protocol.

## Non-goals (explicit)

- AutoCAD toolsets / Smart Blocks / AutoLISP
- Full BREP (OCCT) — mesh CSG remains the WASM-default kernel
- Complete CTB binary parser — named RGB/lineweight tables only

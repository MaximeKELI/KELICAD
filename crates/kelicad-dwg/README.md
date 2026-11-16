# kelicad-dwg

Pluggable `DwgBridge` for DWG import/export (ADR 0012). CI uses
`MockDwgBridge` (DXF proxy container). Production on-prem plugs an ODA worker
via `KELICAD_DWG_WORKER`.

```bash
cargo test -p kelicad-dwg
```

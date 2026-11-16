# KELICAD Desktop (Tauri 2)

Native shell around the web UI with:

- Open / Save `.kelicad` via OS dialogs (`kelicad-io` SQLite)
- Same React + WASM core as the browser app
- Split 2D / 3D WebGPU viewport

## Prerequisites

- Rust stable, Node.js 20+
- Linux: `webkit2gtk`, `libayatana-appindicator` (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

## Dev

```bash
# from repo root — build WASM once
cd apps/web && npm install && npm run build:wasm

# desktop
cd ../desktop && npm install && npm run dev
```

## Commands

| Command | Role |
|---------|------|
| `core_ping` | Smoke linkage to CAD crates |
| `open_kelicad` | Load native `.kelicad` → snapshot JSON |
| `save_kelicad` | Snapshot JSON → native `.kelicad` |

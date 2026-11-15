# kelicad-wasm

`wasm-bindgen` façade for the browser. The React app talks to the CAD core only
through this crate (or Tauri commands on desktop).

Build (optional for M0 web GPU demo):

```bash
rustup target add wasm32-unknown-unknown
cargo build -p kelicad-wasm --target wasm32-unknown-unknown
```

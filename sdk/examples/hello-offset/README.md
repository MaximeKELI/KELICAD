# hello-offset (third-party example plugin)

Manifest allows `doc_read` and writes only on layer `"0"`. The guest creates a
line from (0,0) to (1,0). Network and other layers are denied by the host.

Run via:

```bash
cargo test -p kelicad-plugin
kelicad-cli plugin run --manifest sdk/examples/hello-offset/plugin.json --script sdk/examples/hello-offset/script.json
```

Optional WASI build is out of CI path; see ADR 0009 (`NativeGuest` / `ScriptGuest`).

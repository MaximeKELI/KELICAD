# kelicad-gateway

In-memory collaboration rooms + WebSocket sync (M4).

```bash
cargo run -p kelicad-gateway
# listens on http://127.0.0.1:8787
# POST /rooms → { room_id, ws_url }
# WS /rooms/:id/ws
```

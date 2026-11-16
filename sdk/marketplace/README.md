# KELICAD plugin marketplace catalog

`catalog.json` lists signed plugins (ed25519). Demo signatures use
`DEMO_MARKET_SEED` from `kelicad-plugin` — replace for production.

```bash
cargo run -p kelicad-cli -- market verify --catalog sdk/marketplace/catalog.json
cargo run -p kelicad-cli -- market install hello-offset --catalog sdk/marketplace/catalog.json --dir /tmp/plugins
```

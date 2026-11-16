# On-premise deployment (M8)

Charter §19: air-gapped / on-site option. This folder is a **skeleton**:

1. Build release binaries on a networked build host:
   ```bash
   cargo build -p kelicad-gateway --release
   ```
2. Copy `target/release/kelicad-gateway` into the image context (or adjust Dockerfile).
3. Run:
   ```bash
   cd infra/onprem
   KELICAD_AUTH_MODE=dev docker compose up gateway
   ```
4. Optional stub DWG worker: `docker compose --profile oda-stub up`.

## Auth modes

| `KELICAD_AUTH_MODE` | Behavior |
|---------------------|----------|
| `off` (default) | No JWT; anonymous editor (100% local charter rule) |
| `dev` | `POST /v1/auth/dev-token` issues HS256 JWT |
| `oidc` | Same HS256 validation with configured issuer (plug real IdP JWKS later) |

## DWG

True DWG requires a licensed ODA worker pointed at by `KELICAD_DWG_WORKER`.
CI and demos use `MockDwgBridge` / proxy containers (ADR 0012).

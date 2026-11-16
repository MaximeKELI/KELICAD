# ADR 0012 — DWG bridge, enterprise auth, marketplace (M8)

- **Status:** Accepted
- **Date:** 2026-11-16
- **Deciders:** KELICAD engineering

## Context

M8 requires DWG via a licensed ODA library, SSO, audit, on-premise deployment,
and a plugin marketplace (charter §§2.7, 15–17, 19). Embedding Teigha/ODA in
default CI/WASM is not viable (license + native deps).

## Decision

1. **`DwgBridge` trait** in `kelicad-dwg`: convert DWG ↔ DXF/session. Default
   `MockDwgBridge` uses a documented `KELICAD_DWG_PROXY1` container around DXF
   for tests. `ExternalDwgBridge` shells out to `KELICAD_DWG_WORKER` for real ODA.
2. **Gateway auth:** JWT bearer with roles `viewer` | `editor` | `admin`.
   `KELICAD_AUTH_MODE=off|dev|oidc` — off preserves 100% local use; `dev` issues
   HS256 tokens; `oidc` validates via configured secret/issuer claims.
3. **`kelicad-audit`:** hash-chained append-only events (`prev_hash` + SHA-256).
4. **Marketplace:** signed catalog (ed25519 over entry content hash); CLI
   `market list|verify|install`. Plugin code signing remains capability-gated
   host (ADR 0009) plus catalog verification.
5. **On-prem:** `infra/onprem/docker-compose.yml` runs gateway (+ optional
   converter stub). GDPR: `/v1/me/export` and `/v1/me` delete by JWT `sub`.

## Consequences

- True Autodesk DWG binary decoding is never reverse-engineered in-tree.
- Local mode without accounts remains the default when auth mode is `off`.
- Full OIDC JWKS rotation / SAML are deferred beyond the M8 subset.

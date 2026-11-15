# ADR 0009 — Plugin sandbox and hostcall ABI

- **Status:** Accepted
- **Date:** 2026-11-15
- **Deciders:** KELICAD engineering

## Context

M5 requires third-party plugins that cannot crash the app or access undeclared
resources. The charter specifies WASM/WASI with a permission manifest.

## Decision

1. **Capability host:** every plugin side effect goes through JSON hostcalls
   checked against `Manifest.permissions` (fail closed).
2. **Guests:** `NativeGuest` (in-process, same ABI — CI and example plugin) and
   optional `wasmtime` WASI guests behind feature `wasm` for `.wasm` bytes.
3. **Limits:** max hostcalls per `invoke`; wasmtime fuel when enabled.
4. **No direct `Document` access** from guests — only `Operation` via the host.

## Consequences

- Undeclared layer writes and network hosts are denied with `PluginError::Denied`.
- Example plugin ships under `sdk/examples/`; CLI installs/runs via manifest.
- Signing / marketplace deferred.

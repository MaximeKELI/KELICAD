# KELICAD native format specification — v0

**Status:** Draft v0 (schema reserved; I/O implementation follows this spec)  
**File extension:** `.kelicad`  
**Container:** SQLite 3 database (single file)  
**Related ADR:** [0004-kelicad-sqlite-format.md](../docs/adr/0004-kelicad-sqlite-format.md)

## 1. Goals

- Open, inspectable, migratable CAD document format.
- Operation journal as durable source of truth; periodic snapshots for fast open.
- Same file for desktop and (via sql.js / OPFS) browser persistence.
- Schema version + forward migrations required for every breaking change.

## 2. Identification

| Key | Value |
|-----|-------|
| MIME (proposed) | `application/vnd.kelicad+sqlite3` |
| Magic | Standard SQLite header (`SQLite format 3\0`) |
| App id | `meta.format = kelicad` |
| Schema version | integer in `meta.schema_version` (v0 → `0`) |

## 3. Tables (v0)

### 3.1 `meta`

Key/value document metadata.

| Column | Type | Notes |
|--------|------|-------|
| `key` | TEXT PK | e.g. `format`, `schema_version`, `created_at`, `units`, `tolerance_abs` |
| `value` | TEXT | JSON or plain string |

Required keys for a valid v0 file:

- `format` = `kelicad`
- `schema_version` = `0`
- `doc_id` = UUID v7 string

### 3.2 `layers`

| Column | Type | Notes |
|--------|------|-------|
| `id` | BLOB(16) PK | UUID v7 |
| `name` | TEXT NOT NULL | |
| `color_r/g/b` | INTEGER | 0–255 |
| `lineweight` | INTEGER | hundredths of mm; 0 = default |
| `frozen` | INTEGER | 0/1 |
| `locked` | INTEGER | 0/1 |
| `no_plot` | INTEGER | 0/1 |

Default layer name `"0"` must exist.

### 3.3 `entities`

Materialized entity index (derived view; may be rebuilt from oplog).

| Column | Type | Notes |
|--------|------|-------|
| `id` | BLOB(16) PK | UUID v7 |
| `kind` | TEXT | `point`, `line`, `arc`, `circle`, … |
| `layer_id` | BLOB(16) | FK → layers |
| `geometry_json` | TEXT | Canonical JSON for the geometry component |
| `deleted` | INTEGER | 0/1 soft-delete marker for snapshots |

### 3.4 `oplog`

Immutable append-only journal.

| Column | Type | Notes |
|--------|------|-------|
| `seq` | INTEGER PK | Monotonic per document |
| `op_id` | BLOB(16) | UUID v7 of the operation |
| `ts` | INTEGER | Unix ms **injected** at write time (not used for CRDT ordering in-core) |
| `actor` | TEXT | Optional user/device id |
| `payload` | TEXT | JSON encoding of `Operation` |

### 3.5 `snapshots`

| Column | Type | Notes |
|--------|------|-------|
| `id` | INTEGER PK | |
| `seq_upto` | INTEGER | Oplog sequence covered |
| `created_at` | INTEGER | Unix ms |
| `blob` | BLOB | Compressed snapshot of entities/layers (format TBD: messagepack/zstd) |

### 3.6 Reserved for later (empty in v0 writers)

`blocks`, `styles`, `assets`, `comments` — tables may be created empty with
`schema_version = 0` so later migrations only `ALTER` / fill.

## 4. Units and precision

- Coordinates stored as IEEE-754 `f64` (JSON numbers or binary in snapshots).
- Document absolute tolerance in `meta.tolerance_abs` (default `1e-9`).
- Never rely on bitwise float equality when interpreting files.

## 5. Validity rules (v0)

1. `meta.format` and `meta.schema_version` present.
2. Layer `"0"` present.
3. Every entity `layer_id` references an existing layer.
4. `oplog.seq` contiguous from 1…N when a snapshot claims `seq_upto = N`.
5. Writers MUST commit schema changes and oplog appends in a single SQLite
   transaction.

## 6. Non-goals for v0

- CRDT merge metadata (M4).
- DWG embedding.
- Password encryption of the SQLite file (later: SQLCipher or container).

## 7. Change control

Any incompatible schema change increments `schema_version` and ships a
migration in `kelicad-io`. Spec patches that are additive may stay on the same
major draft letter until M1 freeze (`v0.1`).

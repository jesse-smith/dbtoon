# Quickstart: Column Type Metadata

**Feature**: 003-column-type-metadata
**Date**: 2026-02-12

## What This Feature Does

Adds SQL column type information (e.g., `INT`, `VARCHAR(255)`) to every query output. The output changes from a bare tabular array to a root object containing both `types` metadata and `rows` data.

## Files to Modify

| File | Change | Why |
|---|---|---|
| `src/backend/sqlserver.rs` | Add `normalize_odbc_type()` function; replace `format!("{:?}", ...)` with it | FR-005: Normalize SQL Server types from debug format |
| `src/format.rs` | Wrap output in root object with `types` + `rows` | FR-001, FR-002, FR-003: Include types in output |
| `tests/unit/format_test.rs` | Update all 4 format tests for new output structure | SC-005: Existing tests updated |

## Files NOT Modified

| File | Why |
|---|---|
| `src/backend/mod.rs` | `ColumnMeta` already has `type_name` field |
| `src/backend/databricks.rs` | Already produces standard SQL type names (FR-006) |
| `src/cli.rs` | No new CLI arguments |
| `src/config.rs` | No config changes |
| `src/output.rs` | Treats TOON as opaque string |
| `src/main.rs` | Pipeline unchanged |
| `Cargo.toml` | No new dependencies |

## Implementation Order

1. **Type normalization function** (`sqlserver.rs`) — pure function, independently testable
2. **Wire normalization** into SQL Server backend (`sqlserver.rs` line 126)
3. **Update `to_toon()`** to produce root object (`format.rs`)
4. **Update format tests** (`format_test.rs`)

## Verification

```bash
cargo test
cargo clippy
```

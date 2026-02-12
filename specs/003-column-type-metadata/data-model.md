# Data Model: Column Type Metadata

**Feature**: 003-column-type-metadata
**Date**: 2026-02-12

## Entities

### ColumnMeta (existing — no changes)

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Column identifier from result set |
| `type_name` | `String` | SQL type string (normalized for SQL Server, pass-through for Databricks) |

**Location**: `src/backend/mod.rs`

**Change**: The `type_name` field already exists. The change is in how it's populated (SQL Server normalization) and consumed (included in output).

### QueryResult (existing — no changes)

| Field | Type | Description |
|---|---|---|
| `columns` | `Vec<ColumnMeta>` | Column metadata including names and types |
| `rows` | `Vec<Vec<CellValue>>` | Row data as stringified cell values |
| `total_rows` | `Option<usize>` | Total available rows (if known) |
| `truncated` | `bool` | Whether row limit was applied |

**Location**: `src/backend/mod.rs`

**Change**: None. The struct already carries all information needed; the output layer just needs to serialize `type_name`.

### CellValue (existing — no changes)

| Variant | Type | Description |
|---|---|---|
| `Text` | `String` | Stringified cell value |
| `Null` | — | SQL NULL |

**Location**: `src/backend/mod.rs`

## Output Structure

### Before (current)

TOON output is a bare tabular array:

```json
[
  {"id": "1", "name": "Alice"},
  {"id": "2", "name": "Bob"}
]
```

### After (proposed)

TOON output is a root object with `types` and `rows`:

```json
{
  "types": ["INT", "VARCHAR(255)"],
  "rows": [
    {"id": "1", "name": "Alice"},
    {"id": "2", "name": "Bob"}
  ]
}
```

**Field ordering**: `types` before `rows` (FR-003), enforced by `serde_json::Map` insertion order.

**Zero-row case**:

```json
{
  "types": ["INT", "VARCHAR(255)"],
  "rows": []
}
```

## State Transitions

None. This feature adds metadata to output; no mutable state, no persistence, no lifecycle changes.

## Validation Rules

1. `types.len() == columns.len()` — enforced structurally (both derived from `QueryResult.columns`)
2. Each type string is non-empty — enforced by normalization function (fallback to `UNKNOWN`)
3. Output is valid TOON — enforced by `toon_format::encode_default()`

# Output Format Contract: Query Results with Type Metadata

**Feature**: 003-column-type-metadata
**Date**: 2026-02-12

## Contract

All successful query executions produce TOON-encoded output with the following JSON-equivalent structure:

```json
{
  "types": ["<type_1>", "<type_2>", ..., "<type_N>"],
  "rows": [
    {"<col_1>": "<val>", "<col_2>": "<val>", ...},
    ...
  ]
}
```

### Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `types` | `Array<String>` | Yes | SQL type names, one per column, positionally aligned with column headers |
| `rows` | `Array<Object>` | Yes | Tabular row data (may be empty) |

### Invariants

1. `types` field MUST appear before `rows` in serialized output
2. `types.length == number of columns in rows` (when rows is non-empty)
3. `types.length == number of column headers` (always)
4. Each type string is a standard SQL type name (e.g., `INT`, `VARCHAR(255)`, `DECIMAL(10,2)`)
5. Output MUST be valid TOON that round-trips through encode/decode

### Type String Format

| Pattern | Examples |
|---|---|
| Simple type | `INT`, `BIGINT`, `BIT`, `REAL`, `DATE` |
| Type with length | `VARCHAR(255)`, `NVARCHAR(100)`, `BINARY(16)` |
| Type with precision/scale | `DECIMAL(10,2)`, `NUMERIC(18,0)` |
| Type with precision | `FLOAT(53)`, `TIME(7)`, `DATETIME2(3)` |
| MAX types | `VARCHAR(MAX)`, `NVARCHAR(MAX)`, `VARBINARY(MAX)` |
| Unknown/fallback | `UNKNOWN` |

### Examples

**Standard result:**
```
TOON encoding of:
{
  "types": ["INT", "VARCHAR(255)", "VARCHAR(255)"],
  "rows": [
    {"id": "1", "name": "Alice", "email": "alice@co.com"},
    {"id": "2", "name": "Bob", "email": "bob@co.com"}
  ]
}
```

**Zero-row result:**
```
TOON encoding of:
{
  "types": ["INT", "VARCHAR(255)"],
  "rows": []
}
```

### Breaking Change

This is a **breaking change** to the output format. Previous output was a bare TOON array:
```
[2]{id,name,email}:
  1,Alice,alice@co.com
  2,Bob,bob@co.com
```

New output is a TOON object containing the array plus type metadata.

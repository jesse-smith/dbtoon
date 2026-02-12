# Feature: Add Column Types to Output Metadata

**GitHub Issue:** #5

## Description

Add SQL column type metadata to TOON query output. Both backends (SQL Server via ODBC, Databricks via REST API) already capture column types into `ColumnMeta.type_name` but this information is discarded during TOON formatting.

## Design Decision

Encode types as a **primitive inline array field** within a valid TOON root object, positionally aligned with the tabular column header. The entire output must remain valid, parseable TOON — no out-of-band prefixes or custom extensions.

### Current output (bare tabular array)

```
[2]{id,name,email}:
  1,Alice,alice@co.com
  2,Bob,bob@co.com
```

### Target output (root object with types metadata + tabular rows)

```
types[3]: INT,VARCHAR,VARCHAR
rows[2]{id,name,email}:
  1,Alice,alice@co.com
  2,Bob,bob@co.com
```

## What Already Exists

- `ColumnMeta` struct (`src/backend/mod.rs`) already has a `type_name: String` field populated by both backends.
- **SQL Server** (`src/backend/sqlserver.rs`): Uses `describe_col()` → `format!("{:?}", col_desc.data_type)`, producing Rust debug format (e.g. `Varchar { length: 255 }`). These should be normalized to standard SQL type strings (e.g. `VARCHAR(255)`).
- **Databricks** (`src/backend/databricks.rs`): Gets `type_name` directly from the REST API manifest (e.g. `STRING`, `INT`, `DECIMAL(10,2)`). Already clean.

## Scope

- **Format change** in `to_toon()` (`src/format.rs`): Build a JSON object `{ "types": [...], "rows": [...] }` and pass to `encode_default` instead of a bare JSON array.
- **SQL Server type normalization**: Map `odbc_api::DataType` variants to conventional SQL type strings instead of using debug format.
- **No backend trait changes** — `QueryResult` and `ColumnMeta` already carry the needed data.
- **Ensure `serde_json` field ordering** so `types` appears before `rows` in output (may require `preserve_order` feature on `serde_json`).
- **Update zero-row special case** in `to_toon()` to include types in the manual header.
- **Update existing format tests** to reflect the new root-object structure.

## Constraints

- Output must be valid TOON that round-trips through `toon_format::decode`.
- Values remain stringified (`CellValue::Text | Null`) — this feature is metadata-only, not runtime typing.

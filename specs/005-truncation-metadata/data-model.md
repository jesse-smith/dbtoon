# Data Model: Truncation Metadata

**Feature**: 005-truncation-metadata | **Date**: 2026-02-12

## Entities

### Existing: QueryResult (UNCHANGED)

```rust
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<CellValue>>,
    pub total_rows: Option<usize>,
    pub truncated: bool,               // Already carries truncation state
}
```

No changes to this structure. The `truncated` field already exists — this feature makes it visible in outputs.

### Existing: AppConfig (UNCHANGED)

```rust
pub struct AppConfig {
    // ...
    pub default_row_limit: Option<usize>,  // Used for message construction
    pub output_file: Option<PathBuf>,
}
```

No changes. The row limit is used only to construct the truncation message in `output_result()`.

### Existing: DbtoonError (UNCHANGED)

No new error variants. The `print_summary()` signature changes to return `Result<(), DbtoonError>` (encoding can fail), but uses the existing `Format { message }` variant.

## Data Flow

### Truncation Message Construction

The truncation message is constructed once in `output_result()` and passed to all format writers:

```text
QueryResult.truncated + QueryResult.rows.len()
    │
    ├─ truncated=true  → message = Some("Showing N rows. Use --no-limit to return all rows.")
    └─ truncated=false → message = None
```

### Output Routing with Truncation

```text
(QueryResult, truncated, message)
    │
    ├─ Stdout (no --output)
    │   └─ format::to_toon(result, truncated, message) → TOON with embedded keys → stdout
    │
    ├─ File: TOON
    │   ├─ format::to_toon(result, truncated, message) → TOON with embedded keys → file
    │   └─ output::print_summary(rows, path, truncated, message) → valid TOON → stdout
    │
    ├─ File: CSV
    │   ├─ format_csv::write_csv(result, path) → file (no metadata in CSV)
    │   └─ output::print_summary(rows, path, truncated, message) → valid TOON → stdout
    │
    ├─ File: Parquet
    │   ├─ format_parquet::write_parquet(result, path, truncated, message) → file with schema metadata
    │   └─ output::print_summary(rows, path, truncated, message) → valid TOON → stdout
    │
    └─ File: Arrow IPC
        ├─ format_arrow::write_arrow(result, path, truncated, message) → file with schema metadata
        └─ output::print_summary(rows, path, truncated, message) → valid TOON → stdout

    (all paths, when truncated)
    └─ output::print_truncation_warning(message) → stderr
```

## Metadata Shapes

### TOON Root Object (stdout and .toon file)

When truncated:
```json
{
  "types": ["INT", "VARCHAR(255)"],
  "rows": [{"id": "1", "name": "Alice"}],
  "truncated": true,
  "message": "Showing 500 rows. Use --no-limit to return all rows."
}
```

When not truncated:
```json
{
  "types": ["INT", "VARCHAR(255)"],
  "rows": [{"id": "1", "name": "Alice"}],
  "truncated": false
}
```

### Parquet File Metadata (key-value in file footer)

When truncated:
| Key | Value |
|-----|-------|
| `dbtoon:truncated` | `"true"` |
| `dbtoon:message` | `"Showing 500 rows. Use --no-limit to return all rows."` |

When not truncated: no `dbtoon:` keys present.

### Arrow IPC Schema Metadata (key-value in schema message)

Same keys and values as Parquet (stored in Arrow `Schema.metadata` HashMap).

### Print Summary (stdout for all file outputs)

When truncated:
```json
{
  "rows_written": 500,
  "file": "/path/to/output.csv",
  "truncated": true,
  "message": "Showing 500 rows. Use --no-limit to return all rows."
}
```

When not truncated:
```json
{
  "rows_written": 500,
  "file": "/path/to/output.csv",
  "truncated": false
}
```

(Shown as JSON for clarity — actual output is TOON-encoded via `toon_format::encode_default()`.)

### Stderr Warning

When truncated (all output formats):
```
warning: Showing 500 rows. Use --no-limit to return all rows.
```

When not truncated: nothing emitted to stderr.

## Validation Rules

- `"truncated"` key is ALWAYS present in TOON output (both `true` and `false`) — prevents ambiguity with older tool versions
- `"message"` key is ONLY present when `truncated` is `true` — keeps non-truncated output clean
- Parquet/Arrow metadata keys use `dbtoon:` namespace prefix — prevents collisions with other tools
- Parquet/Arrow metadata is ONLY present when truncated — non-truncated files have no `dbtoon:` keys
- Print summary is always valid TOON — parseable by any TOON-compatible consumer
- Stderr warning uses `"warning: "` prefix — consistent with existing `"error: "` prefix pattern

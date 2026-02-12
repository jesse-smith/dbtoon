# Output Format Contract

**Feature**: 004-output-formats | **Date**: 2026-02-12

## Format Detection API

### `format_detect::detect_format`

```rust
/// Detect the output format from a file path extension.
/// Returns the format and the (possibly normalized) path.
///
/// - `.toon`, `.txt` → Toon
/// - `.csv` → Csv
/// - `.parquet` → Parquet
/// - `.arrow` → Arrow
/// - No extension → appends `.toon`, returns Toon
/// - Unrecognized → error with supported format list
pub fn detect_format(path: &Path) -> Result<(OutputFormat, PathBuf), DbtoonError>
```

**Inputs**: `path: &Path` — the user-specified `--output` path
**Outputs**: `(OutputFormat, PathBuf)` — the detected format and the canonical path (with `.toon` appended if no extension)
**Errors**: `DbtoonError::Format` with message listing supported extensions

### `OutputFormat` enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Toon,
    Csv,
    Parquet,
    Arrow,
}
```

## Writer APIs

All writer functions accept a `&QueryResult` and a `&Path`, and write directly to the file system.

### CSV Writer

```rust
/// Write query results as RFC 4180 CSV.
/// - Header row from column names
/// - NULL values → empty fields
/// - Values with commas/quotes/newlines are escaped
/// - CRLF line endings (RFC 4180)
pub fn write_csv(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>
```

### Parquet Writer

```rust
/// Write query results as a Parquet file with typed columns.
/// - Column types derived from ColumnMeta.type_name via sql_type_to_arrow()
/// - Unmappable types fall back to string (Utf8)
/// - Values that fail to parse as target type cause column-level fallback to string
/// - Uses Snappy compression
/// - NULL values → native Parquet nulls
pub fn write_parquet(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>
```

### Arrow IPC Writer

```rust
/// Write query results as an Arrow IPC file with typed columns.
/// - Same type mapping as Parquet (shared via format_columnar)
/// - NULL values → native Arrow nulls
pub fn write_arrow(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>
```

### Shared: Columnar Utilities

```rust
/// Map a SQL type string to an Arrow DataType.
/// Unknown types map to Utf8 (string fallback).
pub fn sql_type_to_arrow(type_name: &str) -> DataType

/// Build an Arrow RecordBatch from a QueryResult.
/// Each column is converted to a typed Arrow array based on its type_name.
/// If value parsing fails for a column, that column falls back to Utf8.
pub fn build_record_batch(result: &QueryResult) -> Result<(Arc<Schema>, RecordBatch), DbtoonError>
```

## Dispatch Contract (main.rs)

### Fail-Fast Detection (exec_read / exec_write)

Format detection happens **before** `execute_query()` so that unrecognized extensions fail fast without executing a query (spec US1 scenario 6):

```rust
// In exec_read() and exec_write(), before execute_query():
let format_info = if let Some(ref path) = app_config.output_file {
    Some(format_detect::detect_format(path)?)  // fails fast on bad extension
} else {
    None
};

let result = execute_query(&app_config, &sql, verbose).await?;
output_result(&app_config, &result, format_info)?;
```

### Format-Aware output_result()

The `output_result()` signature changes from:

```rust
// BEFORE
fn output_result(app_config: &AppConfig, result: &QueryResult) -> Result<(), DbtoonError>
```

To:

```rust
// AFTER
fn output_result(
    app_config: &AppConfig,
    result: &QueryResult,
    format_info: Option<(OutputFormat, PathBuf)>,
) -> Result<(), DbtoonError>
```

Dispatch logic:

```rust
if let Some((format, path)) = format_info {
    match format {
        OutputFormat::Toon => {
            let toon = format::to_toon(result)?;
            output::write_file(&toon, &path)?;
        }
        OutputFormat::Csv => format_csv::write_csv(result, &path)?,
        OutputFormat::Parquet => format_parquet::write_parquet(result, &path)?,
        OutputFormat::Arrow => format_arrow::write_arrow(result, &path)?,
    }
    output::print_summary(result.rows.len(), &path, result.truncated);
} else {
    let toon = format::to_toon(result)?;
    output::print_result(&toon);
    // truncation warning handled as before
}
```

## Error Contract

No new `DbtoonError` variants. All format-specific errors map to:

```rust
DbtoonError::Format { message: String }
```

Error messages follow existing patterns:
- Unsupported extension: `format: unsupported output format ".xlsx" — supported: .toon, .txt, .csv, .parquet, .arrow`
- Write failure: `format: failed to write Parquet file: {underlying_error}`
- Type mapping failure: silent fallback to string (FR-009), no user-visible error

## Behavioral Guarantees

1. **Stdout is always TOON** — format dispatch only applies when `--output` is specified
2. **No new CLI flags** — format is determined entirely by file extension
3. **Case-insensitive** — `.CSV`, `.Csv`, `.csv` all produce CSV
4. **Overwrite without prompting** — matches existing TOON file behavior
5. **Empty results** — all formats produce valid files with schema/headers and zero data rows
6. **Summary line unchanged** — `rows_written`, `file`, `truncated` printed to stdout regardless of format

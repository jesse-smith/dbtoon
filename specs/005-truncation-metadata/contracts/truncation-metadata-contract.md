# Truncation Metadata Contract

**Feature**: 005-truncation-metadata | **Date**: 2026-02-12

## Modified APIs

### `format::to_toon` (MODIFIED)

```rust
/// Convert a QueryResult to a TOON-formatted string with truncation metadata.
///
/// Output is a root object: `{ "types": [...], "rows": [...], "truncated": bool, "message"?: str }`.
/// The `"truncated"` key is always present. The `"message"` key is present only when truncated.
pub fn to_toon(
    result: &QueryResult,
    truncated: bool,
    message: Option<&str>,
) -> Result<String, DbtoonError>
```

**Change**: Added `truncated` and `message` parameters. Previously accepted only `&QueryResult`.

**Inputs**:
- `result` — query data (columns + rows)
- `truncated` — whether the result was truncated
- `message` — human-readable truncation description (only `Some` when truncated)

**Outputs**: TOON string with `truncated` and optionally `message` in the root object.

### `output::print_summary` (MODIFIED)

```rust
/// Print file output summary to stdout as a valid TOON object.
///
/// Summary includes: rows_written (number), file (string), truncated (bool),
/// and message (string, only when truncated).
pub fn print_summary(
    rows: usize,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError>
```

**Change**: Added `message` parameter. Return type changed from `()` to `Result<(), DbtoonError>` (TOON encoding can fail). Output changed from ad-hoc `key: value` text to valid TOON via `toon_format::encode_default()`.

### `output::print_truncation_warning` (NEW)

```rust
/// Print a truncation warning to stderr for interactive visibility.
/// Format: "warning: {message}"
pub fn print_truncation_warning(message: &str)
```

**Inputs**: The truncation message string (same text as the in-band `"message"` value).

### `output::print_truncation_message` (REMOVED)

Previously appended non-TOON truncation text to stdout. Replaced by:
1. In-band `"truncated"` and `"message"` keys in the TOON output (FR-001, FR-002)
2. Stderr warning via `print_truncation_warning()` (FR-012)

### `format::to_toon_kv` (REMOVED)

Previously formatted key-value pairs as `key: value\nkey: value`. All callers replaced:
- `print_summary()` now uses `toon_format::encode_default()`
- `print_truncation_message()` removed entirely

### `format_parquet::write_parquet` (MODIFIED)

```rust
/// Write query results as a Parquet file with typed columns and optional truncation metadata.
///
/// When truncated, file metadata includes `dbtoon:truncated` and `dbtoon:message` keys.
pub fn write_parquet(
    result: &QueryResult,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError>
```

**Change**: Added `truncated` and `message` parameters. When truncated, adds metadata to the Arrow schema before creating the Parquet writer.

### `format_arrow::write_arrow` (MODIFIED)

```rust
/// Write query results as an Arrow IPC file with typed columns and optional truncation metadata.
///
/// When truncated, schema metadata includes `dbtoon:truncated` and `dbtoon:message` keys.
pub fn write_arrow(
    result: &QueryResult,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError>
```

**Change**: Added `truncated` and `message` parameters. Same metadata mechanism as Parquet (shared via `format_columnar::with_truncation_metadata()`).

### `format_columnar::with_truncation_metadata` (NEW)

```rust
/// Add truncation metadata to an Arrow schema.
///
/// When truncated, adds `dbtoon:truncated` = "true" and `dbtoon:message` = message
/// to the schema's key-value metadata. When not truncated, returns the schema unchanged.
pub fn with_truncation_metadata(
    schema: Arc<Schema>,
    truncated: bool,
    message: Option<&str>,
) -> Arc<Schema>
```

**Purpose**: Shared helper for Parquet and Arrow writers to avoid duplicating metadata key names.

## Dispatch Contract (main.rs)

### Modified `output_result()`

```rust
fn output_result(
    app_config: &AppConfig,
    result: &backend::QueryResult,
    format_info: Option<(OutputFormat, std::path::PathBuf)>,
) -> Result<(), DbtoonError> {
    // 1. Build truncation message (once)
    let message = if result.truncated {
        Some(format!(
            "Showing {} rows. Use --no-limit to return all rows.",
            result.rows.len()
        ))
    } else {
        None
    };

    // 2. Write output (format-specific)
    if let Some((format, path)) = format_info {
        match format {
            OutputFormat::Toon => {
                let toon = format::to_toon(result, result.truncated, message.as_deref())?;
                output::write_file(&toon, &path)?;
            }
            OutputFormat::Csv => {
                dbtoon::format_csv::write_csv(result, &path)?;
            }
            OutputFormat::Parquet => {
                dbtoon::format_parquet::write_parquet(
                    result, &path, result.truncated, message.as_deref(),
                )?;
            }
            OutputFormat::Arrow => {
                dbtoon::format_arrow::write_arrow(
                    result, &path, result.truncated, message.as_deref(),
                )?;
            }
        }
        output::print_summary(
            result.rows.len(), &path, result.truncated, message.as_deref(),
        )?;
    } else {
        // Stdout: TOON with embedded truncation metadata
        let toon = format::to_toon(result, result.truncated, message.as_deref())?;
        output::print_result(&toon);
    }

    // 3. Stderr warning (all formats, when truncated)
    if let Some(ref msg) = message {
        output::print_truncation_warning(msg);
    }

    Ok(())
}
```

## Behavioral Guarantees

1. **Stdout is always valid TOON** — no non-data text on stdout in any output mode (SC-004)
2. **`truncated` key always present in TOON** — both `true` and `false` values (avoids version ambiguity)
3. **`message` key conditional** — only present when `truncated` is `true`
4. **Parquet/Arrow metadata conditional** — `dbtoon:` keys only present when truncated
5. **Stderr warning always emitted when truncated** — regardless of output format or destination (FR-012)
6. **Print summary is valid TOON** — parseable by any TOON decoder (FR-007)
7. **No new CLI flags** — behavior is automatic based on truncation state
8. **CSV files unchanged** — CSV has no native metadata mechanism; truncation info conveyed via print summary only

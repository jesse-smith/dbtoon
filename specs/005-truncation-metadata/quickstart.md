# Quickstart: Truncation Metadata

**Feature**: 005-truncation-metadata | **Date**: 2026-02-12

## Prerequisites

- Rust 1.85+ (project uses 1.91.1)
- Existing dbtoon build working (`cargo build`)

## No New Dependencies

This feature modifies existing code only. No `Cargo.toml` changes required.

## Behavior Changes

### TOON stdout output (default)

```bash
# Truncated result — TOON object includes truncated + message
dbtoon exec-read "SELECT * FROM large_table"
# stdout:
# types[2]{...}:
#   ...
# rows[500]{...}:
#   ...
# truncated: true
# message: Showing 500 rows. Use --no-limit to return all rows.
#
# stderr: warning: Showing 500 rows. Use --no-limit to return all rows.

# Non-truncated result — truncated key present, no message
dbtoon exec-read "SELECT * FROM small_table"
# stdout:
# types[2]{...}:
#   ...
# rows[3]{...}:
#   ...
# truncated: false
```

### TOON file output

```bash
dbtoon exec-read "SELECT * FROM large_table" -o results.toon
# File contains: types, rows, truncated: true, message: "Showing 500 rows..."
# stdout (summary): rows_written: 500, file: results.toon, truncated: true, message: "..."
# stderr: warning: Showing 500 rows. Use --no-limit to return all rows.
```

### CSV file output

```bash
dbtoon exec-read "SELECT * FROM large_table" -o results.csv
# File contains: CSV data only (no metadata in CSV format)
# stdout (summary): valid TOON object with rows_written, file, truncated, message
# stderr: warning: Showing 500 rows. Use --no-limit to return all rows.
```

### Parquet file output

```bash
dbtoon exec-read "SELECT * FROM large_table" -o results.parquet
# File contains: Parquet data + file metadata: dbtoon:truncated=true, dbtoon:message=...
# stdout (summary): valid TOON object with rows_written, file, truncated, message
# stderr: warning: Showing 500 rows. Use --no-limit to return all rows.

# Verify with pyarrow:
python3 -c "
import pyarrow.parquet as pq
meta = pq.read_metadata('results.parquet').metadata
print(meta[b'dbtoon:truncated'])  # b'true'
print(meta[b'dbtoon:message'])    # b'Showing 500 rows...'
"
```

### Arrow IPC file output

```bash
dbtoon exec-read "SELECT * FROM large_table" -o results.arrow
# File contains: Arrow IPC data + schema metadata: dbtoon:truncated=true, dbtoon:message=...
# stdout (summary): valid TOON object with rows_written, file, truncated, message
# stderr: warning: Showing 500 rows. Use --no-limit to return all rows.

# Verify with pyarrow:
python3 -c "
import pyarrow.ipc as ipc
reader = ipc.open_file('results.arrow')
meta = reader.schema.metadata
print(meta[b'dbtoon:truncated'])  # b'true'
print(meta[b'dbtoon:message'])    # b'Showing 500 rows...'
"
```

## Development Workflow

```bash
# Run all tests
cargo test

# Run only truncation/format-related tests
cargo test truncat
cargo test format
cargo test output

# Check for warnings
cargo clippy
```

## Module Map (modified files)

| File | Change |
|------|--------|
| `src/format.rs` | `to_toon()` adds truncation keys; `to_toon_kv()` removed |
| `src/output.rs` | `print_summary()` produces valid TOON; `print_truncation_message()` removed; `print_truncation_warning()` added |
| `src/format_parquet.rs` | `write_parquet()` accepts truncation info, adds schema metadata |
| `src/format_arrow.rs` | `write_arrow()` accepts truncation info, adds schema metadata |
| `src/format_columnar.rs` | `with_truncation_metadata()` helper added |
| `src/main.rs` | `output_result()` builds message, passes truncation info, emits stderr warning |

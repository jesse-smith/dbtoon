# Quickstart: Multiple Output File Formats

**Feature**: 004-output-formats | **Date**: 2026-02-12

## Prerequisites

- Rust 1.85+ (project uses 1.91.1)
- Existing dbtoon build working (`cargo build`)

## New Dependencies

Add to `Cargo.toml`:

```toml
csv = "1.4"
arrow = { version = "57", default-features = false, features = ["ipc"] }
parquet = { version = "57", default-features = false, features = ["arrow"] }
```

## Usage

### CSV output

```bash
# Write query results as CSV
dbtoon exec-read "SELECT * FROM users" -o results.csv

# Case-insensitive extension
dbtoon exec-read "SELECT * FROM users" -o results.CSV
```

### Parquet output

```bash
# Write query results as Parquet (typed columns)
dbtoon exec-read "SELECT id, name, salary FROM employees" -o results.parquet
```

### Arrow IPC output

```bash
# Write query results as Arrow IPC
dbtoon exec-read "SELECT * FROM metrics" -o results.arrow
```

### TOON output (existing, unchanged)

```bash
# Stdout (default)
dbtoon exec-read "SELECT * FROM users"

# File with .toon extension
dbtoon exec-read "SELECT * FROM users" -o results.toon

# File with .txt extension
dbtoon exec-read "SELECT * FROM users" -o results.txt

# No extension → auto-appends .toon
dbtoon exec-read "SELECT * FROM users" -o results
# → writes results.toon
```

### Error on unrecognized extension

```bash
dbtoon exec-read "SELECT * FROM users" -o results.xlsx
# error: format: unsupported output format ".xlsx" — supported: .toon, .txt, .csv, .parquet, .arrow
```

## Development Workflow

```bash
# Run all tests
cargo test

# Run only format-related tests
cargo test format

# Check for warnings
cargo clippy
```

## Module Map

| File | Purpose |
|------|---------|
| `src/format_detect.rs` | Extension → `OutputFormat` enum |
| `src/format_csv.rs` | CSV writer (RFC 4180) |
| `src/format_columnar.rs` | SQL type → Arrow type mapping + array building |
| `src/format_parquet.rs` | Parquet file writer |
| `src/format_arrow.rs` | Arrow IPC file writer |

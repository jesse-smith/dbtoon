# Research: Multiple Output File Formats

**Feature**: 004-output-formats | **Date**: 2026-02-12

## R1: CSV Library Selection

**Decision**: Use the `csv` crate v1.4

**Rationale**: The `csv` crate is the de facto standard for CSV in Rust (~149M downloads). It handles RFC 4180 quoting and escaping automatically. The only gotcha is that the default line terminator is `\n` (LF), not `\r\n` (CRLF) as RFC 4180 requires — must set `WriterBuilder::terminator(Terminator::CRLF)`.

**Alternatives considered**:
- Manual `std::fs::write` with string formatting — rejected because escaping/quoting edge cases (commas, quotes, newlines in values) are error-prone to implement manually.
- `polars` CSV writer — rejected because it would pull in the entire Polars dependency for a simple write operation.

**LOC estimate**: ~20-25 lines for the writer function.

## R2: Parquet/Arrow Library Selection

**Decision**: Use `arrow` v57 + `parquet` v57 from the Apache arrow-rs project.

**Rationale**: The arrow-rs monorepo is the only mature Parquet/Arrow implementation in Rust. Both crates are at v57.3.0 (Feb 2026), require Rust 1.85+ (we have 1.91.1), and use edition 2024 (matching our project). The `parquet` crate's high-level API (`ArrowWriter`) accepts Arrow `RecordBatch` objects, so both formats share the same array-building code.

**Alternatives considered**:
- `parquet2` (standalone) — rejected because it's less maintained than arrow-rs and doesn't share code with Arrow IPC.
- Building Parquet manually from spec — rejected (enormous effort, no benefit).

**Cargo.toml additions**:
```toml
arrow = { version = "57", default-features = false, features = ["ipc"] }
parquet = { version = "57", default-features = false, features = ["arrow"] }
```

## R3: Feasibility Gate — Type Mapping LOC

**Decision**: Parquet and Arrow IPC are INCLUDED (feasibility gate passes).

**Rationale**: The type-mapping glue code consists of two parts:

1. **SQL type string → Arrow DataType** (~45-55 LOC): Pattern-match on the normalized `type_name` strings produced by `normalize_odbc_type()` (SQL Server) and Databricks passthrough. Maps to Arrow `DataType` variants. Unmappable types fall back to `Utf8` (FR-009).

2. **CellValue::Text → typed Arrow arrays** (~60-75 LOC): For each column, parse string values into the target Arrow type using per-type builder logic. Parse failures for a column cause that column to fall back to `Utf8` (FR-009).

**Total glue code**: ~105-130 LOC. **Below the 200 LOC threshold** from FR-006.

**Key type mappings**:

| SQL Type (from `type_name`) | Arrow DataType | Parse strategy |
|-|-|-|
| `INT`, `INTEGER` | `Int32` | `str::parse::<i32>()` |
| `SMALLINT` | `Int16` | `str::parse::<i16>()` |
| `BIGINT` | `Int64` | `str::parse::<i64>()` |
| `TINYINT` | `UInt8` | `str::parse::<u8>()` (SQL Server TINYINT is unsigned 0-255) |
| `BIT`, `BOOLEAN` | `Boolean` | `"0"/"1"/"true"/"false"` |
| `REAL` | `Float32` | `str::parse::<f32>()` |
| `FLOAT`, `FLOAT(n)` | `Float64` | `str::parse::<f64>()` |
| `VARCHAR(n)`, `NVARCHAR(n)`, `CHAR(n)`, `NCHAR(n)`, `STRING` | `Utf8` | No parse needed |
| `VARCHAR(MAX)`, `NVARCHAR(MAX)` | `Utf8` | No parse needed |
| `DECIMAL(p,s)`, `NUMERIC(p,s)` | `Decimal128(p, s)` | Parse string to i128 scaled value |
| `DATE` | `Date32` | Parse `YYYY-MM-DD` to days-since-epoch |
| `DATETIME2(p)`, `TIMESTAMP` | `Timestamp(Microsecond, None)` | Parse ISO 8601 to microseconds |
| `TIME(p)` | `Time64(Microsecond)` | Parse `HH:MM:SS.fff` to microseconds |
| `BINARY(n)`, `VARBINARY(n)`, `VARBINARY(MAX)` | `Binary` | Hex decode |
| `UNKNOWN`, anything else | `Utf8` | String fallback (FR-009) |

**Risk**: Value parsing from `CellValue::Text` is inherently best-effort since values are already stringified. The string fallback (FR-009) mitigates this — if any value in a column fails to parse as its target type, that entire column falls back to `Utf8`. This is conservative but safe.

## R4: Format Detection Strategy

**Decision**: Detect format from file extension using `Path::extension()`, case-insensitive comparison.

**Rationale**: The spec explicitly states no `--format` flag — format is determined solely by the `--output` path extension. This is the simplest approach and matches user expectations (`.csv` = CSV file).

**Extension mapping**:
- `.toon`, `.txt` → TOON (existing behavior)
- `.csv` → CSV
- `.parquet` → Parquet
- `.arrow` → Arrow IPC
- No extension → append `.toon`, write TOON
- Unrecognized → error with supported format list

**Implementation**: A small `OutputFormat` enum and a `detect_format(path: &Path) -> Result<(OutputFormat, PathBuf), DbtoonError>` function that returns both the detected format and the (possibly modified) path.

## R5: toon-format Compatibility

**Decision**: No changes to TOON output path.

**Rationale**: The existing `format::to_toon()` function remains the sole TOON producer. The new format dispatch in `output_result()` calls `to_toon()` for TOON format and the new writer functions for other formats. No modifications to `format.rs` or `toon-format` usage.

## R6: Error Handling for Arrow/Parquet

**Decision**: Wrap `arrow::error::ArrowError` and `parquet::errors::ParquetError` into `DbtoonError::Format`.

**Rationale**: Both arrow-rs error types implement `std::error::Error` and can be converted to strings. Rather than adding dedicated error variants (which would leak implementation details), map them to the existing `DbtoonError::Format { message }` variant. This keeps the error enum stable and simple.

**Alternatives considered**:
- Adding `#[from] ArrowError` and `#[from] ParquetError` variants — rejected because it exposes internal dependencies in the public error API.
- Adding a generic `#[from] Box<dyn std::error::Error>` — rejected because it loses type information.

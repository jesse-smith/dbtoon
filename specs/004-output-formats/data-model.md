# Data Model: Multiple Output File Formats

**Feature**: 004-output-formats | **Date**: 2026-02-12

## Entities

### OutputFormat (NEW)

An enum representing the supported output serialization formats.

```rust
pub enum OutputFormat {
    Toon,
    Csv,
    Parquet,
    Arrow,
}
```

**States**: This is a value enum with no state transitions. It is determined once from the file extension and used to dispatch to the appropriate writer.

**Relationships**: Used by `output_result()` to select the writing path. Does not interact with `QueryResult` directly — the writer functions accept `&QueryResult` and produce file output.

### Existing: QueryResult (UNCHANGED)

```rust
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,      // Column names + type metadata
    pub rows: Vec<Vec<CellValue>>,     // Row data (Text or Null)
    pub total_rows: Option<usize>,
    pub truncated: bool,
}

pub struct ColumnMeta {
    pub name: String,
    pub type_name: String,             // Normalized SQL type string
}

pub enum CellValue {
    Text(String),
    Null,
}
```

No changes to these structures. All format writers read from `QueryResult` as-is.

### Existing: DbtoonError (MODIFIED)

No new variants added. Arrow and Parquet errors are mapped into the existing `Format { message: String }` variant via `.map_err()`.

### Existing: AppConfig (UNCHANGED)

```rust
pub struct AppConfig {
    // ...
    pub output_file: Option<PathBuf>,  // Already captures --output path
}
```

No changes. The `output_file` field already holds the user-specified path.

## Data Flow

```text
QueryResult
    │
    ├─ [detect_format(path)] → OutputFormat + normalized_path
    │
    ├─ OutputFormat::Toon    → format::to_toon() → String → write_file()
    ├─ OutputFormat::Csv     → format_csv::write_csv() → file
    ├─ OutputFormat::Parquet → format_columnar::build_record_batch()
    │                          → format_parquet::write_parquet() → file
    └─ OutputFormat::Arrow   → format_columnar::build_record_batch()
                               → format_arrow::write_arrow() → file
```

## Type Mapping Model

The `format_columnar` module defines the mapping from SQL type strings to Arrow DataTypes. This is a pure function with no state:

```text
type_name: &str  →  sql_type_to_arrow()  →  arrow::datatypes::DataType
```

Value conversion is column-oriented:

```text
(Vec<CellValue>, DataType)  →  build_array()  →  Arc<dyn arrow::array::Array>
```

If value parsing fails for any cell in a column, the entire column falls back to `Utf8` (string) type. This is a column-level decision, not a cell-level one, because Parquet and Arrow require uniform column types.

## Validation Rules

- File extension detection is case-insensitive (`.CSV` == `.csv`)
- Unrecognized extensions produce `DbtoonError::Format` with supported format list
- No-extension paths get `.toon` appended (path mutation occurs before writing)
- Parent directory must exist (existing check in `output::write_file()`)
- Empty result sets produce valid files with schema/headers but no data rows

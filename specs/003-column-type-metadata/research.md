# Research: Column Type Metadata

**Feature**: 003-column-type-metadata
**Date**: 2026-02-12

## R1: TOON Format Object Support

**Question**: Does toon-format 0.4 support a root object containing both a `types` array and a `rows` tabular array?

**Decision**: Yes — use `toon_format::encode_default()` with a `serde_json::Value::Object` containing `types` (array of strings) and `rows` (array of objects).

**Rationale**: The toon-format crate encodes any valid JSON value. Root objects with mixed child types (primitive arrays, tabular arrays) are part of the TOON specification. `serde_json::Map` preserves insertion order, ensuring `types` appears before `rows` (FR-003).

**Alternatives considered**:
- Custom TOON header format (e.g., embedding types in the `[N]{...}` header line) — rejected because it would require format-level changes and the crate already handles root objects correctly.
- Separate metadata line before the TOON body — rejected because it breaks TOON compliance (FR-004).

## R2: SQL Server ODBC Type Normalization

**Question**: How should SQL Server ODBC `DataType` enum values be normalized to standard SQL type strings?

**Decision**: Pattern-match directly on `odbc_api::DataType` enum variants instead of parsing the debug-format string. Build a dedicated `normalize_odbc_type()` function.

**Rationale**: The `DataType` enum is a stable, well-typed Rust enum. Matching on it is exhaustive (compiler-enforced), avoids string parsing fragility, and gives direct access to parameters (length, precision, scale). The debug format is an implementation detail that could change between crate versions.

**Alternatives considered**:
- Parse the `format!("{:?}", ...)` debug string with regex — rejected because it's fragile, not compiler-checked, and less performant than direct enum matching.
- Use `Display` impl if available — the enum has no `Display` impl, only `Debug`.

### Complete Type Mapping

| `DataType` Variant | SQL Standard Output | Notes |
|---|---|---|
| `Unknown` | `UNKNOWN` | Fallback for unmappable types |
| `Char { length: Some(n) }` | `CHAR(n)` | |
| `Char { length: None }` | `CHAR` | Rare; no length reported |
| `WChar { length: Some(n) }` | `NCHAR(n)` | Unicode fixed-length |
| `WChar { length: None }` | `NCHAR` | |
| `Varchar { length: Some(n) }` | `VARCHAR(n)` | |
| `Varchar { length: None }` | `VARCHAR(MAX)` | |
| `WVarchar { length: Some(n) }` | `NVARCHAR(n)` | |
| `WVarchar { length: None }` | `NVARCHAR(MAX)` | |
| `LongVarchar { .. }` | `VARCHAR(MAX)` | Legacy TEXT mapping |
| `WLongVarchar { .. }` | `NVARCHAR(MAX)` | Legacy NTEXT mapping |
| `Integer` | `INT` | |
| `SmallInt` | `SMALLINT` | |
| `BigInt` | `BIGINT` | |
| `TinyInt` | `TINYINT` | |
| `Float { precision }` | `FLOAT(p)` | Precision in bits |
| `Real` | `REAL` | |
| `Double` | `FLOAT` | SQL Server `FLOAT` = 53-bit |
| `Numeric { precision, scale }` | `NUMERIC(p,s)` | |
| `Decimal { precision, scale }` | `DECIMAL(p,s)` | |
| `Date` | `DATE` | |
| `Time { precision }` | `TIME(p)` | Fractional seconds precision |
| `Timestamp { precision }` | `DATETIME2(p)` | SQL Server mapping |
| `Bit` | `BIT` | |
| `Binary { length: Some(n) }` | `BINARY(n)` | |
| `Binary { length: None }` | `BINARY` | |
| `Varbinary { length: Some(n) }` | `VARBINARY(n)` | |
| `Varbinary { length: None }` | `VARBINARY(MAX)` | |
| `LongVarbinary { .. }` | `VARBINARY(MAX)` | Legacy IMAGE mapping |
| `Other { .. }` | `UNKNOWN` | Fallback for driver-specific types |

## R3: Output Structure Change Impact

**Question**: What is the impact of changing from a bare array to a root object in the TOON output?

**Decision**: The change is localized to `format::to_toon()`. No downstream consumers exist outside this project. All format tests must be updated to expect the new structure.

**Rationale**: The function is the sole serialization point. The output module (`output.rs`) treats TOON as an opaque string. The CLI consumer (stdout/file) is format-agnostic.

**Impact inventory**:
- `src/format.rs` — `to_toon()`: change from bare array to root object
- `tests/unit/format_test.rs` — all 4 format tests: update expected structure
- No CLI, config, or backend changes needed for the output structure itself

## R4: Zero-Row Result Handling

**Question**: How should zero-row results include type metadata in the new output structure?

**Decision**: Use the same root object structure for zero-row results. Build the JSON object with `types` array and empty `rows` array, then encode with `toon_format::encode_default()`. Remove the current special-case manual TOON header.

**Rationale**: The current special case (`format!("[0]{{{}}}:\n", col_names)`) exists because toon-format can't infer columns from an empty array. With a root object structure, we can encode `types` normally and let `rows` be an empty array — `toon_format` handles empty arrays fine as object values. This eliminates the special case and ensures consistent handling.

**Alternatives considered**:
- Keep the special case and extend it with types — rejected because it would mean two different code paths for the same output structure, violating DRY.

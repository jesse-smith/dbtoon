# Feature Specification: Multiple Output File Formats

> **STATUS: COMPLETE** | Merged: 2026-02-12 | Branch: `004-output-formats`

**Feature Branch**: `004-output-formats`
**Created**: 2026-02-12
**Status**: Draft
**Input**: User description: "I'd like to add support for additional output file formats - CSV definitely, parquet and arrow maybe (depending on complexity of data type conversion). This will address issue #4"
**GitHub Issue**: #4

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Export Query Results as CSV (Priority: P1)

A user runs a query against their database and wants the results saved as a CSV file for import into spreadsheet tools, other data pipelines, or quick human-readable inspection. They specify an output file path ending in `.csv`, and the tool writes a well-formed CSV file containing column headers and row data.

**Why this priority**: CSV is the most universally supported tabular data exchange format. It requires minimal transformation from the current text-based output (all values are already text), delivers immediate value for interoperability with virtually any data tool, and is explicitly confirmed as a definite requirement.

**Independent Test**: Can be fully tested by running a query with `--output results.csv` and verifying the resulting file opens correctly in any spreadsheet application or CSV parser, with correct headers and values.

**Acceptance Scenarios**:

1. **Given** a successful query returning rows and columns, **When** the user specifies a CSV output file, **Then** the system writes a valid CSV file with a header row matching column names followed by data rows matching query results.
2. **Given** a query result containing NULL values, **When** exported to CSV, **Then** NULL values are represented as empty fields (no text, just adjacent delimiters).
3. **Given** a query result containing values with commas, quotes, or newlines, **When** exported to CSV, **Then** those values are properly escaped per RFC 4180.
4. **Given** a successful query returning rows, **When** the user does not specify an output file, **Then** the system outputs TOON to stdout (existing behavior).
5a. **Given** the user specifies an output file with a `.toon` or `.txt` extension, **When** the query completes, **Then** the system writes TOON format to that file.
5b. **Given** the user specifies an output path with no extension (e.g., `--output results`), **When** the query completes, **Then** the system appends `.toon` to the path and writes TOON format to the resulting file (e.g., `results.toon`).
6. **Given** the user specifies an output file with an unrecognized extension (e.g., `.dat`, `.xlsx`), **When** the query would run, **Then** the system reports an error listing the supported output formats and does not execute the query.

---

### User Story 2 - Export Query Results as Parquet (Priority: P2)

A user runs a query and wants the results saved as a Parquet file for efficient storage, compression, and consumption by analytics tools. They specify a `.parquet` output path, and the tool writes a valid Parquet file with appropriate column types derived from the query's type metadata.

**Why this priority**: Parquet is the industry-standard columnar format for analytics workloads. It provides significant compression and performance benefits over text formats. However, it requires mapping SQL type metadata to columnar type representations, which adds complexity beyond CSV. Inclusion depends on feasibility of type conversion.

**Independent Test**: Can be fully tested by running a query with `--output results.parquet` and verifying the file is readable by any Parquet-compatible tool (e.g., DuckDB, pandas, polars) with correct column names, types, and values.

**Acceptance Scenarios**:

1. **Given** a successful query with typed columns (integers, strings, decimals, dates), **When** the user specifies a Parquet output file, **Then** the system writes a valid Parquet file with column types that faithfully represent the source SQL types.
2. **Given** a query result containing NULL values, **When** exported to Parquet, **Then** NULL values are represented as native Parquet nulls (not empty strings).
3. **Given** a SQL type that cannot be cleanly mapped to a columnar type, **When** exported to Parquet, **Then** the system falls back to storing that column as a string type rather than failing.
4. **Given** a query returning zero rows, **When** exported to Parquet, **Then** the system writes a valid Parquet file with the correct schema but no data rows.

---

### User Story 3 - Export Query Results as Arrow IPC (Priority: P3)

A user runs a query and wants the results saved as an Arrow IPC file for zero-copy interoperability with Arrow-native tools. They specify a `.arrow` output path, and the tool writes a valid Arrow IPC file with typed columns.

**Why this priority**: Arrow IPC is valuable for in-memory analytics pipelines but has a narrower audience than Parquet. Since Arrow and Parquet share the same underlying type system, Arrow support is a natural extension once Parquet is implemented. Inclusion depends on the same type conversion feasibility as Parquet.

**Independent Test**: Can be fully tested by running a query with `--output results.arrow` and verifying the file is readable by any Arrow-compatible tool with correct column names, types, and values.

**Acceptance Scenarios**:

1. **Given** a successful query with typed columns, **When** the user specifies an Arrow IPC output file, **Then** the system writes a valid Arrow IPC file with appropriate column types.
2. **Given** a query result containing NULL values, **When** exported to Arrow, **Then** NULL values are represented as native Arrow nulls.
3. **Given** a SQL type that cannot be cleanly mapped, **When** exported to Arrow, **Then** the system falls back to storing that column as a string type.

---

### Edge Cases

- What happens when the output file already exists? The system overwrites it (consistent with current TOON file output behavior).
- What happens when the output file has a `.toon` or `.txt` extension? The system writes TOON format.
- What happens when the output file has no extension? The system appends `.toon` to the path and writes TOON format.
- What happens when the output file has an unrecognized extension (e.g., `.dat`, `.xlsx`)? The system rejects the request with an error listing supported formats (`.toon`, `.txt`, `.csv`, `.parquet`, `.arrow`).
- What happens when the disk is full or the path is invalid? The system reports a clear error message and exits with a non-zero status code (consistent with current error handling).
- What happens when a query returns zero rows? All formats produce a valid file with schema/headers but no data rows.
- What happens when column names contain special characters? CSV escapes them per RFC 4180; Parquet and Arrow preserve them as-is in the schema.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support writing query results in CSV format to a file.
- **FR-002**: System MUST produce CSV output conforming to RFC 4180 (comma-delimited, proper quoting and escaping, header row).
- **FR-003**: System MUST represent NULL values as empty fields in CSV output.
- **FR-004**: System MUST detect the output format from the file extension of the `--output` / `-o` path (`.toon` or `.txt` for TOON, `.csv` for CSV, `.parquet` for Parquet, `.arrow` for Arrow IPC). Extension matching MUST be case-insensitive (e.g., `.CSV`, `.Csv`, `.csv` are all recognized as CSV).
- **FR-005**: When the output path has no file extension, the system MUST automatically append `.toon` and write in TOON format.
- **FR-005a**: System MUST reject output files with unrecognized extensions (not `.toon`, `.txt`, `.csv`, `.parquet`, `.arrow`, or absent, compared case-insensitively) with an informative error message that lists the supported formats.
- **FR-006**: System MUST support writing query results in Parquet format to a file, with column types derived from query type metadata. *Conditional*: Include if mapping core SQL types (INT, BIGINT, VARCHAR, DECIMAL, BOOLEAN, DATE/TIMESTAMP) to Parquet types requires <200 LOC of glue code. Exotic/unmappable types use string fallback (FR-009). If this threshold is exceeded, Parquet support is deferred to a follow-up feature.
- **FR-007**: System MUST support writing query results in Arrow IPC format to a file, with column types derived from query type metadata. *Conditional*: Arrow IPC is included only if Parquet is included (they share the same type-mapping work and the same <200 LOC feasibility gate). If Parquet is deferred, Arrow is also deferred.
- **FR-008**: For Parquet and Arrow formats, the system MUST map SQL types to appropriate columnar types (e.g., INT to integer, VARCHAR to string, DECIMAL to decimal, DATE to date).
- **FR-009**: For Parquet and Arrow formats, when a SQL type cannot be cleanly mapped to a columnar type, the system MUST fall back to storing that column as a string.
- **FR-010**: System MUST preserve the existing TOON output behavior as the default — stdout output remains TOON, and `.toon`/`.txt` extensions produce TOON files (no breaking changes).
- **FR-011**: System MUST include column headers/schema in all output formats (header row for CSV, schema for Parquet/Arrow).

### Key Entities

- **Output Format**: The serialization format for query results. Supported formats: TOON (existing default), CSV, Parquet (conditional), Arrow IPC (conditional). Each format has different capabilities regarding type fidelity, human readability, and tool compatibility.
- **Type Mapping**: The correspondence between SQL column types (from query metadata) and target format types. CSV uses string representation for all types; Parquet and Arrow use native typed columns with a string fallback for unmappable types.

## Clarifications

### Session 2026-02-12

- Q: How should diagnostic messages (truncation warning) be handled when non-TOON format goes to stdout? → A: Not applicable — no `--format` flag will be introduced. Output format is determined solely by file extension on `--output`. Stdout is always TOON. The existing stdout diagnostic message issue is tracked separately in BUGS.md.
- Q: Should an explicit `--format` flag be added for format selection? → A: No. Format is determined entirely by file extension of the `--output` path. This keeps the CLI simple and focused on the primary use case: writing files readable by other applications.
- Q: Should file extension matching be case-insensitive? → A: Yes. Case-insensitive matching (`.CSV`, `.Csv`, `.csv` all recognized).
- Q: What criteria determine whether Parquet/Arrow type mapping is feasible? → A: Include if mapping core types (INT, BIGINT, VARCHAR, DECIMAL, BOOLEAN, DATE/TIMESTAMP) requires <200 LOC of glue code; exotic types use string fallback.

## Assumptions

- File overwrite behavior for new formats matches current TOON behavior (overwrite without prompting).
- CSV delimiter is comma (not configurable in this feature; tab-delimited or other variants are out of scope).
- CSV encoding is UTF-8.
- CSV output does not preserve SQL type metadata (inherent limitation of the format; type information is only available via the source query).
- The summary line printed to stdout after file output (row count, file path, truncation status) remains unchanged regardless of output format.
- Parquet and Arrow output, if included, will use reasonable default compression and encoding settings (not user-configurable in this feature).
- No new CLI flags are introduced. Output format is determined entirely by the file extension of the existing `--output` / `-o` path. Stdout output is always TOON.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can export query results to a valid CSV file that is correctly parsed by at least 3 common data tools (spreadsheet application, scripting language CSV library, database import utility) without manual intervention.
- **SC-002**: All existing command-line workflows that do not specify a format continue to produce identical TOON output (zero regressions).
- **SC-003**: Format detection correctly identifies the output format from file extension in 100% of supported extension cases (`.toon`, `.txt`, `.csv`, `.parquet`, `.arrow`).
- **SC-004**: If Parquet is included: Users can export query results to a Parquet file that is readable by standard analytics tools, with correct column types for all standard SQL types (integer, string, decimal, boolean, date/time).
- **SC-005**: If Arrow is included: Users can export query results to an Arrow IPC file that is readable by Arrow-compatible tools, with the same type fidelity as Parquet output.
- **SC-006**: Unrecognized file extensions produce a clear error message listing supported formats, preventing silent misuse. `.toon`/`.txt` extensions produce TOON output; absent extensions auto-append `.toon`.

# Feature Specification: Add Column Types to Output Metadata

**Feature Branch**: `003-column-type-metadata`
**Created**: 2026-02-12
**Status**: Draft
**Input**: User description: "Add SQL column type metadata to TOON query output"
**GitHub Issue**: #5

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Column Types in Query Output (Priority: P1)

A user runs a query against any supported database. The output includes column type metadata (e.g., `INT`, `VARCHAR(255)`, `DECIMAL(10,2)`) alongside the tabular results, allowing the user to understand the data types of each column without consulting the database schema separately.

**Why this priority**: This is the core value of the feature. Without type metadata in the output, users must manually look up schema information to interpret result data types. Adding types directly to output eliminates this context-switching.

**Independent Test**: Can be fully tested by running any query that returns results and verifying that column type names appear in the output, positionally aligned with column headers.

**Acceptance Scenarios**:

1. **Given** a query returning rows with columns of various types, **When** the user runs the query, **Then** the output contains a `types` field listing one type name per column, positionally aligned with the column headers.
2. **Given** a query returning rows, **When** the user views the output, **Then** the `types` field appears before the `rows` field in the output structure.
3. **Given** the output from any query, **When** the output is parsed by a TOON-compliant decoder, **Then** parsing succeeds without errors (output is valid TOON).

---

### User Story 2 - Normalized Type Names Across Backends (Priority: P2)

A user runs the same logical query against different database backends (SQL Server, Databricks). The type names in the output are human-readable, standard SQL type strings (e.g., `VARCHAR(255)`, `INT`, `DECIMAL(10,2)`) regardless of which backend is used. The SQL Server backend currently produces internal debug-format strings (e.g., `Varchar { length: 255 }`) which must be normalized to standard SQL type notation.

**Why this priority**: Consistent, readable type names are essential for users who work across multiple backends or who share query results. Debug-format strings are confusing and unusable for downstream tooling.

**Independent Test**: Can be tested by running queries against a SQL Server backend and verifying the type strings in the output are standard SQL format, not debug format.

**Acceptance Scenarios**:

1. **Given** a SQL Server query with `VARCHAR(255)` columns, **When** the user views the output, **Then** the type name reads `VARCHAR(255)` (not `Varchar { length: 255 }` or other debug format).
2. **Given** a Databricks query with `STRING` and `DECIMAL(10,2)` columns, **When** the user views the output, **Then** the type names read `STRING` and `DECIMAL(10,2)` exactly as provided by Databricks.
3. **Given** any query on any backend, **When** the user views the type names, **Then** all type names follow standard SQL type notation with parameters where applicable (e.g., `VARCHAR(100)`, `DECIMAL(18,2)`, `INT`).

---

### User Story 3 - Type Metadata for Zero-Row Results (Priority: P3)

A user runs a query that returns zero rows. The output still includes column type metadata so the user can verify the schema of the result set even when no data rows are present.

**Why this priority**: Zero-row results still carry useful schema information. Omitting types on empty results would be inconsistent and would prevent users from validating column types during development or debugging.

**Independent Test**: Can be tested by running a query known to return zero rows and verifying the output includes column types and headers.

**Acceptance Scenarios**:

1. **Given** a query returning zero rows, **When** the user views the output, **Then** the output includes a `types` field with the correct type names for each column.
2. **Given** a query returning zero rows, **When** the user views the output, **Then** the output includes column headers and the `types` field, and the output is valid TOON.

---

### Edge Cases

- What happens when a column type has no parameters (e.g., `INT`, `BIT`)? Type name should appear without parentheses.
- What happens when a column type has parameters (e.g., `VARCHAR(255)`, `DECIMAL(10,2)`)? Type name should include the parameters in standard SQL notation.
- What happens when a query returns a single column? The types field should contain exactly one type name.
- What happens when a query returns many columns? The types field should contain one type name per column, all positionally aligned.
- How does the system handle unknown or unmappable SQL Server type variants? The system produces `UNKNOWN` as the fallback type string rather than crashing.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST include a `types` metadata field in query output containing one type name per result column.
- **FR-002**: Type names MUST be positionally aligned with the corresponding column headers — the Nth type corresponds to the Nth column.
- **FR-003**: The `types` field MUST appear before the `rows` field in the output structure.
- **FR-004**: All output MUST remain valid TOON that can be successfully parsed by a TOON-compliant decoder.
- **FR-005**: SQL Server type names MUST be normalized from internal debug format to standard SQL type notation (e.g., `VARCHAR(255)`, `INT`, `DECIMAL(10,2)`).
- **FR-006**: Databricks type names MUST be passed through as-is from the backend (they are already in standard format).
- **FR-007**: Type metadata MUST be included in output even when the query returns zero rows.
- **FR-008**: The system MUST handle all common SQL data types from both backends, producing a readable type string for each.

### Key Entities

- **Column Type**: A string representing the SQL data type of a result column (e.g., `INT`, `VARCHAR(255)`). One per column, positionally aligned with column headers.
- **Output Structure**: The query output changes from a bare tabular array to a root object containing two fields: `types` (array of type strings) and `rows` (the tabular result data).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of query outputs (across all backends) include column type metadata when the query executes successfully.
- **SC-002**: All type names from SQL Server output are human-readable standard SQL type strings — zero instances of debug-format strings (e.g., `Variant { ... }`) in output.
- **SC-003**: 100% of generated outputs pass TOON round-trip validation (encode then decode without error or data loss).
- **SC-004**: Zero-row query outputs include type metadata in 100% of cases.
- **SC-005**: Existing query functionality is not degraded — all previously passing tests continue to pass (updated for new output structure).

## Assumptions

- The existing backend infrastructure already captures column type information and makes it available in the query result metadata. No new backend capabilities are required.
- The Databricks backend provides clean, standard type names that require no transformation.
- The SQL Server backend provides type information in an internal format that requires mapping to standard SQL notation.
- The TOON format supports the proposed root-object structure (`types` + `rows` fields) without requiring format specification changes.
- Values in query results remain stringified — this feature adds metadata only, not runtime type enforcement.

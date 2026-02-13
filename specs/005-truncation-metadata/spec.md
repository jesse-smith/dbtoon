# Feature Specification: Truncation Metadata

> **STATUS: COMPLETE** | Merged: 2026-02-12 | Branch: `005-truncation-metadata`

**Feature Branch**: `005-truncation-metadata`
**Created**: 2026-02-12
**Status**: Complete
**Input**: User description: "Add truncated and message keys to TOON object, and equivalent metadata to parquet and arrow files. CSV should be handled by print summary - but print summary should be valid toon and include both truncated and message keys, for all non-toon outputs, not just CSV."

## Clarifications

### Session 2026-02-12

- Q: Should a human-readable truncation warning also be printed to stderr for interactive visibility, alongside the in-band metadata? → A: Yes — emit a human-readable truncation warning to stderr in addition to in-band metadata.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Self-Describing Truncation in TOON Stdout (Priority: P1)

A user pipes dbtoon's TOON stdout output into another tool for automated processing. When the query result is truncated due to a row limit, the receiving tool must be able to detect this from the data itself — without relying on stderr or any out-of-band signal. The TOON object includes `"truncated": true` and a human-readable `"message"` key, making the result self-describing.

**Why this priority**: This is the core problem. A truncated result that looks complete is silent data loss — the most dangerous failure mode for a data tool. Stdout TOON is the default output path and the one most likely to be piped.

**Independent Test**: Can be fully tested by running a query with a row limit and verifying the TOON object on stdout contains `"truncated": true` and a `"message"` string. When not truncated, the TOON object contains `"truncated": false` and no `"message"` key.

**Acceptance Scenarios**:

1. **Given** a query that returns more rows than the configured limit, **When** output goes to stdout (no `--output`), **Then** the TOON object includes `"truncated": true` and `"message"` with a human-readable description of the truncation.
2. **Given** a query that returns fewer rows than the limit (or no limit is set), **When** output goes to stdout, **Then** the TOON object includes `"truncated": false` and no `"message"` key.
3. **Given** a truncated result piped to another tool, **When** the receiving tool parses the TOON object, **Then** it can programmatically detect truncation by checking the `"truncated"` key.

---

### User Story 2 - Truncation Metadata in Parquet and Arrow Files (Priority: P2)

A user writes query results to a Parquet or Arrow IPC file. When the result is truncated, the file's native metadata carries the truncation signal, so any tool reading the file can detect that the data is incomplete.

**Why this priority**: File outputs are the second most common path and are consumed by downstream tools that may never see the terminal. Parquet and Arrow both support key-value metadata natively, so this is a natural fit.

**Independent Test**: Can be tested by writing a truncated result to a Parquet file and reading back the file metadata to verify `"truncated"` and `"message"` keys exist. Same for Arrow IPC.

**Acceptance Scenarios**:

1. **Given** a truncated query result written to a Parquet file, **When** a tool reads the file's metadata, **Then** it finds key `"dbtoon:truncated"` with value `"true"` and key `"dbtoon:message"` with the truncation description.
2. **Given** a truncated query result written to an Arrow IPC file, **When** a tool reads the schema metadata, **Then** it finds key `"dbtoon:truncated"` with value `"true"` and key `"dbtoon:message"` with the truncation description.
3. **Given** a non-truncated result written to Parquet or Arrow, **When** a tool reads the file metadata, **Then** no `"dbtoon:truncated"` or `"dbtoon:message"` keys are present.

---

### User Story 3 - Valid TOON Print Summary for Non-TOON File Outputs (Priority: P3)

A user writes results to a file in any non-TOON format (CSV, Parquet, Arrow). After writing, the tool prints a summary to stdout. This summary must be valid TOON so that a wrapper script consuming stdout can parse it reliably. The summary includes `"truncated"` and `"message"` keys (when applicable) in addition to the existing `"rows_written"` and `"file"` keys.

**Why this priority**: The print summary is the stdout signal for file-output workflows. Making it valid TOON (instead of ad-hoc `key: value` lines) makes it machine-parseable and consistent with the rest of the tool's contract.

**Independent Test**: Can be tested by writing a truncated result to a CSV file and verifying the stdout summary is a valid TOON object containing `"rows_written"`, `"file"`, `"truncated": true`, and `"message"`.

**Acceptance Scenarios**:

1. **Given** a truncated result written to a CSV file, **When** the summary is printed to stdout, **Then** the summary is a valid TOON object with keys `"rows_written"`, `"file"`, `"truncated"` (true), and `"message"`.
2. **Given** a non-truncated result written to a Parquet file, **When** the summary is printed to stdout, **Then** the summary is a valid TOON object with keys `"rows_written"`, `"file"`, and `"truncated"` (false), and no `"message"` key.
3. **Given** a truncated result written to any non-TOON file format, **When** a wrapper script parses stdout, **Then** it can reliably detect truncation from the TOON summary.

---

### User Story 4 - TOON File Output Includes Truncation Metadata (Priority: P3)

A user writes TOON output to a file (via `--output file.toon`). The file contains the same self-describing truncation metadata as stdout TOON output — `"truncated"` and `"message"` keys in the root object.

**Why this priority**: Completes the coverage so that all TOON output, whether to stdout or file, is self-describing. The print summary (Story 3) also applies to TOON file output.

**Independent Test**: Can be tested by writing a truncated result to a `.toon` file and verifying the file's root object contains `"truncated": true` and `"message"`.

**Acceptance Scenarios**:

1. **Given** a truncated result written to a `.toon` file, **When** the file is read, **Then** the root TOON object contains `"truncated": true` and `"message"`.
2. **Given** a non-truncated result written to a `.toon` file, **When** the file is read, **Then** the root TOON object contains `"truncated": false` and no `"message"` key.

---

### Edge Cases

- What happens when `truncated` is true but no row limit is configured? (This shouldn't occur — truncation is only set when a limit is applied. If it does, `"truncated": true` should still be emitted, with a generic message.)
- What happens when output is TOON to stdout and the result has zero rows but is truncated? (The TOON object should still contain the truncation keys alongside an empty `"rows"` array.)
- What happens with the existing `print_truncation_message` function? (Its stdout behavior is removed — truncation info moves into the TOON object for stdout, and into the print summary for file outputs. A stderr warning replaces the old stdout message for interactive visibility.)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When output format is TOON (stdout or file), the root TOON object MUST include a `"truncated"` key with a boolean value indicating whether the result was truncated.
- **FR-002**: When the result is truncated and output format is TOON, the root TOON object MUST include a `"message"` key with a human-readable string describing the truncation (e.g., "Showing 1000 rows. Use --no-limit to return all rows.").
- **FR-003**: When the result is not truncated and output format is TOON, the root TOON object MUST NOT include a `"message"` key.
- **FR-004**: When a truncated result is written to a Parquet file, the file MUST contain key-value metadata entries `"dbtoon:truncated"` = `"true"` and `"dbtoon:message"` = the truncation description.
- **FR-005**: When a truncated result is written to an Arrow IPC file, the schema MUST contain key-value metadata entries `"dbtoon:truncated"` = `"true"` and `"dbtoon:message"` = the truncation description.
- **FR-006**: When a non-truncated result is written to a Parquet or Arrow IPC file, no `"dbtoon:truncated"` or `"dbtoon:message"` metadata entries MUST be present.
- **FR-007**: For all non-TOON file outputs (CSV, Parquet, Arrow), the stdout print summary MUST be a valid TOON object (not ad-hoc `key: value` text).
- **FR-008**: The print summary TOON object MUST include `"rows_written"`, `"file"`, and `"truncated"` keys for all non-TOON file outputs.
- **FR-009**: When the result is truncated, the print summary TOON object MUST also include a `"message"` key with the truncation description. When not truncated, no `"message"` key.
- **FR-010**: For TOON file output, the print summary MUST also be emitted to stdout as a valid TOON object with the same keys as other file formats.
- **FR-011**: The existing separate `print_truncation_message` behavior (appending non-TOON text after the TOON object on stdout) MUST be removed.
- **FR-012**: When a result is truncated, a human-readable truncation warning MUST also be emitted to stderr, regardless of output format or destination. This provides interactive visibility without polluting the data stream on stdout.

### Key Entities

- **Truncation Metadata**: A set of key-value pairs (`truncated`, `message`) that describe whether a query result was truncated and why. Embedded directly in the output format's native metadata mechanism.
- **Print Summary**: A valid TOON object emitted to stdout after writing results to a file, containing write statistics and truncation metadata.

## Assumptions

- The `"truncated"` key is always present in TOON output (both true and false). This avoids ambiguity — absence of the key would be indistinguishable from an older version of the tool that doesn't emit it.
- The `"message"` key is only present when truncated is true. This keeps non-truncated output clean.
- Parquet and Arrow metadata keys are namespaced with `"dbtoon:"` to avoid collisions with other tools' metadata.
- The truncation message text format remains the existing pattern: "Showing N rows. Use --no-limit to return all rows."
- The print summary applies to all file outputs including TOON files, for consistency.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Any tool consuming dbtoon TOON output (stdout or file) can programmatically determine whether the result is truncated by checking the `"truncated"` key, with zero reliance on stderr or out-of-band signals.
- **SC-002**: Any tool reading dbtoon Parquet or Arrow IPC files can determine truncation status from the file's native metadata, without external context.
- **SC-003**: The stdout print summary for all file outputs is parseable as valid TOON by any TOON-compatible parser.
- **SC-004**: No non-data text (truncation warnings, ad-hoc messages) is emitted to stdout in any output mode. Stdout contains only valid TOON.

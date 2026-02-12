# Tasks: Multiple Output File Formats

> **STATUS: COMPLETE** | Merged: 2026-02-12 | Branch: `004-output-formats`

**Input**: Design documents from `/specs/004-output-formats/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: TDD is required per the constitution check — tests are written before implementation for each module.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add dependencies and create module scaffolding so all subsequent phases compile

- [X] T001 Add `csv = "1.4"`, `arrow = { version = "57", default-features = false, features = ["ipc"] }`, `parquet = { version = "57", default-features = false, features = ["arrow"] }` to Cargo.toml and run `cargo check` to verify resolution
- [X] T002 [P] Create stub source modules in src/ and register in src/lib.rs: `format_detect.rs` (with `OutputFormat` enum skeleton and `detect_format` signature returning `todo!()`), `format_csv.rs` (with `write_csv` signature returning `todo!()`), `format_columnar.rs` (with `sql_type_to_arrow` and `build_record_batch` signatures returning `todo!()`), `format_parquet.rs` (with `write_parquet` signature returning `todo!()`), `format_arrow.rs` (with `write_arrow` signature returning `todo!()`)
- [X] T003 [P] Create empty stub test modules in tests/unit/ and register in tests/unit/mod.rs: `format_detect_test.rs`, `format_csv_test.rs`, `format_columnar_test.rs`, `format_parquet_test.rs`, `format_arrow_test.rs`

**Checkpoint**: `cargo check` and `cargo test` pass with stubs. All new modules are registered.

---

## Phase 2: Foundational — Format Detection (Blocking Prerequisites)

**Purpose**: Implement the `OutputFormat` enum and `detect_format()` function that ALL user stories depend on, and wire up the format-aware dispatch in `main.rs`

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Write format detection tests in tests/unit/format_detect_test.rs covering: `.csv`→Csv, `.parquet`→Parquet, `.arrow`→Arrow, `.toon`→Toon, `.txt`→Toon, no extension→appends `.toon` and returns Toon, `.CSV`/`.Csv` case-insensitivity, unrecognized extension (e.g. `.xlsx`)→error with supported format list. Tests should reference `dbtoon::format_detect::{OutputFormat, detect_format}`.
- [X] T005 Implement `OutputFormat` enum (`Toon`, `Csv`, `Parquet`, `Arrow`) with `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` and `detect_format(path: &Path) -> Result<(OutputFormat, PathBuf), DbtoonError>` in src/format_detect.rs. Use `Path::extension()` with `to_ascii_lowercase()` for case-insensitive matching. Return `DbtoonError::Format` for unrecognized extensions with message: `unsupported output format ".xxx" — supported: .toon, .txt, .csv, .parquet, .arrow`. No-extension paths get `.toon` appended to the returned PathBuf.
- [X] T006 Modify src/main.rs: (1) In `exec_read()` and `exec_write()`, call `format_detect::detect_format()` before `execute_query()` when `app_config.output_file` is `Some` — this ensures unrecognized extensions fail fast before query execution per spec scenario US1-6. (2) Pass the detected `(OutputFormat, PathBuf)` to `output_result()`. (3) Rewrite `output_result()` to dispatch by `OutputFormat`: Toon→existing `format::to_toon()` + `output::write_file()` path; Csv/Parquet/Arrow→call respective writer functions. For now, Csv/Parquet/Arrow arms can return `DbtoonError::Format { message: "not yet implemented".into() }` until their phases complete. (4) Update the verbose message from `"formatting TOON output..."` to be format-aware.

**Checkpoint**: `cargo test` passes — including all existing TOON-related tests, which serve as regression coverage for FR-010 (zero breaking changes to TOON output). Format detection works for all extensions. `output_result()` dispatches by format (non-TOON formats error with "not yet implemented").

---

## Phase 3: User Story 1 — Export Query Results as CSV (Priority: P1) 🎯 MVP

**Goal**: Users can run `dbtoon exec-read "..." -o results.csv` and get a valid RFC 4180 CSV file

**Independent Test**: Write a CSV, verify it parses correctly with headers, correct values, proper NULL/escaping handling

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T007 [US1] Write CSV writer tests in tests/unit/format_csv_test.rs covering: (1) basic output — header row from `ColumnMeta.name` fields + data rows from `CellValue::Text` values; (2) NULL values → empty fields (adjacent delimiters); (3) values containing commas, double quotes, and newlines are escaped per RFC 4180; (4) CRLF line terminator; (5) empty result set (zero rows) → header-only CSV; (6) column names with special characters are escaped. Write to a `Vec<u8>` buffer (no `tempfile` dependency needed). Tests should construct `QueryResult` directly (no DB needed).

### Implementation for User Story 1

- [X] T008 [US1] Implement `write_csv(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>` in src/format_csv.rs. Use `csv::WriterBuilder` with `terminator(Terminator::CRLF)` per research.md R1. Write header row from `result.columns[*].name`, then data rows mapping `CellValue::Text(s)` → `s` and `CellValue::Null` → `""`. Map `csv::Error` to `DbtoonError::Format`.
- [X] T009 [US1] Remove the `"not yet implemented"` placeholder for `OutputFormat::Csv` in the `output_result()` dispatch in src/main.rs — replace with `format_csv::write_csv(result, &path)?`

**Checkpoint**: `cargo test` passes. Running with `-o results.csv` produces a valid RFC 4180 CSV. All US1 acceptance scenarios are satisfied. This is the MVP.

---

## Phase 4: User Story 2 — Export Query Results as Parquet (Priority: P2)

**Goal**: Users can run `dbtoon exec-read "..." -o results.parquet` and get a valid Parquet file with typed columns

**Independent Test**: Write a Parquet file, read it back with the `parquet` crate, verify schema types and values match

### Tests for User Story 2

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T010 [P] [US2] Write columnar type mapping tests in tests/unit/format_columnar_test.rs covering: (1) `sql_type_to_arrow()` maps all types from research.md R3 table — INT→Int32, BIGINT→Int64, VARCHAR(n)→Utf8, DECIMAL(p,s)→Decimal128(p,s), BOOLEAN→Boolean, DATE→Date32, DATETIME2→Timestamp(Microsecond), etc.; (2) unknown/exotic types → Utf8 fallback; (3) `build_record_batch()` with mixed types produces correct Arrow arrays; (4) NULL values produce null entries in arrays; (5) column-level fallback — if any value in a column fails to parse, entire column falls back to Utf8; (6) empty result set → valid schema with zero-row RecordBatch.
- [X] T011 [P] [US2] Write Parquet writer tests in tests/unit/format_parquet_test.rs covering: (1) write a Parquet file from a QueryResult and read it back with `parquet::arrow::arrow_reader::ParquetRecordBatchReader` — verify column names, types, and values; (2) NULL values are native Parquet nulls (not empty strings); (3) empty result set → valid Parquet file with schema and zero rows; (4) string fallback columns are stored as Utf8.

### Implementation for User Story 2

- [X] T012 [US2] Implement `sql_type_to_arrow(type_name: &str) -> DataType` in src/format_columnar.rs. Normalize the input (uppercase, trim) and match against the type mapping table from research.md R3. Parse DECIMAL(p,s) parameters. Unknown types → `DataType::Utf8`.
- [X] T013 [US2] Implement `build_record_batch(result: &QueryResult) -> Result<(Arc<Schema>, RecordBatch), DbtoonError>` in src/format_columnar.rs. For each column: determine target DataType via `sql_type_to_arrow()`, attempt to build a typed Arrow array from the column's `CellValue` entries (Text→parse, Null→null). If any parse fails in a column, rebuild that column as `StringArray` (Utf8 fallback per FR-009). Assemble into a `RecordBatch`. Map `ArrowError` to `DbtoonError::Format`.
- [X] T014 [US2] Implement `write_parquet(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>` in src/format_parquet.rs. Call `format_columnar::build_record_batch()`, create a `File`, write with `parquet::arrow::ArrowWriter` using default (Snappy) compression. Map `ParquetError` to `DbtoonError::Format`.
- [X] T015 [US2] Remove the `"not yet implemented"` placeholder for `OutputFormat::Parquet` in the `output_result()` dispatch in src/main.rs — replace with `format_parquet::write_parquet(result, &path)?`

**Checkpoint**: `cargo test` passes. Running with `-o results.parquet` produces a valid Parquet file readable by standard tools. Type mapping glue code is under 200 LOC (FR-006). All US2 acceptance scenarios are satisfied.

---

## Phase 5: User Story 3 — Export Query Results as Arrow IPC (Priority: P3)

**Goal**: Users can run `dbtoon exec-read "..." -o results.arrow` and get a valid Arrow IPC file with typed columns

**Independent Test**: Write an Arrow IPC file, read it back with the `arrow` crate's IPC reader, verify schema types and values match

**Dependencies**: Shares `format_columnar::build_record_batch()` from US2 (Phase 4). US2 must be complete first.

### Tests for User Story 3

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T016 [US3] Write Arrow IPC writer tests in tests/unit/format_arrow_test.rs covering: (1) write an Arrow IPC file from a QueryResult and read it back with `arrow::ipc::reader::FileReader` — verify column names, types, and values; (2) NULL values are native Arrow nulls; (3) empty result set → valid Arrow IPC file with schema and zero rows.

### Implementation for User Story 3

- [X] T017 [US3] Implement `write_arrow(result: &QueryResult, path: &Path) -> Result<(), DbtoonError>` in src/format_arrow.rs. Call `format_columnar::build_record_batch()`, create a `File`, write with `arrow::ipc::writer::FileWriter`. Map `ArrowError` to `DbtoonError::Format`.
- [X] T018 [US3] Remove the `"not yet implemented"` placeholder for `OutputFormat::Arrow` in the `output_result()` dispatch in src/main.rs — replace with `format_arrow::write_arrow(result, &path)?`

**Checkpoint**: `cargo test` passes. Running with `-o results.arrow` produces a valid Arrow IPC file. All US3 acceptance scenarios are satisfied.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all formats

- [X] T019 [P] Run `cargo clippy` and fix all warnings across new modules in src/format_detect.rs, src/format_csv.rs, src/format_columnar.rs, src/format_parquet.rs, src/format_arrow.rs
- [X] T020 Verify `src/format_columnar.rs` is under 200 LOC (FR-006 feasibility gate) — if exceeded, evaluate whether to defer Parquet/Arrow or refactor
- [X] T021 Run quickstart.md validation scenarios: CSV output, Parquet output, Arrow IPC output, TOON stdout, TOON file (.toon and .txt), no-extension auto-append, unrecognized extension error

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — BLOCKS all user stories
- **US1 CSV (Phase 3)**: Depends on Phase 2. No dependencies on other stories.
- **US2 Parquet (Phase 4)**: Depends on Phase 2. No dependencies on other stories.
- **US3 Arrow (Phase 5)**: Depends on Phase 2 AND Phase 4 (shares `format_columnar` from US2)
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Phase 2 — fully independent
- **User Story 2 (P2)**: Can start after Phase 2 — fully independent
- **User Story 3 (P3)**: Can start after Phase 4 — depends on `format_columnar` module from US2

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD)
- Implementation fills in the tested API
- Wire into `output_result()` dispatch as final step
- Story complete before moving to next priority

### Parallel Opportunities

- T002 and T003 can run in parallel (src/ vs tests/ directories)
- T010 and T011 can run in parallel (different test files, both for US2)
- US1 (Phase 3) and US2 (Phase 4) can run in parallel after Phase 2
- T019 is parallel with T020

---

## Parallel Example: User Story 2

```bash
# Launch both test tasks for US2 together:
Task: "Write columnar type mapping tests in tests/unit/format_columnar_test.rs" (T010)
Task: "Write Parquet writer tests in tests/unit/format_parquet_test.rs" (T011)

# Then implement sequentially (T012 → T013 → T014 → T015):
# format_columnar.rs first (T012, T013), then format_parquet.rs (T014), then wire dispatch (T015)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (add deps, create stubs)
2. Complete Phase 2: Foundational (format detection + dispatch)
3. Complete Phase 3: User Story 1 — CSV export
4. **STOP and VALIDATE**: `cargo test`, CSV output is correct
5. This delivers the confirmed P1 requirement with zero risk

### Incremental Delivery

1. Setup + Foundational → Format detection working, dispatch skeleton ready
2. Add US1 (CSV) → Test independently → MVP complete
3. Add US2 (Parquet) → Test independently → Columnar type mapping proven
4. Add US3 (Arrow IPC) → Test independently → Reuses US2 columnar code
5. Polish → Clippy, feasibility gate, quickstart validation

### Key Risk Mitigation

- **FR-006 feasibility gate**: Research estimates 105-130 LOC for type mapping. If `format_columnar.rs` exceeds 200 LOC after T013, stop and evaluate before proceeding to US3.
- **String fallback (FR-009)**: Implemented at column level — if any cell in a column fails to parse, the entire column becomes Utf8. This is conservative but guarantees valid output.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Tests construct `QueryResult` directly — no database connection needed for any test
- `format_columnar.rs` is shared infrastructure for US2 and US3 but is implemented as part of US2
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently

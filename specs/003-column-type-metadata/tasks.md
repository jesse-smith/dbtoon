# Tasks: Add Column Types to Output Metadata

**Input**: Design documents from `/specs/003-column-type-metadata/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are included — the spec mandates TDD (Constitution IV) and plan.md specifies test-first workflow.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: No new project setup needed — this feature modifies an existing codebase. No new dependencies (`Cargo.toml` unchanged per quickstart.md). Phase 1 is empty.

*(No tasks — existing project structure is sufficient.)*

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Type normalization function — pure, independently testable, required by both US1 (types in output) and US2 (normalized types).

**⚠️ CRITICAL**: User Story 1 and 2 both depend on the normalization function existing before output can include types.

- [x] T001 Write unit tests for `normalize_odbc_type()` covering all `DataType` variants in `tests/unit/format_test.rs` — tests must fail initially (function doesn't exist yet). Cover: simple types (`Integer` → `INT`), types with length (`Varchar { length: Some(255) }` → `VARCHAR(255)`), MAX types (`Varchar { length: None }` → `VARCHAR(MAX)`), precision/scale types (`Decimal { precision, scale }` → `DECIMAL(p,s)`), unknown/fallback (`Unknown` → `UNKNOWN`, `Other` → `UNKNOWN`).
- [x] T002 Implement `normalize_odbc_type(data_type: &DataType) -> String` in `src/backend/sqlserver.rs` — exhaustive `match` on `odbc_api::DataType` per the mapping in research.md R2. Function is `pub(crate)`. All T001 tests must pass.
- [x] T003 Replace `format!("{:?}", ...)` with `normalize_odbc_type()` call at the type-name assignment site (~line 126) in `src/backend/sqlserver.rs`.

**Checkpoint**: `normalize_odbc_type()` works for all ODBC type variants. `cargo test` passes. SQL Server backend now populates `ColumnMeta.type_name` with standard SQL strings.

---

## Phase 3: User Story 1 — Column Types in Query Output (Priority: P1) 🎯 MVP

**Goal**: Query output includes a `types` field listing one type name per column, positionally aligned with column headers. Output is a root object `{ types: [...], rows: [...] }`.

**Independent Test**: Run any query that returns results and verify the output contains `types` before `rows`, with one type string per column.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T004 [US1] Update existing format tests in `tests/unit/format_test.rs` to expect the new root-object output structure (`{ "types": [...], "rows": [...] }`) instead of a bare array. All 4 existing tests should now fail (output format hasn't changed yet).

### Implementation for User Story 1

- [x] T005 [US1] Update `to_toon()` in `src/format.rs` to build a `serde_json::Value::Object` with `types` (array of type strings from `QueryResult.columns`) inserted before `rows` (existing tabular data). Encode the object with `toon_format::encode_default()`. Remove the zero-row special-case manual TOON header (per research.md R4 — the root-object approach handles empty rows naturally). Verify: `cargo test` passes — all T004 tests green, TOON round-trip succeeds.

**Checkpoint**: User Story 1 complete. All query outputs include `types` metadata. Output is valid TOON. Zero-row results handled by same code path (no special case).

---

## Phase 4: User Story 2 — Normalized Type Names Across Backends (Priority: P2)

**Goal**: SQL Server type names in output are human-readable standard SQL strings (not debug format). Databricks types pass through as-is.

**Independent Test**: Run a query against SQL Server and verify type strings are standard SQL format (e.g., `VARCHAR(255)`, not `Varchar { length: 255 }`).

### Implementation for User Story 2

- [x] T006 [US2] Verify end-to-end normalization in `tests/unit/format_test.rs`: (a) construct a `QueryResult` with SQL Server–style normalized type names (e.g., `VARCHAR(255)`, `INT`) and verify the TOON output contains those exact strings (no debug-format leakage); (b) construct a `QueryResult` with Databricks-style type names (e.g., `STRING`, `DECIMAL(10,2)`) and verify they appear unchanged in output (FR-006 passthrough). Run `cargo test` and confirm both pass.

**Checkpoint**: User Story 2 complete. SQL Server types are normalized. Databricks types pass through unchanged (FR-006 — no code changes needed, already standard).

---

## Phase 5: User Story 3 — Type Metadata for Zero-Row Results (Priority: P3)

**Goal**: Zero-row query results still include column type metadata.

**Independent Test**: Run a query known to return zero rows and verify the output includes `types` with correct type names and an empty `rows` array.

### Tests for User Story 3

- [x] T007 [US3] Add or verify a zero-row format test in `tests/unit/format_test.rs`: construct a `QueryResult` with columns but empty rows, call `to_toon()`, and verify the output contains `types` with correct type names and `rows` as an empty array. Test must pass (T005 implementation should already handle this — if it fails, fix `to_toon()`).

**Checkpoint**: User Story 3 complete. Zero-row results include type metadata. All format tests pass.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all stories

- [x] T008 Run `cargo clippy` and fix any warnings in modified files (`src/backend/sqlserver.rs`, `src/format.rs`, `tests/unit/format_test.rs`). Then run `cargo test` full suite — all tests pass, no regressions. Matches quickstart.md verification steps.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Empty — no setup needed
- **Foundational (Phase 2)**: No dependencies — can start immediately. BLOCKS all user stories.
- **User Story 1 (Phase 3)**: Depends on Phase 2 (T002 for normalization function, T003 for wiring)
- **User Story 2 (Phase 4)**: Depends on Phase 2 (T003 for normalized types) and Phase 3 (T005 for types in output). Note: US2's implementation is structurally delivered by T001–T003 in Phase 2; Phase 4 is verification only.
- **User Story 3 (Phase 5)**: Depends on Phase 3 (T005 for output structure)
- **Polish (Phase 6)**: Depends on all user stories complete

### Within Each User Story

- Tests written and failing before implementation
- Implementation makes tests pass
- Commit after each task or logical group

### Parallel Opportunities

- T001 and T004 could be written in parallel (different test concerns, same file — but same file means sequential is safer)
- T007 (US3 test) can run after T005 with no additional code — verification only
- US2 (Phase 4) and US3 (Phase 5) are independent of each other and could run in parallel after US1

---

## Parallel Example: After Phase 2

```text
# After foundational phase completes, US2 verification and US3 test can run in parallel:
Task T006 [US2]: Verify normalization flows through to output
Task T007 [US3]: Add/verify zero-row format test

# But both depend on US1 (T005) completing first.
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 2: Foundational (T001–T003)
2. Complete Phase 3: User Story 1 (T004–T005)
3. **STOP and VALIDATE**: `cargo test` — all format tests pass with new structure
4. MVP delivers: types in output, normalized SQL Server types, zero-row handling (all from same code path)

### Incremental Delivery

1. Phase 2 → Normalization function ready
2. Phase 3 (US1) → Types in output → Validate independently (MVP!)
3. Phase 4 (US2) → Verify normalization end-to-end → Validate
4. Phase 5 (US3) → Verify zero-row handling → Validate
5. Phase 6 → Polish and final validation

### Note on Scope

This feature is compact (~60 lines of new logic, 3 files). The MVP (through Phase 3) delivers all three user stories functionally — US2 and US3 are primarily verification phases confirming the foundational + US1 implementation covers their requirements. The sequential approach (Phase 2 → 3 → 4 → 5 → 6) is natural and efficient for a single developer.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently

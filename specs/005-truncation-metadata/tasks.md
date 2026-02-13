# Tasks: Truncation Metadata

**Input**: Design documents from `/specs/005-truncation-metadata/`
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓, quickstart.md ✓

**Tests**: Included per constitution check (IV. TDD: "Tests written before implementation for each changed module.")

**Organization**: Tasks grouped by user story. US1 is MVP. US4 depends on US1 and US3.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Verify existing build is clean before modifications

- [X] T001 Verify existing build and tests pass via `cargo test && cargo clippy`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: This feature modifies existing files only — no new modules, dependencies, or infrastructure. No blocking prerequisites.

**Note**: The truncation message construction (shared by all stories) is included in US1 as part of the `output_result()` restructuring. For parallel execution of US2/US3, see Dependencies section.

**Checkpoint**: No foundational work needed — user story implementation can begin after Setup.

---

## Phase 3: User Story 1 — Self-Describing Truncation in TOON Stdout (Priority: P1) 🎯 MVP

**Goal**: TOON stdout output includes `"truncated"` (always) and `"message"` (when truncated) keys in the root object, making truncated results self-describing.

**Independent Test**: Run a query with a row limit → verify TOON on stdout contains `"truncated": true` and `"message"`. Run without truncation → verify `"truncated": false` and no `"message"`.

### Tests for User Story 1 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T002 [P] [US1] Write failing test for truncated TOON output containing `"truncated": true` and `"message"` key in `tests/unit/format_test.rs` (include sub-case: zero rows + truncated per spec edge case)
- [X] T003 [US1] Write failing test for non-truncated TOON output containing `"truncated": false` and no `"message"` key in `tests/unit/format_test.rs`

### Implementation for User Story 1

- [X] T004 [US1] Modify `to_toon()` signature in `src/format.rs` to accept `truncated: bool` and `message: Option<&str>`, embed `"truncated"` and conditional `"message"` in root TOON object before encoding
- [X] T005 [US1] Add truncation message construction block and update all `to_toon()` call sites (stdout and TOON file paths) in `src/main.rs` `output_result()` — NOTE: retain existing `print_truncation_message()` call; its removal is deferred to T020

**Checkpoint**: TOON output (stdout and file) is self-describing with truncation metadata. T002/T003 tests pass.

---

## Phase 4: User Story 2 — Truncation Metadata in Parquet and Arrow Files (Priority: P2)

**Goal**: Truncated Parquet and Arrow IPC files include `dbtoon:truncated` and `dbtoon:message` in native file/schema metadata. Non-truncated files have no `dbtoon:` keys.

**Independent Test**: Write truncated result to Parquet → read file metadata → verify `dbtoon:truncated` = `"true"` and `dbtoon:message` present. Same for Arrow IPC. Write non-truncated → verify no `dbtoon:` keys.

### Tests for User Story 2 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T006 [P] [US2] Write failing tests for `dbtoon:truncated` and `dbtoon:message` in Parquet file metadata (truncated and non-truncated cases) in `tests/unit/format_parquet_test.rs`
- [X] T007 [P] [US2] Write failing tests for `dbtoon:truncated` and `dbtoon:message` in Arrow IPC schema metadata (truncated and non-truncated cases) in `tests/unit/format_arrow_test.rs`

### Implementation for User Story 2

- [X] T008 [P] [US2] Implement `with_truncation_metadata()` helper in `src/format_columnar.rs` that adds `dbtoon:truncated` and `dbtoon:message` to an Arrow `Schema` when truncated, returns schema unchanged when not
- [X] T009 [P] [US2] Modify `write_parquet()` in `src/format_parquet.rs` to accept `truncated: bool` and `message: Option<&str>`, use WriterProperties for Parquet file-level metadata
- [X] T010 [P] [US2] Modify `write_arrow()` in `src/format_arrow.rs` to accept `truncated: bool` and `message: Option<&str>`, call `with_truncation_metadata()` on schema before creating `FileWriter`
- [X] T011 [US2] Update Parquet and Arrow call sites in `src/main.rs` `output_result()` to pass `result.truncated` and `message.as_deref()` to `write_parquet()` and `write_arrow()`

**Checkpoint**: Parquet and Arrow files carry truncation metadata when results are truncated. T006/T007 tests pass.

---

## Phase 5: User Story 3 — Valid TOON Print Summary for File Outputs (Priority: P3)

**Goal**: Print summary (stdout after file write) is a valid TOON object with `"rows_written"`, `"file"`, `"truncated"`, and `"message"` (when truncated) keys, replacing the ad-hoc `key: value` format.

**Independent Test**: Write truncated result to CSV → verify stdout summary parses as valid TOON containing all expected keys. Write non-truncated → verify `"truncated": false` and no `"message"`.

### Tests for User Story 3 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T012 [P] [US3] Write failing tests for `print_summary()` producing valid TOON output (truncated and non-truncated cases) in `tests/unit/output_test.rs` (NEW file)

### Implementation for User Story 3

- [X] T013 [US3] Rewrite `print_summary()` in `src/output.rs` to accept `truncated: bool` and `message: Option<&str>`, change return type from `()` to `Result<(), DbtoonError>`, build `serde_json::Value::Object` with `rows_written`, `file`, `truncated`, and conditional `message`, encode via `toon_format::encode_default()`
- [X] T014 [US3] Update all `print_summary()` call sites in `src/main.rs` `output_result()` to pass `result.truncated` and `message.as_deref()`

**Checkpoint**: File output summaries on stdout are valid TOON with truncation metadata. T012 tests pass.

---

## Phase 6: User Story 4 — TOON File Output Includes Truncation Metadata (Priority: P3)

**Goal**: TOON file output (`--output file.toon`) contains the same `"truncated"` and `"message"` keys as stdout TOON, and emits a valid TOON print summary on stdout.

**Depends on**: US1 (`to_toon()` with truncation params), US3 (`print_summary()` with truncation params)

**Independent Test**: Write truncated result to `.toon` file → read file → verify root object has `"truncated": true` and `"message"`. Verify stdout summary is valid TOON with truncation keys.

### Tests for User Story 4 ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T015 [P] [US4] Write failing round-trip test: encode with truncation args via `to_toon()`, write to temp `.toon` file, read back, and verify `"truncated"` and `"message"` keys are preserved in `tests/unit/format_test.rs`
- [X] T016 [P] [US4] Write failing test for TOON file output print summary containing truncation keys in `tests/unit/output_test.rs`

### Implementation for User Story 4

- [X] T017 [US4] Verify TOON file path in `output_result()` in `src/main.rs` passes truncation metadata to `to_toon()` and calls `print_summary()` with truncation args (should be wired from US1 T005 + US3 T014; validate and fix if needed)

**Checkpoint**: TOON file output is self-describing with truncation metadata. T015/T016 tests pass.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Stderr warning, removal of old functions, final validation

> **NOTE: Write test FIRST, ensure it FAILS before implementation**

- [X] T018 Write failing test for `print_truncation_warning()` stderr output in `tests/unit/output_test.rs`
- [X] T019 Implement `print_truncation_warning()` in `src/output.rs` and wire stderr warning call at end of `output_result()` in `src/main.rs` (FR-012)
- [X] T020 Remove `print_truncation_message()` from `src/output.rs` and all call sites in `src/main.rs` (FR-011)
- [X] T021 Remove `to_toon_kv()` from `src/format.rs` and all callers (R5: only callers are `print_truncation_message` and `print_summary`, both replaced)
- [X] T022 Run quickstart.md validation scenarios against modified build
- [X] T023 Final `cargo test && cargo clippy` clean pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — verify build first
- **Foundational (Phase 2)**: N/A — no foundational work for this feature
- **US1 (Phase 3)**: Depends on Setup — includes message construction in `output_result()`
- **US2 (Phase 4)**: Module-level changes (T008-T010) independent of US1; main.rs wiring (T011) depends on US1 T005 for the message variable
- **US3 (Phase 5)**: Module-level changes (T013) independent of US1; main.rs wiring (T014) depends on US1 T005 for the message variable
- **US4 (Phase 6)**: Depends on US1 (to_toon wiring) + US3 (print_summary wiring) — verification only
- **Polish (Phase 7)**: Depends on all user stories complete; T020/T021 (removals) must follow all stories

### User Story Dependencies

- **US1 (P1)**: Start first — establishes message construction pattern in `output_result()`
- **US2 (P2)**: Module changes parallel with US1; main.rs wiring after US1 T005
- **US3 (P3)**: Module changes parallel with US1; main.rs wiring after US1 T005
- **US4 (P3)**: Depends on US1 + US3 complete — mostly verification

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Implementation order: helper functions → core function modifications → main.rs call site wiring
- Story complete when all its tests pass

### Parallel Opportunities

- **Module-level changes**: T004 (format.rs), T008-T010 (format_columnar/parquet/arrow), T013 (output.rs) all touch different files — can proceed in parallel
- **Within US1**: T002 then T003 (same file: `format_test.rs`)
- **Within US2**: T006/T007 (tests) can run in parallel; T008/T009/T010 (impl) can run in parallel
- **Within Phase 7**: T018→T019 sequential (TDD); T020→T021 sequential (caller removal order); these two chains can proceed in parallel

---

## Parallel Example: User Story 2

```bash
# Launch all tests for US2 together:
Task: "Write failing Parquet metadata tests in tests/unit/format_parquet_test.rs"
Task: "Write failing Arrow metadata tests in tests/unit/format_arrow_test.rs"

# Launch all implementations for US2 together (after tests fail):
Task: "Implement with_truncation_metadata() in src/format_columnar.rs"
Task: "Modify write_parquet() in src/format_parquet.rs"
Task: "Modify write_arrow() in src/format_arrow.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001)
2. Complete Phase 3: User Story 1 (T002-T005)
3. **STOP and VALIDATE**: TOON stdout is self-describing — the core problem (silent data loss on piped stdout) is solved
4. This alone delivers the highest-priority value

### Incremental Delivery

1. Setup → Build verified
2. US1 → TOON stdout/file self-describing → **MVP!**
3. US2 → Parquet/Arrow files carry metadata → File consumers can detect truncation
4. US3 → Print summary is valid TOON → Machine-parseable file output summaries
5. US4 → Verify TOON file path complete → Full TOON coverage
6. Polish → Stderr warning + cleanup → Clean interface, no legacy code
7. Each increment is independently testable and committable

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- No new dependencies — `Cargo.toml` unchanged
- ~6 modified source files + 1 new test file (~100-150 LOC net change)
- Commit after each phase or logical task group
- Stop at any checkpoint to validate independently

# Tasks: Rewrite Query Validation as Deny-List with Safe EXEC Allowlist

**Input**: Design documents from `/specs/008-write-query-detection/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md

**Tests**: Included — TDD is NON-NEGOTIABLE per constitution check. Write tests first, verify they fail, then implement.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story. Phases are ordered for TDD compliance: tests (red) before implementation (green).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Update shared types and constants that all user stories depend on

- [X] T001 Update `DenialKind` enum: replace `WriteStatement` with `Dml`, `Ddl`, `Dcl`, `Operational`; remove `Unrecognized`; keep `StoredProcedure`, `SelectInto`, `CteWrappedWrite`, `ParseFailure` in `src/validation.rs`
- [X] T002 Add `SAFE_PROCEDURES` compile-time constant (`&[&str]`, 21 entries from data-model.md) in `src/validation.rs`
- [X] T003 Update test helpers `assert_safe` and `assert_denied` in `tests/unit/validation_test.rs` (no functional change — ensure they still work after DenialKind rename)

---

## Phase 2: US1 Tests — TDD Red Phase (Priority: P1) MVP

**Purpose**: Write tests for legitimate read-only patterns FIRST. These MUST fail on the current allowlist code, proving the red phase before the deny-list rewrite.

> **These tests encode the core problem motivating this feature. Write them, run them, confirm they fail.**

- [X] T004 [P] [US1] Add test: `SET NOCOUNT ON; SELECT * FROM users` → Safe in `tests/unit/validation_test.rs`
- [X] T005 [P] [US1] Add test: `BEGIN TRANSACTION; SELECT * FROM orders; COMMIT` → Safe in `tests/unit/validation_test.rs` *(adjusted from `BEGIN TRAN` — sqlparser doesn't support T-SQL abbreviation)*
- [X] T006 [P] [US1] Add test: `DECLARE @id INT = 1; SELECT * FROM users WHERE id = @id` → Safe in `tests/unit/validation_test.rs`
- [X] T007 [P] [US1] Add test: `SET NOCOUNT ON` (standalone SET, no SELECT) → Safe in `tests/unit/validation_test.rs`
- [X] T008 [P] [US1] Add test: `BEGIN TRANSACTION; SELECT 1; COMMIT` → Safe (no writes inside transaction) in `tests/unit/validation_test.rs`
- [X] T009 [P] [US1] Add test: `BEGIN TRANSACTION; DROP TABLE users; COMMIT` → Denied (write nested inside transaction) in `tests/unit/validation_test.rs`

**Checkpoint**: All US1 tests written and confirmed FAILING on current allowlist. Red phase complete.

---

## Phase 3: Foundational Deny-List Rewrite — TDD Green Phase

**Purpose**: Replace the allowlist core with deny-list core. This is the green phase that makes US1 tests pass.

**CRITICAL**: US1 tests (Phase 2) MUST be written and failing before this phase begins.

- [X] T010 Replace `is_safe_statement()` with `is_denied_statement(stmt: &Statement, dialect: BackendDialect) -> Option<(DenialKind, String)>` in `src/validation.rs` — initial skeleton with `_ => None` catch-all (deny-list default = allow)
- [X] T011 Update `validate()` loop to call `is_denied_statement()` instead of `is_safe_statement()` + `classify_denial()` in `src/validation.rs`
- [X] T012 Remove `is_safe_statement()`, `classify_denial()`, and `classify_query_denial()` functions from `src/validation.rs` (logic absorbed into `is_denied_statement`)
- [X] T013 Migrate `Query` handling into `is_denied_statement`: check `select.into` for SELECT INTO, check `SetExpr::Insert/Update/Delete/Merge` for CTE-wrapped writes — preserved as `check_query_denial()` in `src/validation.rs`
- [X] T014 Add recursive nested statement validation for `StartTransaction`, `If`, `While` blocks — if any nested statement is denied, deny the outer statement in `src/validation.rs`
- [X] T015 [US1] Run `cargo test validation` and verify all US1 tests (T004–T009) now PASS in `tests/unit/validation_test.rs`

**Checkpoint**: Core deny-list mechanism in place. `_ => None` means all previously-rejected safe statements (SET, DECLARE, BEGIN/COMMIT) now pass. US1 green phase complete.

---

## Phase 4: User Story 2 — Write operations still blocked (Priority: P1)

**Goal**: INSERT, UPDATE, DELETE, DROP, GRANT, and all other write operations are denied with category-specific denial reasons.

**Independent Test**: Submit known write operations and verify they are denied with appropriate `DenialKind` and detail messages.

### Tests for User Story 2

> **Write tests FIRST, then add deny-list match arms in `is_denied_statement`**

- [X] T016 [P] [US2] Update existing INSERT/UPDATE/DELETE/DROP tests to expect `Dml` instead of `WriteStatement` in `tests/unit/validation_test.rs` *(already done in Phase 1 — tests were written with new DenialKind names)*
- [X] T017 [P] [US2] Add test: `GRANT SELECT ON users TO public_role` → Denied with `Dcl` in `tests/unit/validation_test.rs`
- [X] T018 [P] [US2] Add test: `REVOKE SELECT ON users FROM public_role` → Denied with `Dcl` in `tests/unit/validation_test.rs`
- [X] T019 [P] [US2] Add test: `CREATE INDEX idx ON users (name)` → Denied with `Ddl` in `tests/unit/validation_test.rs`
- [X] T020 [P] [US2] Add test: `TRUNCATE TABLE users` → Denied with `Ddl` in `tests/unit/validation_test.rs`
- [X] T021 [P] [US2] Add test: `MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET name = source.name` → Denied with `Dml` in `tests/unit/validation_test.rs`
- [X] T022 [P] [US2] Add test: `SELECT * INTO new_table FROM users` → Denied with `SelectInto` (regression check) in `tests/unit/validation_test.rs`
- [X] T023 [P] [US2] Add test: `WITH cte AS (SELECT 1) INSERT INTO users SELECT * FROM cte` → Denied with `CteWrappedWrite` (regression check) in `tests/unit/validation_test.rs`
- [X] T024 [P] [US2] Add test: `BACKUP DATABASE mydb TO DISK = 'path'` on SqlServer → **Result: ParseFailure** (sqlparser 0.61 does not parse T-SQL BACKUP; fails at parse stage) in `tests/unit/validation_test.rs`
- [X] T025 [P] [US2] Add test: `DBCC CHECKDB` on SqlServer → **Result: ParseFailure** (sqlparser 0.61 does not parse T-SQL DBCC; fails at parse stage) in `tests/unit/validation_test.rs`

### Implementation for User Story 2

- [X] T026 [US2] Add DML deny arms in `is_denied_statement`: `Insert`, `Update`, `Delete`, `Merge` → `Dml` with detail format `"Denied: DML statement (INSERT)"` in `src/validation.rs` *(already done in Phase 3)*
- [X] T027 [US2] Add DDL deny arms in `is_denied_statement`: `CreateTable`, `CreateView`, `CreateIndex`, `CreateFunction`, `CreateProcedure`, `CreateTrigger`, `CreateSequence`, `CreateSchema`, `CreateDatabase`, `CreateType`, `CreateDomain`, `CreateExtension`, `CreateVirtualTable`, `CreateMacro`, `CreateSecret`, `CreateStage`, `CreateConnector`, `CreatePolicy`, `AlterTable`, `AlterView`, `AlterSchema`, `AlterIndex`, `AlterType`, `AlterConnector`, `AlterPolicy`, `Drop`, `DropFunction`, `DropProcedure`, `DropTrigger`, `DropExtension`, `DropSecret`, `DropConnector`, `DropPolicy`, `DropOperator`, `DropOperatorFamily`, `DropOperatorClass`, `DropDomain`, `Truncate`, `RenameTable` → `Ddl` in `src/validation.rs`
- [X] T028 [US2] Add DCL deny arms in `is_denied_statement`: `Grant`, `Revoke`, `Deny`, `CreateUser`, `AlterUser`, `CreateRole`, `AlterRole` → `Dcl` in `src/validation.rs`
- [X] T029 [US2] Add Operational deny arms in `is_denied_statement`: `Copy`, `CopyIntoSnowflake`, `LoadData`, `Unload`, `Kill`, `Flush`, `Install`, `AttachDatabase`, `AttachDuckDBDatabase`, `DetachDuckDBDatabase` → `Operational` in `src/validation.rs`
- [X] T030 [US2] Update denial detail message format from `"query would modify state: X"` to `"Denied: {category} statement ({type})"` per FR-016 in `src/validation.rs` *(already done in Phase 3)*
- [X] T031 [US2] Run `cargo test validation` and verify all US2 tests pass in `tests/unit/validation_test.rs`

**Checkpoint**: All write operations denied with category-specific reasons. No regressions on SELECT INTO or CTE-wrapped writes. BACKUP/DBCC behavior documented.

---

## Phase 5: User Story 3 — Schema exploration via safe system procedures (Priority: P2)

**Goal**: EXEC with allowlisted SQL Server system procedures passes validation; non-allowlisted and non-SQL-Server EXEC is denied.

**Independent Test**: Submit EXEC calls for each allowlisted procedure on SQL Server dialect and verify they pass; submit non-allowlisted EXEC and verify denied.

### Pre-existing Test Updates

- [X] T032 [US3] Update existing `test_exec_denied` and `test_execute_denied` tests to use non-allowlisted procedure names (e.g., `EXEC my_custom_proc` instead of `EXEC sp_help`) since sp_help/sp_who will now be Safe on SqlServer in `tests/unit/validation_test.rs`

### Tests for User Story 3

> **Write tests FIRST, ensure they FAIL before adding allowlist logic**

- [X] T033 [P] [US3] Add test: `EXEC sp_help 'users'` on SqlServer → Safe in `tests/unit/validation_test.rs`
- [X] T034 [P] [US3] Add test: `EXEC sp_columns 'orders'` on SqlServer → Safe in `tests/unit/validation_test.rs`
- [X] T035 [P] [US3] Add test: `EXEC SP_HELP 'users'` (uppercase) on SqlServer → Safe (case-insensitive, FR-010) in `tests/unit/validation_test.rs`
- [X] T036 [P] [US3] Add test: `EXEC master.dbo.sp_help 'users'` on SqlServer → Safe (3-part name, FR-015) in `tests/unit/validation_test.rs`
- [X] T037 [P] [US3] Add test: `EXEC dbo.sp_help 'users'` on SqlServer → Safe (2-part name, FR-015) in `tests/unit/validation_test.rs`
- [X] T038 [P] [US3] Add test: `EXEC sp_executesql N'SELECT 1'` on SqlServer → Denied (explicitly excluded, FR-011) in `tests/unit/validation_test.rs`
- [X] T039 [P] [US3] Add test: `EXEC my_custom_proc` on SqlServer → Denied (not in allowlist) in `tests/unit/validation_test.rs`
- [X] T040 [P] [US3] Add test: `EXEC sp_help 'users'` on Databricks → Denied (allowlist is SQL Server-only, FR-013) in `tests/unit/validation_test.rs`
- [X] T041 [P] [US3] Add test: `EXEC sp_help_evil` on SqlServer → Denied (exact match, not prefix) in `tests/unit/validation_test.rs`

### Implementation for User Story 3

- [X] T042 [US3] Add `check_exec_allowlist(name: &ObjectName, dialect: BackendDialect) -> bool` helper: extract final segment of multi-part name, lowercase compare against `SAFE_PROCEDURES`, return false for non-SqlServer dialect in `src/validation.rs`
- [X] T043 [US3] Wire `Execute` arm in `is_denied_statement` to call `check_exec_allowlist` — if allowed return `None`, else return `Some((StoredProcedure, detail))` in `src/validation.rs`
- [X] T044 [US3] Run `cargo test validation` and verify all US3 tests pass in `tests/unit/validation_test.rs`

**Checkpoint**: Safe system procedures pass on SQL Server. All other EXEC denied. Allowlist is case-insensitive and handles multi-part names (2-part and 3-part).

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, cleanup, and regression checks

- [X] T045 [P] Run full `cargo test` (all tests, not just validation) to verify no regressions across codebase
- [X] T046 Run `cargo clippy -- -D warnings` and fix any warnings in `src/validation.rs` and `tests/unit/validation_test.rs`
- [X] T047 Verify `main.rs` still compiles and works — `DenialKind` variant changes are permitted per FR-014 (amended); confirm `main.rs` uses `detail` strings, not enum match in `src/main.rs`
- [X] T048 Run quickstart.md validation: execute the test commands from quickstart.md and confirm all pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **US1 Tests (Phase 2)**: Depends on Phase 1 — write tests, confirm they FAIL (TDD red phase)
- **Foundational (Phase 3)**: Depends on Phase 2 — implement deny-list, confirm US1 tests PASS (TDD green phase)
- **User Story 2 (Phase 4)**: Depends on Phase 3 — tests then deny arms
- **User Story 3 (Phase 5)**: Depends on Phase 3 — tests then allowlist logic
- **Polish (Phase 6)**: Depends on all user stories complete

### User Story Dependencies

- **User Story 1 (P1)**: Tests in Phase 2, verified by Phase 3's `_ => None` catch-all
- **User Story 2 (P1)**: Independent — verified by explicit deny match arms in Phase 4
- **User Story 3 (P2)**: Independent — verified by EXEC allowlist logic in Phase 5
- All three stories modify the same file (`src/validation.rs`) so they execute sequentially, not in parallel

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD red→green)
- Implementation in `src/validation.rs` before verification
- Story complete before moving to next priority

### Parallel Opportunities

- All US1 tests (T004–T009) can be written in parallel
- All US2 tests (T016–T025) can be written in parallel
- All US3 tests (T033–T041) can be written in parallel
- US2 deny arm implementations (T026–T029) are logically independent but touch the same function — execute sequentially
- T045, T046 in Polish phase can run in parallel

---

## Parallel Example: User Story 3

```bash
# Write all US3 tests together (all in tests/unit/validation_test.rs, different test functions):
Task: "Add test: EXEC sp_help on SqlServer → Safe"
Task: "Add test: EXEC SP_HELP uppercase → Safe"
Task: "Add test: EXEC master.dbo.sp_help 3-part → Safe"
Task: "Add test: EXEC dbo.sp_help 2-part → Safe"
Task: "Add test: EXEC sp_executesql → Denied"
Task: "Add test: EXEC my_custom_proc → Denied"
Task: "Add test: EXEC sp_help on Databricks → Denied"
Task: "Add test: EXEC sp_help_evil → Denied"
```

---

## Implementation Strategy

### MVP First (User Story 1 + User Story 2)

1. Complete Phase 1: Setup (update DenialKind, add SAFE_PROCEDURES)
2. Complete Phase 2: Write US1 tests, confirm they FAIL (TDD red)
3. Complete Phase 3: Foundational deny-list rewrite, confirm US1 tests PASS (TDD green)
4. Complete Phase 4: US2 tests + deny arms
5. **STOP and VALIDATE**: Both P1 stories independently functional

### Incremental Delivery

1. Setup → US1 Tests (red) → Foundational (green) → US1 verified (MVP core)
2. Add US2 → Write operations blocked with category-specific reasons
3. Add US3 → Safe EXEC allowlist for SQL Server
4. Polish → Full test suite, clippy clean, regression-free

---

## Notes

- Both source files already exist — no new files created
- `DenialKind` variant rename is a public API change; permitted per FR-014 (amended) since FR-016 requires category-specific denial reasons
- The `_ => None` catch-all is the key architectural shift — unknown = allowed
- T024/T025 are diagnostic tasks: determine whether sqlparser parses BACKUP/DBCC or rejects them, then document the result
- Total: 2 files modified (`src/validation.rs`, `tests/unit/validation_test.rs`)

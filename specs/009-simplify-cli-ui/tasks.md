# Tasks: Simplify CLI Interface

**Input**: Design documents from `/specs/009-simplify-cli-ui/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: TDD is required per CLAUDE.md. Tests are written before implementation in each phase.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependency changes and new module scaffolding

- [X] T001 Add `toml_edit = "0.25"` and remove `directories` from `Cargo.toml`
- [X] T002 [P] Create empty `src/init.rs` with module doc comment, add `pub mod init` to `src/lib.rs`
- [X] T003 [P] Create empty `src/profile.rs` with module doc comment, add `pub mod profile` to `src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story — config path, `$VAR` resolution, CLI restructure, config-missing error

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests for Foundational

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T004 [P] Write unit tests for `default_config_path()` (HOME-based, no `directories` crate) in `tests/unit/config_test.rs`
- [X] T005 [P] Write unit tests for `resolve_env_var()` — literal passthrough, `$VAR` resolution, `$$` escape, unset var error — in `tests/unit/config_test.rs`
- [X] T006 [P] Write CLI parsing tests for new command structure (init, query -P, profile subcommands, warehouse list -P, global flags including `-c` custom config path) in `tests/unit/cli_test.rs`
- [X] T007 [P] Write test for config-missing error message directing user to `dbtoon init` in `tests/unit/config_test.rs`

### Implementation for Foundational

- [X] T008 Replace `directories::ProjectDirs` with `HOME`-based `default_config_path()` in `src/config.rs` (R3)
- [X] T009 [P] Implement `resolve_env_var()`, `resolve_profile_string()`, `resolve_profile_secret()` in `src/config.rs` (R2)
- [X] T010 Restructure `Command` enum in `src/cli.rs` — replace `ExecRead`/`ExecWrite`/`ListWarehouses` with `Init`/`Query`/`Profile`/`Warehouse` per CLI contract; add `QueryArgs`, `ProfileCommand`, `WarehouseCommand` structs with all flags/args
- [X] T011 Implement config-missing check: when config file not found, emit error directing user to `dbtoon init` in `src/config.rs`
- [X] T012 Remove all `DBTOON_*` `env` attributes from clap structs in `src/cli.rs` and remove `DBTOON_*` env-var reads from `src/config.rs`

**Checkpoint**: Foundation ready — `cargo test` passes, CLI parses new commands (handlers may still be `todo!()`), `$VAR` resolution works, config path uses HOME

---

## Phase 3: User Story 1 — First-Time Setup with Config Initialization (Priority: P1) 🎯 MVP

**Goal**: `dbtoon init` creates config file with defaults and example profiles; detects Databricks env vars

**Independent Test**: Run `dbtoon init` in a temp dir and verify config file contents and stdout output

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T013 [P] [US1] Write unit tests for init template generation (default template, env-var-detected template, directory creation, unwritable directory error) in `tests/unit/init_test.rs`
- [X] T014 [P] [US1] Write test for `dbtoon init` when config already exists (should warn, not overwrite) in `tests/unit/init_test.rs`

### Implementation for User Story 1

- [X] T015 [US1] Implement `dbtoon init` logic in `src/init.rs`: template generation, Databricks env var detection, directory creation, already-exists guard (R5)
- [X] T016 [US1] Wire `Init` command dispatch in `src/main.rs` to call init logic

**Checkpoint**: `dbtoon init` works end-to-end in a temp directory

---

## Phase 4: User Story 2 — Execute a Query Using a Profile (Priority: P1)

**Goal**: `dbtoon query -P <profile> "SQL"` executes queries using profile connection settings with CLI overrides

**Independent Test**: Create a profile manually in a temp config, run `dbtoon query -P <profile> "SELECT 1"` and verify execution/error handling

### Tests for User Story 2

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T017 [P] [US2] Write unit tests for profile loading and config resolution (CLI > profile > defaults > Databricks env fallback) in `tests/unit/config_test.rs`
- [X] T018 [P] [US2] Write tests for query input conflict (positional SQL vs `-f`), `--database`/`--catalog` mutual exclusivity, `--no-limit` behavior in `tests/unit/cli_test.rs`
- [X] T019 [P] [US2] Write test for `--allow-write` flag gating write queries in `tests/unit/cli_test.rs`

### Implementation for User Story 2

- [X] T020 [US2] Implement profile loading from TOML config — deserialize `[profiles.<name>]`, resolve `$VAR` fields, apply `[defaults]` fallback in `src/config.rs`
- [X] T021 [US2] Implement config resolution hierarchy: CLI flags > profile > defaults > Databricks env vars in `src/config.rs`
- [X] T022 [US2] Wire `Query` command dispatch in `src/main.rs` — connect `QueryArgs` to existing query execution logic (backend dispatch, output formatting, write validation)
- [X] T023 [US2] Handle `-f`/`--file` SQL input, `--no-limit`, `--limit`, `--timeout`, `--database`/`--catalog`/`--schema` overrides in `src/main.rs`

**Checkpoint**: `dbtoon query -P dev "SELECT 1"` works with a manually-created config file; all override flags function correctly

---

## Phase 5: User Story 3 — Profile Management (Priority: P2)

**Goal**: `profile create/edit/show/list/test/delete/rename` subcommands manage profiles via `toml_edit`

**Independent Test**: Run profile CRUD commands and verify config file is updated correctly

### Tests for User Story 3

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T024 [P] [US3] Write tests for `profile create` — new profile with `$VAR` defaults, `--set` overrides, duplicate name rejection, field validation per backend in `tests/unit/profile_test.rs`
- [X] T025 [P] [US3] Write tests for `profile edit` — `--set key=value`, `--set key=` removal, `--unset key`, invalid field rejection in `tests/unit/profile_test.rs`
- [X] T026 [P] [US3] Write tests for `profile show` (resolved values, masking, `--show-secrets` reveals masked values, unset env-var warning), `profile list`, `profile delete`, `profile rename` in `tests/unit/profile_test.rs`

### Implementation for User Story 3

- [X] T027 [US3] Implement `profile create` in `src/profile.rs` — add `[profiles.<name>]` via `toml_edit`, generate backend-appropriate `$VAR` defaults, apply `--set` overrides, validate fields per backend (R1, R4)
- [X] T028 [US3] Implement `profile edit` in `src/profile.rs` — `--set key=value`, `--set key=` removal, `--unset key` removal via `toml_edit` (R1)
- [X] T029 [P] [US3] Implement `profile show` in `src/profile.rs` — display resolved values with credential masking, show `$VAR` name + resolved value, warn on unset env vars
- [X] T030 [P] [US3] Implement `profile list` in `src/profile.rs` — enumerate `[profiles.*]` keys from config
- [X] T031 [P] [US3] Implement `profile delete` in `src/profile.rs` — remove `[profiles.<name>]` via `toml_edit`
- [X] T032 [P] [US3] Implement `profile rename` in `src/profile.rs` — rename key in `[profiles]` table via `toml_edit`, preserve all fields
- [X] T033 [US3] Wire all `Profile` subcommand dispatches in `src/main.rs`

**Checkpoint**: Full profile CRUD works; config file comments/formatting preserved across edits

---

## Phase 6: User Story 4 — Warehouse Listing via Profile (Priority: P3)

**Goal**: `dbtoon warehouse list -P <profile>` lists Databricks warehouses using profile connection

**Independent Test**: Run `dbtoon warehouse list -P <databricks-profile>` and verify warehouse list is returned

### Tests for User Story 4

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T034 [US4] Write test for `warehouse list` requiring `-P` flag and rejecting legacy `--host`/`--token` flags in `tests/unit/cli_test.rs`

### Implementation for User Story 4

- [X] T035 [US4] Wire `Warehouse` command dispatch in `src/main.rs` — connect `warehouse list -P <profile>` to existing warehouse listing logic

**Checkpoint**: `dbtoon warehouse list -P dbx` works with a configured Databricks profile

---

## Phase 7: User Story 5 — Config File Requirement Enforcement (Priority: P1)

**Goal**: Commands requiring config (query, profile *, warehouse list) show helpful error directing to `dbtoon init`

**Independent Test**: Delete/rename config file, run any config-dependent command, verify helpful error message

*Note: The config-missing check was implemented in Phase 2 (T011). This phase validates it's wired into all command paths.*

### Tests for User Story 5

- [X] T036 [US5] Write integration test verifying `query`, `profile list`, and `warehouse list` all produce the "run dbtoon init" error when config is missing in `tests/unit/config_test.rs`

### Implementation for User Story 5

- [X] T037 [US5] Ensure config-missing guard is applied in all command dispatch paths (query, profile *, warehouse) in `src/main.rs`

**Checkpoint**: All config-dependent commands show the init hint when config is absent

---

## Phase 8: User Story 6 — Config Resolution Hierarchy (Priority: P2)

**Goal**: CLI flags > TOML profile > TOML defaults > Databricks standard env vars, with `$VAR` error on unset

**Independent Test**: Set values at multiple hierarchy levels and verify correct precedence

*Note: Core resolution was implemented in Phase 4 (T021). This phase adds integration-level validation and Databricks env-var fallback.*

### Tests for User Story 6

- [X] T038 [P] [US6] Write integration tests for full resolution hierarchy — CLI override wins, profile wins over defaults, defaults win over Databricks env, `$VAR` to unset var errors in `tests/unit/config_test.rs`
- [X] T039 [P] [US6] Write test for Databricks standard env vars as lowest-priority fallback (e.g., `DATABRICKS_CATALOG` used when no profile/default sets catalog) in `tests/unit/config_test.rs`

### Implementation for User Story 6

- [X] T040 [US6] Implement Databricks standard env-var fallback (lowest priority) for `host`, `token`, `warehouse_id`, `catalog`, `schema` in `src/config.rs`
- [X] T041 [US6] Verify and fix precedence: CLI > profile > defaults > Databricks env in `src/config.rs` and `src/main.rs`

**Checkpoint**: All 4 resolution levels work with correct precedence; unset `$VAR` references error cleanly

---

## Phase 9: User Story 7 — Removal of Legacy Commands and Env Vars (Priority: P2)

**Goal**: `exec-read`, `exec-write`, connection-identity flags, and `DBTOON_*` env vars are fully removed

**Independent Test**: Run removed commands/flags and verify clap rejects them

*Note: Env var removal was done in T012; CLI restructure in T010 already removed old commands. This phase validates completeness.*

### Tests for User Story 7

- [X] T042 [US7] Write tests verifying `exec-read` and `exec-write` are unrecognized subcommands, `--server`/`--host`/`--token` etc. are rejected on `query`, and `DBTOON_*` env vars have no effect in `tests/unit/cli_test.rs`

### Implementation for User Story 7

- [X] T043 [US7] Audit all source files to verify T010/T012 completeness — confirm zero remaining legacy command, flag, or env-var references

**Checkpoint**: Zero legacy CLI surface remains; clap rejects all removed commands/flags

---

## Phase 10: User Story 8 — Updated Documentation (Priority: P3)

**Goal**: README and `--help` text reflect new command structure

**Independent Test**: Review README content and run `dbtoon --help`, `dbtoon query --help` to verify

- [X] T045 [US8] Update README.md — show `dbtoon init` as first step, use `query -P <profile>` in all examples, document Databricks standard env vars only, remove all `exec-read`/`exec-write`/`DBTOON_*` references
- [X] T046 [US8] Review and update all clap `about`/`long_about`/`help` strings in `src/cli.rs` to reflect new structure

**Checkpoint**: README and help text match the new CLI contract

---

## Phase 11: User Story 3 (cont.) — Profile Test Command (Priority: P2)

**Goal**: `profile test <name>` verifies connectivity and reports success/failure

**Independent Test**: Run `dbtoon profile test <name>` against a configured profile

### Tests for Profile Test

- [X] T047 [US3] Write test for `profile test` — missing required fields error, connectivity attempt in `tests/unit/profile_test.rs`

### Implementation for Profile Test

- [X] T048 [US3] Implement `profile test` in `src/profile.rs` — validate required fields, attempt backend connection, report result
- [X] T049 [US3] Wire `profile test` dispatch in `src/main.rs`

**Checkpoint**: `dbtoon profile test mydb` reports connection success or specific failure

---

## Phase 12: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, cleanup, and cross-cutting improvements

- [X] T050 Run full `cargo test` suite and fix any failures
- [X] T051 Run `cargo clippy` and resolve all warnings
- [X] T052 [P] Remove unused imports, dead code, and `todo!()` stubs across all source files
- [X] T053 Run quickstart.md validation — execute the implementation order steps end-to-end

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 Init (Phase 3)**: Depends on Phase 2
- **US2 Query (Phase 4)**: Depends on Phase 2 (and benefits from US1 for config creation)
- **US3 Profile (Phase 5)**: Depends on Phase 2
- **US4 Warehouse (Phase 6)**: Depends on Phase 2 + Phase 4 (query wiring pattern)
- **US5 Config Enforcement (Phase 7)**: Depends on Phase 2 + Phase 4 + Phase 5 + Phase 6 (all command paths must exist)
- **US6 Resolution Hierarchy (Phase 8)**: Depends on Phase 4 (profile loading exists)
- **US7 Legacy Removal (Phase 9)**: Depends on Phase 2 (new CLI in place)
- **US8 Documentation (Phase 10)**: Depends on all functional phases
- **Profile Test (Phase 11)**: Depends on Phase 5 (profile infrastructure)
- **Polish (Phase 12)**: Depends on all prior phases

### User Story Dependencies

- **US1 (Init)**: Independent after Phase 2
- **US2 (Query)**: Independent after Phase 2; benefits from US1 for easier testing
- **US3 (Profile CRUD)**: Independent after Phase 2
- **US4 (Warehouse)**: Depends on query wiring pattern from US2
- **US5 (Config Enforcement)**: Depends on all command paths existing (US2, US3, US4)
- **US6 (Resolution)**: Depends on US2 (profile loading infrastructure)
- **US7 (Legacy Removal)**: Can verify after Phase 2; final audit after all commands wired
- **US8 (Documentation)**: Depends on all functional stories

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Infrastructure (config/resolution) before command wiring
- Command wiring before dispatch in main.rs
- Story complete before moving to next priority

### Parallel Opportunities

- T002, T003 (module scaffolding) in parallel
- T004, T005, T006, T007 (foundational tests) in parallel
- T009 (env var resolution) parallel with T008 (different functions in same file — careful)
- T013, T014 (US1 tests) in parallel
- T017, T018, T019 (US2 tests) in parallel
- T024, T025, T026 (US3 tests) in parallel
- T029, T030, T031, T032 (profile show/list/delete/rename) in parallel
- T038, T039 (US6 tests) in parallel
- T050, T051, T052 (polish) partially in parallel

---

## Parallel Example: User Story 3 (Profile Management)

```bash
# Launch all US3 tests together:
Task: "Write tests for profile create in tests/unit/profile_test.rs" (T024)
Task: "Write tests for profile edit in tests/unit/profile_test.rs" (T025)
Task: "Write tests for profile show/list/delete/rename in tests/unit/profile_test.rs" (T026)

# After tests written, launch independent profile operations:
Task: "Implement profile show in src/profile.rs" (T029)
Task: "Implement profile list in src/profile.rs" (T030)
Task: "Implement profile delete in src/profile.rs" (T031)
Task: "Implement profile rename in src/profile.rs" (T032)
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2 + 5)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: US1 (Init) — users can create config
4. Complete Phase 4: US2 (Query) — users can execute queries
5. **STOP and VALIDATE**: `dbtoon init` → `dbtoon query -P dev "SELECT 1"` works end-to-end
6. Complete Phase 7: US5 (Config Enforcement) after US3/US4 command paths exist

### Incremental Delivery

1. Setup + Foundational → foundation ready
2. US1 (Init) → users can bootstrap config
3. US2 (Query) → core query workflow works
4. US3 (Profile CRUD) → users can manage profiles without editing TOML
5. US4 (Warehouse) → warehouse listing via profile
6. US5 (Config Enforcement) → all command paths guarded (requires US2, US3, US4)
7. US6 (Resolution) → full config resolution hierarchy
8. US7 (Legacy Removal) → clean break from old surface
9. US8 (Documentation) → README reflects new reality
10. Polish → final quality pass

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- TDD: Write tests, verify they fail, then implement
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- `profile test` is split to Phase 11 because it requires backend connectivity (higher complexity, lower priority within US3)

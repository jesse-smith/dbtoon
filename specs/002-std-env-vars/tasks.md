# Tasks: Standard Databricks Environment Variable Fallback

**Input**: Design documents from `/specs/002-std-env-vars/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md

**Tests**: Required — constitution mandates TDD (Principle IV, NON-NEGOTIABLE). Tests written before implementation within each phase.

**Organization**: Tasks grouped by user story. Each story is independently testable.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Test Infrastructure)

**Purpose**: Add env-var test helpers that all user story tests depend on

- [ ] T001 Add static `ENV_MUTEX: Mutex<()>` and `EnvGuard` drop-based cleanup struct to `tests/unit/config_test.rs` — the guard sets env vars on creation, removes them on `Drop`, and holds the mutex lock for the duration. See plan.md D4 for design.

**Checkpoint**: Test infrastructure ready — `cargo test` passes with no new test failures

---

## Phase 2: Foundational (Helper Functions)

**Purpose**: Add `non_empty` and `env_non_empty` helper functions that all Databricks resolution changes depend on

- [ ] T002 Write unit tests for `non_empty(Option<&str>) -> Option<&str>` in `tests/unit/config_test.rs` — test cases: `None` → `None`, `Some("")` → `None`, `Some("value")` → `Some("value")`
- [ ] T003 Write unit tests for `env_non_empty(key: &str) -> Option<String>` in `tests/unit/config_test.rs` — test cases: var unset → `None`, var set to `""` → `None`, var set to `"value"` → `Some("value")`. Use `EnvGuard` from T001.
- [ ] T004 Implement `non_empty` and `env_non_empty` as `pub(crate)` functions in `src/config.rs`. `non_empty` filters `Some("")` to `None`. `env_non_empty` calls `std::env::var` and filters empty strings. See plan.md D3.
- [ ] T005 Verify T002 and T003 tests pass with `cargo test`

**Checkpoint**: Helpers implemented and tested — `cargo test` passes, `cargo clippy` clean

---

## Phase 3: User Story 1 — Standard Databricks Env Vars as Fallback (Priority: P1) 🎯 MVP

**Goal**: When dbtoon-specific env vars are absent and no TOML profile is set, standard Databricks env vars (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`) are used.

**Independent Test**: Set only standard env vars, call `load_from_exec_args` with backend=databricks, verify resolved `BackendConfig::Databricks` fields match the standard env var values.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before T008 implementation**

- [ ] T006 [US1] Write test `test_databricks_std_env_fallback` in `tests/unit/config_test.rs` — set all 5 standard Databricks env vars via `EnvGuard`, call `load_from_exec_args` with `backend=Some("databricks")` and no host/token/warehouse/catalog/schema args, assert all 5 fields resolve from the standard env vars. Covers spec acceptance scenario US1.1 and FR-001.
- [ ] T007 [US1] Write test `test_dbtoon_env_overrides_std_env` in `tests/unit/config_test.rs` — set both dbtoon-specific (`DBTOON_DATABRICKS_HOST`) and standard (`DATABRICKS_HOST`) env vars to different values, construct `ExecArgs` with `host=None` (so clap env kicks in — but since we construct `ExecArgs` directly, simulate by setting `args.host = Some("dbtoon-host")`), assert dbtoon value wins. Covers spec acceptance scenario US1.2 and FR-002.

### Implementation for User Story 1

- [ ] T008 [US1] Modify the `"databricks"` branch in `load_from_exec_args` in `src/config.rs` to add standard env var fallback for all 5 fields. For non-secret fields (host, warehouse_id, catalog, schema): wrap existing `args.*` and `profile.*` lookups with `non_empty()`, chain `.or(env_non_empty("DATABRICKS_*").as_deref())` after the TOML profile tier. For token: chain `.or_else(|| env_non_empty("DATABRICKS_TOKEN").map(SecretString::from))` after the existing `profile.token` fallback. Create temporary `let` bindings for `env_non_empty` results to satisfy borrow lifetimes. See plan.md D1, D2, D5 for the exact mapping table.
- [ ] T009 [US1] Run `cargo test` — verify T006 and T007 pass and all pre-existing tests still pass (SC-002)

**Checkpoint**: `load_from_exec_args` supports standard env var fallback. MVP complete — users can connect with only standard vars set.

---

## Phase 4: User Story 2 — TOML Profile Values Override Standard Env Vars (Priority: P2)

**Goal**: TOML profile values take precedence over standard Databricks env vars. Also extend fallback to `load_from_list_warehouses_args`.

**Independent Test**: Set standard env vars, configure a TOML profile with different values, call `load_from_exec_args` with that profile, verify TOML values are used.

### Tests for User Story 2

> **NOTE: Write these tests FIRST, ensure they FAIL (for list-warehouses) or verify they PASS (for exec-args TOML override) before T013 implementation**

- [ ] T010 [US2] Write test `test_toml_profile_overrides_std_env` in `tests/unit/config_test.rs` — create a temp TOML config file with a profile containing `host`, `token`, `warehouse_id`, `catalog`, `schema` values. Set standard env vars to different values. Call `load_from_exec_args` with `profile=Some("test")` and the temp config path. Assert all fields match the TOML values, not the env vars. Covers spec US2 acceptance scenario 1, FR-002.
- [ ] T011 [US2] Write test `test_toml_partial_profile_falls_through_to_std_env` in `tests/unit/config_test.rs` — create a temp TOML config file with a profile that has `host` and `token` but NOT `catalog`. Set `DATABRICKS_CATALOG=env-catalog`. Call `load_from_exec_args`. Assert `host` comes from TOML, `catalog` comes from standard env var. Covers spec US2 acceptance scenario 2, FR-003.

- [ ] T012 [US2] Write test `test_list_warehouses_std_env_fallback` in `tests/unit/config_test.rs` — set all 5 standard Databricks env vars via `EnvGuard`, call `load_from_list_warehouses_args` with `backend=Some("databricks")` and no host/token/warehouse/catalog/schema args, assert all 5 fields resolve from the standard env vars. Mirrors T006 but for the list-warehouses code path. Covers FR-001 for list-warehouses.

### Implementation for User Story 2

- [ ] T013 [US2] Modify `load_from_list_warehouses_args` in `src/config.rs` to add the same standard env var fallback pattern for host, token, warehouse_id, catalog, and schema. Apply `non_empty()` wrapping and `env_non_empty()` fallbacks matching the pattern from T008.
- [ ] T014 [US2] Run `cargo test` — verify T010, T011, T012 pass, all pre-existing tests still pass

**Checkpoint**: Both `load_from_exec_args` and `load_from_list_warehouses_args` support standard env var fallback with correct TOML precedence.

---

## Phase 5: User Story 3 — Comprehensive Priority-Ladder Tests (Priority: P2)

**Goal**: Test coverage for all tiers of the priority ladder and edge cases, ensuring no regressions as the config system evolves.

**Independent Test**: Run `cargo test` and verify all new tests pass, covering at least 5 distinct priority-ladder scenarios (SC-003).

### Tests for User Story 3

- [ ] T015 [US3] Write test `test_cli_flag_overrides_all_tiers` in `tests/unit/config_test.rs` — set standard env vars, create TOML profile, AND set `args.host = Some("cli-host")`. Assert CLI value wins. Covers the full ladder: CLI > TOML > std env.
- [ ] T016 [US3] Write test `test_empty_dbtoon_env_falls_through_to_std_env` in `tests/unit/config_test.rs` — set `args.host = Some("")` (simulating empty `DBTOON_DATABRICKS_HOST`) and `DATABRICKS_HOST=std-host`. Assert `std-host` is used. Covers FR-004, spec edge case 2.
- [ ] T017 [US3] Write test `test_empty_std_env_treated_as_unset` in `tests/unit/config_test.rs` — set `DATABRICKS_HOST=""` with no other host source. Assert error (host is required). Set `DATABRICKS_CATALOG=""` with no other catalog source. Assert `catalog` is `None`. Covers FR-004, FR-007, spec edge case 1.
- [ ] T018 [US3] Write test `test_independent_field_resolution` in `tests/unit/config_test.rs` — set `args.host = Some("cli-host")`, `DATABRICKS_TOKEN=std-token`, `DATABRICKS_SQL_WAREHOUSE_ID=std-wh` (no TOML profile). Assert host from CLI, token from std env, warehouse from std env. Covers FR-003, spec edge case 3.
- [ ] T019 [US3] Write test `test_std_env_token_fallback` in `tests/unit/config_test.rs` — set only `DATABRICKS_TOKEN=std-token` (no `args.token`, no `profile.token_env`, no `profile.token`). Assert token resolves to `std-token`. Covers the specific `resolve_secret` chain + new fallback for tokens (plan.md D2).
- [ ] T020 [US3] Write test `test_dotenv_std_vars_participate` in `tests/unit/config_test.rs` — write a `.env` file containing `DATABRICKS_HOST=dotenv-host` to a temp directory, call `dotenvy::from_path()` on it, then call `load_from_exec_args` with no host arg. Assert host resolves to `dotenv-host`. Covers spec edge case 4.
- [ ] T021 [US3] Run `cargo test` — verify all tests pass (T015–T020 + all pre-existing). Confirm at least 6 distinct priority-ladder scenarios covered (SC-003). Run `cargo clippy` — verify no warnings.

**Checkpoint**: Comprehensive test coverage in place. All priority-ladder tiers exercised. Edge cases covered.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and cleanup

- [ ] T022 Run `cargo clippy` and fix any warnings introduced by new code in `src/config.rs`
- [ ] T023 Run `cargo test` end-to-end — verify all existing tests pass unmodified (SC-002) and no new warnings
- [ ] T024 Verify no changes to `src/cli.rs` or any other file outside `src/config.rs` and `tests/unit/config_test.rs` (SC-004, FR-005)

**Checkpoint**: Feature complete. All tests pass, clippy clean, no unintended changes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 (uses `EnvGuard`)
- **US1 (Phase 3)**: Depends on Phase 2 (uses `non_empty`, `env_non_empty`, `EnvGuard`)
- **US2 (Phase 4)**: Depends on Phase 3 (reuses US1 implementation pattern; TOML override tests validate US1's correctness)
- **US3 (Phase 5)**: Depends on Phase 4 (all implementation complete; this phase is tests-only)
- **Polish (Phase 6)**: Depends on Phase 5

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational (Phase 2) — no story dependencies
- **US2 (P2)**: Depends on US1 implementation — TOML override tests require the standard env var fallback to already exist
- **US3 (P2)**: Depends on US1 + US2 implementation — comprehensive tests exercise all tiers

### Within Each Phase (TDD Order)

1. Write tests FIRST — verify they fail (or compile-error if testing unimplemented functions)
2. Implement the minimum code to make tests pass
3. Run `cargo test` to confirm
4. Commit the phase as one minimum viable unit of work

### Commit Boundaries (per constitution V)

- **Commit 1**: T001–T005 (test infrastructure + helpers)
- **Commit 2**: T006–T009 (US1: standard env var fallback in exec-args)
- **Commit 3**: T010–T014 (US2: TOML override tests + list-warehouses fallback)
- **Commit 4**: T015–T021 (US3: comprehensive priority-ladder tests)
- **Commit 5**: T022–T024 (polish — only if changes needed)

---

## Parallel Example: Phase 2

```text
# These tests touch the same file but test independent functions:
T002: Unit tests for non_empty
T003: Unit tests for env_non_empty
# Not marked [P] because they're in the same file — write sequentially then implement
```

## Parallel Example: Phase 5

```text
# All US3 tests are independent scenarios in the same file:
T015: CLI overrides all tiers
T016: Empty dbtoon env falls through
T017: Empty std env treated as unset
T018: Independent field resolution
T019: Token fallback chain
T020: Dotenv std vars participate
# Write all tests, then run T021 to verify
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Test infrastructure
2. Complete Phase 2: Helper functions
3. Complete Phase 3: US1 — standard env var fallback in `load_from_exec_args`
4. **STOP and VALIDATE**: `cargo test` passes, users can connect with standard env vars
5. This is a usable feature — can be reviewed/merged as-is if needed

### Incremental Delivery

1. Phases 1+2 → Foundation ready (Commit 1)
2. Phase 3 → US1 complete → MVP usable (Commit 2)
3. Phase 4 → US2 complete → TOML override verified, list-warehouses works (Commit 3)
4. Phase 5 → US3 complete → Full test coverage (Commit 4)
5. Phase 6 → Polish → Ship-ready (Commit 5, if needed)

---

## Notes

- All files modified: `src/config.rs` (implementation), `tests/unit/config_test.rs` (tests) — just 2 files
- `src/cli.rs` is NOT modified — clap `env` bindings stay as-is
- `resolve_secret` is NOT modified — standard token env var is chained after its call
- Tests that set env vars MUST use `EnvGuard` to ensure cleanup and serialization
- Temp TOML files for profile tests should use `tempfile` or write to a known temp path and clean up

# Tasks: Self-Contained SQL Server Backend

**Input**: Design documents from `/specs/007-tiberius-mssql/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included — TDD approach mandated by project constitution (plan.md Constitution Check IV) and project guidelines.

**Organization**: This feature is a single-module rewrite (`src/backend/sqlserver.rs`). User stories US1 (macOS integrated auth), US2 (SQL login), and US3 (Linux integrated auth) are implemented within the US4 (seamless migration) rewrite since they represent auth branches in the same function (`build_tiberius_config`). Each target platform (macOS, Linux, Windows) has a separate verification task in Phase 4.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Swap dependencies from ODBC to tiberius

- [X] T001 Update Cargo.toml: remove `odbc-api` from `[dependencies]` and `[dev-dependencies]`, add `tiberius` 0.12 (features: `tds73`, `native-tls`, `sql-browser-tokio`), `tokio-util` 0.7 (features: `compat`), `futures-util` 0.3; add platform-conditional `integrated-auth-gssapi` feature for tiberius under `[target.'cfg(not(windows))'.dependencies]` in Cargo.toml

---

## Phase 2: Foundational — TDD Scaffold + Tests (Red Phase)

**Purpose**: Establish the new module structure and write all unit tests before implementation. Tests will compile but fail (red phase of TDD).

**CRITICAL**: No implementation work (Phase 3) can begin until this phase is complete.

- [X] T002 Rewrite src/backend/sqlserver.rs with new imports (`tiberius`, `tokio_util`, `futures_util`), `SqlServerBackend` struct (unchanged fields: server, database, auth, trust_server_certificate), constructor, and stub function signatures for `parse_server_address`, `normalize_tiberius_type`, `column_data_to_string`, `build_tiberius_config`, `describe_result_columns`, and `Backend::execute` (all stubs use `todo!()`) in src/backend/sqlserver.rs
- [X] T003 [P] Write `parse_server_address` unit tests covering all 9 format variants: plain hostname, hostname with port, hostname with instance, hostname with instance and port, `tcp:` prefix stripping, IPv4 address, IPv4 with port, IPv4 with instance, invalid port error — per contracts/server-address-parsing.md in tests/unit/sqlserver_test.rs
- [X] T004 [P] Write `normalize_tiberius_type` unit tests covering all 27+ `ColumnType` variants: Null->UNKNOWN, Bit/Bitn->BIT, Int1->TINYINT, Int2->SMALLINT, Int4/Intn->INT, Int8->BIGINT, Float4->REAL, Float8/Floatn->FLOAT, Money->MONEY, Money4->SMALLMONEY, Datetime/Datetimen->DATETIME, Datetime4->SMALLDATETIME, Datetime2->DATETIME2, Daten->DATE, Timen->TIME, DatetimeOffsetn->DATETIMEOFFSET, Decimaln->DECIMAL, Numericn->NUMERIC, BigVarChar->VARCHAR, BigChar->CHAR, NVarchar->NVARCHAR, NChar->NCHAR, BigVarBin->VARBINARY, BigBinary->BINARY, Guid->UNIQUEIDENTIFIER, Xml->XML, Text->TEXT, NText->NTEXT, Image->IMAGE, SSVariant->SQL_VARIANT, Udt->UNKNOWN — per contracts/type-normalization.md in tests/unit/sqlserver_test.rs
- [X] T005 [P] Write `column_data_to_string` unit tests covering all 17+ `ColumnData` variants: U8/I16/I32/I64 as decimal integers, F32/F64 as floats, Bit as `0`/`1` (not true/false), String as-is, Guid as hyphenated UUID, Binary with `0x` hex prefix, Numeric with trailing zeros preserved, DateTime as `YYYY-MM-DD HH:MM:SS.mmm`, SmallDateTime as `YYYY-MM-DD HH:MM:SS`, Date as `YYYY-MM-DD`, Time as `HH:MM:SS.nnnnnnn`, DateTime2 as `YYYY-MM-DD HH:MM:SS.nnnnnnn`, DateTimeOffset with timezone, Xml as-is, all None variants as CellValue::Null — per contracts/type-normalization.md value-to-string table in tests/unit/sqlserver_test.rs
- [X] T006 Register `sqlserver_test` module in tests/unit/mod.rs

**Checkpoint**: All tests compile but fail (red). Scaffold compiles with `todo!()` stubs.

---

## Phase 3: User Story 4 — Seamless Migration (Priority: P1) + US1 + US2

**Goal**: Complete tiberius backend with full behavioral parity — all CLI flags, config keys, environment variables, output formats, and error behaviors identical to ODBC backend. Implements US1 (macOS integrated auth) and US2 (SQL login) within the same rewrite since they are auth branches in a single function.

**Independent Test**: `cargo test` — all existing + new unit tests pass. `cargo clippy -- -D warnings` — zero warnings.

### Implementation for User Story 4

- [ ] T007 [US4] Implement `parse_server_address` function: split on `\` for instance name, split on `,` for port, strip `tcp:` prefix, return `Result<(host, Option<u16>, Option<String>), DbtoonError>`, return `Err(DbtoonError::Config)` for invalid port values in src/backend/sqlserver.rs
- [ ] T008 [P] [US4] Implement `normalize_tiberius_type` fallback mapper: match all `ColumnType` variants to uppercase SQL type strings (Null->UNKNOWN, Bit/Bitn->BIT, Int1->TINYINT, Int2->SMALLINT, Int4/Intn->INT, Int8->BIGINT, Float4->REAL, Float8/Floatn->FLOAT, Money->MONEY, Money4->SMALLMONEY, all datetime variants, Decimaln->DECIMAL, Numericn->NUMERIC, all string/binary/special types) in src/backend/sqlserver.rs
- [ ] T009 [P] [US4] Implement `column_data_to_string` converter: match all `ColumnData` variants with ODBC-parity formatting — Bit as `0`/`1`, Numeric with trailing zeros via scale, DateTime2 with 7-digit fractional seconds, Guid as hyphenated lowercase, Binary with `0x` hex uppercase, all None variants as `CellValue::Null` in src/backend/sqlserver.rs
- [ ] T010 [US4] Implement `build_tiberius_config` method: call `parse_server_address` to extract host/port/instance, set `config.host()`, `config.port()`, `config.instance_name()`, set `config.database()`, dispatch auth (`SqlServerAuth::WindowsIntegrated` -> `AuthMethod::Integrated`, `SqlServerAuth::SqlLogin` -> `AuthMethod::sql_server(user, pass)`), set `config.trust_cert()` when `trust_server_certificate` is true, default `EncryptionLevel::Required` in src/backend/sqlserver.rs
- [ ] T011 [US4] Implement `describe_result_columns` async function: execute `sys.dm_exec_describe_first_result_set(@P1, NULL, 0)` as parameterized query on the client, iterate result rows to extract `name` and `system_type_name` (uppercased) into `Vec<ColumnMeta>`; on any error (permissions, unsupported query), fall back to reading `QueryStream` column metadata and mapping via `normalize_tiberius_type`, emitting a diagnostic warning in src/backend/sqlserver.rs
- [ ] T012 [US4] Implement `Backend::execute`: call `build_tiberius_config`, extract `(host, port, instance_name)` from `parse_server_address`, TCP connect via `TcpStream::connect((host, port))`, apply `.compat_write()` adapter, `Client::connect(config, stream)` (tiberius handles SQL Browser resolution internally when `config.instance_name()` is set via `sql-browser-tokio` feature), call `describe_result_columns`, execute user query via `client.query(sql, &[])`, stream rows via `QueryStream::try_next()` converting each row's columns with `column_data_to_string`, enforce row limit with `truncated = true`, wrap entire operation in `tokio::time::timeout(Duration::from_secs(timeout_secs))`, map tiberius errors to `DbtoonError` variants (connection errors -> `Connection`, auth errors -> `Auth`, query errors -> `Query`, elapsed -> `Timeout`) in src/backend/sqlserver.rs

**Checkpoint**: All unit tests pass (green). `cargo test` succeeds. `cargo clippy -- -D warnings` passes. Binary compiles without `odbc-api`.

---

## Phase 4: Platform Auth Verification (US1/US3/FR-004)

**Goal**: Verify that integrated auth is correctly configured for all three target platforms.

**Independent Test**: Inspect Cargo.toml platform-conditional sections. Optionally cross-check with `cargo check --target <triple>` if cross-compilation toolchain is available.

### macOS Verification (US1, FR-002)

- [ ] T013a [US1] Verify macOS integrated auth configuration: confirm `[target.'cfg(not(windows))'.dependencies]` section in Cargo.toml enables `integrated-auth-gssapi` feature for tiberius (macOS uses GSS.framework at runtime — no additional packages required); optionally verify with `cargo check --target aarch64-apple-darwin` or `x86_64-apple-darwin`

### Linux Verification (US3, FR-003)

- [ ] T013b [US3] Verify Linux integrated auth configuration: confirm `[target.'cfg(not(windows))'.dependencies]` section in Cargo.toml enables `integrated-auth-gssapi` feature for tiberius (Linux links against system `libgssapi-krb5` at runtime); optionally verify with `cargo check --target x86_64-unknown-linux-gnu` if cross-compilation toolchain is available

### Windows Verification (FR-004)

- [ ] T013c [US4] Verify Windows integrated auth configuration: confirm `[target.'cfg(windows)'.dependencies]` section in Cargo.toml does NOT include `integrated-auth-gssapi` and instead relies on tiberius default `winauth` feature for SSPI; optionally verify with `cargo check --target x86_64-pc-windows-msvc` if cross-compilation toolchain is available

**Checkpoint**: Cargo.toml has correct platform-conditional features for all three platforms. macOS uses GSS.framework, Linux links `libgssapi-krb5`, Windows uses SSPI via default `winauth`.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final verification and cleanup across all user stories

- [ ] T014 Update tests/unit/config_test.rs to remove any `odbc-api` dev-dependency references if present
- [ ] T015 Run full test suite (`cargo test`) and linter (`cargo clippy -- -D warnings`) — verify zero failures and zero warnings; explicitly confirm: (a) credential masking tests pass (FR-012 — passwords redacted by default, exposed with `--show-secrets`), (b) Databricks backend tests pass unmodified (FR-013 — no cross-backend regressions)
- [ ] T016 Run quickstart.md manual validation scenarios against a SQL Server instance (SQL login auth, Windows auth, named instance, trust cert flag)
- [ ] T017 Measure binary size (SC-007): record current ODBC-based release binary size, build the tiberius-based binary with `cargo build --release`, compare sizes and confirm increase is <50%
- [ ] T018 Measure memory usage for large result sets (SC-006): run a query returning 100k+ rows against a SQL Server instance, monitor peak RSS (e.g., via `/usr/bin/time -l` on macOS or `command time -v` on Linux), and confirm it does not exceed the ODBC baseline by more than 20%

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001) — BLOCKS all user stories
- **US4+US1+US2 (Phase 3)**: Depends on Foundational (Phase 2) completion
- **Platform Verification (Phase 4)**: Depends on Setup (T001) — can run in parallel with Phase 3
- **Polish (Phase 5)**: Depends on Phase 3 and Phase 4 completion

### User Story Dependencies

- **User Story 4 (P1)**: Can start after Foundational (Phase 2) — encompasses US1 and US2 implementation
- **User Story 1 (P1)**: Implemented within US4 (T010 — integrated auth branch in `build_tiberius_config`); verified by T013a (macOS platform config)
- **User Story 2 (P2)**: Implemented within US4 (T010 — SQL login branch in `build_tiberius_config`)
- **User Story 3 (P3)**: Depends on T001 (Cargo.toml features) — verification only (T013b), same code as US1
- **FR-004 (Windows)**: Depends on T001 (Cargo.toml features) — verification only (T013c)

### Within Phase 3 (US4)

- T007 MUST complete before T010 (config builder calls `parse_server_address`)
- T008 and T009 are [P] — can run in parallel with each other and with T007
- T010 depends on T007 (uses `parse_server_address`)
- T011 depends on T008 (fallback path uses `normalize_tiberius_type`)
- T012 depends on T009 + T010 + T011 (orchestrates all components)

### Parallel Opportunities

- Phase 2: T003, T004, T005 are [P] — different test groups, can be authored in parallel
- Phase 3: T008 and T009 are [P] — independent pure functions with no shared state
- Phase 4: T013a, T013b, T013c are [P] — independent platform checks; can also run in parallel with Phase 3

---

## Parallel Example: Phase 3 Implementation

```bash
# After T007 (parse_server_address) is complete:
# Launch independent pure functions together:
Task T008: "Implement normalize_tiberius_type in src/backend/sqlserver.rs"
Task T009: "Implement column_data_to_string in src/backend/sqlserver.rs"

# After T008, T009, T010 are complete:
# These depend on prior tasks:
Task T011: "Implement describe_result_columns in src/backend/sqlserver.rs"
Task T012: "Implement Backend::execute in src/backend/sqlserver.rs"
```

---

## Implementation Strategy

### MVP First (User Story 4 — Seamless Migration)

1. Complete Phase 1: Setup (Cargo.toml dependency swap)
2. Complete Phase 2: Foundational (scaffold + tests — red phase)
3. Complete Phase 3: US4 implementation (green phase)
4. **STOP and VALIDATE**: `cargo test` + `cargo clippy -- -D warnings` — all pass
5. Binary is now self-contained (no ODBC driver dependency)

### Incremental Delivery

1. Complete Setup + Foundational -> Tests written, scaffold ready
2. Implement pure functions (T007-T009) -> Unit tests pass -> Core logic verified
3. Implement config + describe + execute (T010-T012) -> Full backend works -> **MVP complete**
4. Verify all platforms (T013a-T013c) -> Cross-platform confidence
5. Polish (T014-T018) -> Clean, validated release with verified non-functional requirements

---

## Notes

- [P] tasks = independent functions or different files, no shared mutable state
- This is a single-file rewrite — most tasks modify `src/backend/sqlserver.rs`
- US1 (macOS auth), US2 (SQL login), US3 (Linux auth) are branches/features within the US4 (migration) implementation
- **Highest risk area**: `column_data_to_string` (T009) — value formatting must match ODBC output exactly (FR-007)
- DMV fallback (`normalize_tiberius_type`) should emit a diagnostic warning when used
- One commit per task per project convention
- TDD discipline: write tests first (Phase 2), then implement to make them pass (Phase 3)

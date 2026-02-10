# Tasks: Multi-Database Query CLI

**Input**: Design documents from `/specs/001-multi-db-query/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-interface.md, contracts/config-schema.toml, quickstart.md

**Tests**: Included. The plan explicitly specifies unit test files in the project structure and lists TDD as a passing constitution principle.

**Organization**: Tasks are grouped by user story (6 stories, P1-P6) to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- All file paths are relative to repository root

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Create the Rust project skeleton with all dependencies

- [ ] T001 Initialize Rust binary crate with `cargo init --name dbtoon` and configure Cargo.toml with all dependencies: `odbc-api = "20"`, `reqwest = { version = "0.12", features = ["json"] }`, `tokio = { version = "1", features = ["rt", "macros"] }`, `sqlparser = "0.61"`, `toon-format = { version = "0.4", default-features = false }`, `clap = { version = "4.5", features = ["derive"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `toml = "0.8"`, `directories = "6"`, `thiserror = "2"`, `anyhow = "1"`, `secrecy = "0.10"`, `dotenvy = "0.15"` — set edition = "2024" in Cargo.toml
- [ ] T002 Create source directory structure per plan.md: `src/backend/`, `tests/unit/`, `tests/integration/` — add placeholder `mod.rs` for `src/backend/` and any required test harness entry points

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, traits, and infrastructure that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [ ] T003 [P] Define error types using thiserror in src/error.rs — `DbtoonError` enum with variants: `Validation { reason: String }`, `Connection { message: String }`, `Query { message: String }`, `Timeout { seconds: u64 }`, `Config { message: String }`, `Auth { message: String }`, `Io(std::io::Error)`, `Format { message: String }` — each variant must produce stderr-friendly messages matching the error categories in contracts/cli-interface.md (validation, connection, query, timeout, config, auth)
- [ ] T004 [P] Implement credential masking helpers in src/masking.rs — utility functions for formatting values that may contain SecretString fields: when `show_secrets` is false, display `[REDACTED]`; when true, expose via `.expose_secret()` — used by verbose/diagnostic output and error messages per FR-017
- [ ] T005 [P] Define Backend trait and core result types in src/backend/mod.rs — `Backend` trait with async `execute(&self, sql: &str, limit: Option<usize>, timeout_secs: u64) -> Result<QueryResult, DbtoonError>` method; `QueryResult { columns: Vec<ColumnMeta>, rows: Vec<Vec<CellValue>>, total_rows: Option<usize>, truncated: bool }`; `ColumnMeta { name: String, type_name: String }`; `CellValue` enum with `Text(String)` and `Null` variants — re-export from backend module
- [ ] T006 Implement TOON formatting in src/format.rs — convert `QueryResult` to TOON string: build `Vec<serde_json::Value>` (array of objects mapping column names to cell values, with `CellValue::Null` as `serde_json::Value::Null` and `CellValue::Text` as `serde_json::Value::String`), then call `toon_format::encode_default()` — handle zero-row results (produces TOON with headers but no data rows per edge case spec)
- [ ] T007 [P] Define complete CLI structure in src/cli.rs — clap derive structs: root `Cli` with global flags (`--config`, `--verbose`, `--show-secrets`), subcommands enum (`ExecRead`, `ExecWrite`, `ListWarehouses`); `ExecRead` with all flags from contracts/cli-interface.md (positional SQL vs --file, --backend, --server, --database, --username, --password, --windows-auth, --host, --token, --warehouse, --catalog, --schema, --limit with default 500, --no-limit, --timeout with default 60, --output, --profile); `ExecWrite` with same flags plus allow_write from env/config; `ListWarehouses` with --host, --token, --profile
- [ ] T008 Implement config loading in src/config.rs — define `AppConfig`, `BackendConfig` (SqlServer/Databricks variants), `SqlServerAuth` (WindowsIntegrated/SqlLogin) types matching data-model.md; load TOML config file from platform path via `directories` crate (Linux: ~/.config/dbtoon/config.toml, macOS: ~/Library/Application Support/dbtoon/config.toml, Windows: %APPDATA%\dbtoon\config.toml) or --config override; parse `[defaults]` and `[profiles.*]` sections per config-schema.toml; resolve env vars (DBTOON_BACKEND, DBTOON_SERVER, DBTOON_PASSWORD, DBTOON_DATABRICKS_TOKEN, etc.); merge with precedence: CLI flags > env vars > config file > defaults; support `password_env` and `token_env` indirection in config profiles; use SecretString for password and token fields
- [ ] T009 Implement output routing in src/output.rs — `print_result(toon_string: &str)` writes to stdout; `print_error(err: &DbtoonError)` writes formatted `error: <category>: <message>` to stderr matching contracts/cli-interface.md error format; `print_summary(rows: usize, path: &Path, truncated: bool)` for file output summary
- [ ] T010 Implement main entry point in src/main.rs — `#[tokio::main] async fn main()`: parse CLI with clap, load `.env` via dotenvy (optional, ignore if missing), build AppConfig via config::load() with CLI args + env + file, match on subcommand and dispatch to handler functions (stubs returning `todo!()` for exec_read, exec_write, list_warehouses), catch all errors at top level and route through output::print_error with exit code 1

**Checkpoint**: Project compiles with `cargo build`. All modules exist with public type definitions. Main dispatches to subcommand stubs.

---

## Phase 3: User Story 1 — Execute Read-Only Query Against SQL Server (Priority: P1)

**Goal**: A user runs `dbtoon exec-read --backend sqlserver ...` with a SELECT query and receives TOON-formatted results on stdout. Write queries (INSERT, UPDATE, DELETE, SELECT INTO, EXEC, unparseable SQL) are rejected before execution.

**Independent Test**: Run a SELECT query against a SQL Server instance; verify TOON output. Run an INSERT query; verify rejection with clear error.

### Implementation for User Story 1

- [ ] T011 [US1] Implement read-only query validation in src/validation.rs — `pub fn validate(sql: &str, dialect: Dialect) -> Result<(), DbtoonError>` using sqlparser: parse with `Parser::parse_sql(dialect, sql)` → `Vec<Statement>`; parse failure = return `DbtoonError::Validation` (FR-009); iterate all statements (FR-010); allowlist: `Statement::Query` (recurse into body), `Statement::Explain*`, `Statement::ShowTables`/`ShowColumns`/`ShowVariable` and other Show variants, `Statement::Use`; for `Statement::Query` → recurse `query.body`: `SetExpr::Select` safe only if `select.into.is_none()` (SELECT INTO → deny), `SetExpr::Insert|Update|Delete|Merge` → deny (CTE-wrapped writes), `SetExpr::SetOperation` → recurse both sides; deny `Statement::Execute`/stored procedures; default `_ =>` deny unrecognized (FR-007); return first `DenialReason` with `DenialKind` and human-readable detail string
- [ ] T012 [US1] Implement SQL Server backend in src/backend/sqlserver.rs — `pub struct SqlServerBackend` implementing Backend trait; constructor takes `&BackendConfig::SqlServer`; build ODBC connection string (Windows Auth: `Trusted_Connection=yes`, SQL Auth: `UID=...;PWD=...` with `escape_attribute_value` for password); `Environment::new()` for ODBC env; `env.connect_with_connection_string(conn_str, ConnectionOptions { login_timeout_sec: Some(30) })`; `execute(sql, (), Some(timeout_secs))` → `Option<CursorImpl>`; if `None` return empty QueryResult (zero rows with no columns); extract column names via `cursor.num_result_cols()` + `cursor.describe_col()` (1-based indexing); create `ColumnarAnyBuffer` with `BufferDesc::from_data_type()` (fallback `BufferDesc::Text { max_str_len: 255 }` for unmapped types), batch size 5000; `cursor.bind_buffer(buffer)` → `RowSetCursor`; fetch batches, convert to `Vec<Vec<CellValue>>` (NULL → `CellValue::Null`, value → `CellValue::Text(string)`); handle `Error::Diagnostics { record, .. }` for SQLSTATE errors → map to `DbtoonError::Connection` or `DbtoonError::Auth`
- [ ] T013 [US1] Wire exec-read subcommand end-to-end in src/main.rs — implement `async fn exec_read(config: AppConfig, sql: String)`: resolve SQL input from positional `<SQL>` argument or `--file <PATH>` (read file contents); select dialect based on backend type (MsSqlDialect for SQL Server); call `validation::validate(&sql, dialect)?`; construct backend from config (`SqlServerBackend::new(&config.backend)`); call `backend.execute(&sql, limit, timeout).await?`; call `format::to_toon(&result)?`; route output via `output::print_result()` or `output::write_file()` based on config.output_file; return `Ok(())`

### Tests for User Story 1

- [ ] T014 [P] [US1] Write validation unit tests in tests/unit/validation_test.rs — table-driven tests covering all acceptance scenarios: SELECT allowed, SELECT with JOIN allowed, EXPLAIN allowed, DESCRIBE/SHOW/USE allowed, INSERT denied (DenialKind::WriteStatement), UPDATE denied, DELETE denied, DROP TABLE denied, SELECT INTO denied (DenialKind::SelectInto), WITH cte AS (...) INSERT denied (DenialKind::CteWrappedWrite), WITH cte AS (...) DELETE denied, EXEC/EXECUTE denied (DenialKind::StoredProcedure), unparseable SQL denied (DenialKind::ParseFailure), multi-statement batch where one is INSERT → entire batch denied, multi-statement all-SELECT → allowed
- [ ] T015 [P] [US1] Write TOON formatting unit tests in tests/unit/format_test.rs — test cases: 3-column 2-row result → verify TOON tabular format string, zero-row result → headers only (`[0]{col1,col2}:`), NULL cell value → `null` literal in TOON output, single-column single-row edge case, special characters in cell values
- [ ] T016 [P] [US1] Write config unit tests in tests/unit/config_test.rs — test cases: parse valid TOML profile, env var overrides config file value, CLI flag overrides env var, missing required field (no backend specified) → clear Config error, profile not found → clear error, password_env indirection resolves from env, default values (row_limit=500, timeout=60, allow_write=false)
- [ ] T017 [P] [US1] Write masking unit tests in tests/unit/masking_test.rs — test cases: SecretString in Debug output shows `[REDACTED]`, show_secrets=false masks password/token in formatted output, show_secrets=true exposes actual values

**Checkpoint**: `cargo test` passes for unit tests. Running `dbtoon exec-read --backend sqlserver --server localhost --database testdb --windows-auth "SELECT 1"` against a live SQL Server returns TOON output. Running with an INSERT statement returns a validation error to stderr with exit code 1.

---

## Phase 4: User Story 2 — Execute Read-Only Query Against Databricks (Priority: P2)

**Goal**: A user runs `dbtoon exec-read --backend databricks ...` with a SELECT query and receives TOON-formatted results. Databricks-specific writes (OPTIMIZE, VACUUM, MERGE) are rejected. Auth errors produce clear messages.

**Independent Test**: Run a SELECT query against a Databricks SQL warehouse; verify TOON output. Submit a query with an expired token; verify clear auth error.

### Implementation for User Story 2

- [ ] T018 [US2] Implement Databricks backend in src/backend/databricks.rs — `pub struct DatabricksBackend` implementing Backend trait; constructor takes `&BackendConfig::Databricks`; build reqwest client with `Authorization: Bearer <token>` header; `execute()`: POST to `https://{host}/api/2.0/sql/statements/` with JSON body `{ warehouse_id, statement: sql, wait_timeout: "50s", on_wait_timeout: "CONTINUE", row_limit: limit }` (omit row_limit if None); parse response: if `status.state == "SUCCEEDED"` → extract `manifest.schema.columns` for ColumnMeta (name + type_name) and `result.data_array` for rows (`Option<String>` → CellValue); if `PENDING`/`RUNNING` → poll `GET /api/2.0/sql/statements/{statement_id}` every 2 seconds; if own timeout_secs exceeded → POST `../cancel` and return `DbtoonError::Timeout`; if `FAILED` → return `DbtoonError::Query` with `status.error.message`; handle HTTP errors: 401 → `DbtoonError::Auth("invalid or expired token")`, 403 → `DbtoonError::Auth("insufficient warehouse permissions")`, 404 → `DbtoonError::Config("warehouse not found")`; set `QueryResult.truncated = manifest.truncated`
- [ ] T019 [US2] Wire Databricks backend into exec-read dispatch in src/main.rs — update `exec_read()` to match on backend type: `BackendConfig::SqlServer` → `SqlServerBackend`, `BackendConfig::Databricks` → `DatabricksBackend`; both flow through the same validate → execute → format → output pipeline; use `DatabricksDialect` for sqlparser validation when backend is Databricks

**Checkpoint**: `dbtoon exec-read --backend databricks --host <host> --warehouse <id> --token <token> "SELECT 1"` returns TOON output. OPTIMIZE/VACUUM queries are rejected by validation.

---

## Phase 5: User Story 3 — Execute Write Query (Priority: P3)

**Goal**: A user with `DBTOON_ALLOW_WRITE=true` runs `dbtoon exec-write` to execute a state-modifying query. Without the env/config flag, the command is denied.

**Independent Test**: Run INSERT with allow_write=true; verify execution. Run INSERT with allow_write=false; verify denial.

### Implementation for User Story 3

- [ ] T020 [US3] Implement exec-write execution path in src/main.rs — implement `async fn exec_write(config: AppConfig, sql: String)`: check `config.allow_write`; if false → return `DbtoonError::Auth("write access denied — set DBTOON_ALLOW_WRITE=true to enable")` (FR-004: both the flag AND exec-write command are required); if true → skip validation entirely, construct backend from config, call `backend.execute(&sql, limit, timeout).await?`, format and output result; both SQL Server and Databricks backends are supported

**Checkpoint**: `DBTOON_ALLOW_WRITE=true dbtoon exec-write --backend sqlserver ... "INSERT INTO t VALUES (1)"` executes and returns result. Without the env var, returns auth error.

---

## Phase 6: User Story 4 — Row Limits and Result Truncation (Priority: P4)

**Goal**: Large result sets are truncated to 500 rows by default with a clear message. Users can override with `--no-limit`.

**Independent Test**: Query a 10,000-row table with default settings; verify 500 rows returned with truncation message. Re-run with --no-limit; verify all rows.

### Implementation for User Story 4

- [ ] T021 [P] [US4] Add row limit enforcement to SQL Server backend in src/backend/sqlserver.rs — during ColumnarAnyBuffer fetch loop, stop accumulating rows after `limit` rows reached; set `QueryResult.truncated = true` when more rows were available; pass `total_rows = None` (ODBC doesn't provide total count before fetching all)
- [ ] T022 [P] [US4] Add row_limit parameter to Databricks API request in src/backend/databricks.rs — already included in T018 POST body; ensure when `--no-limit` is set, omit `row_limit` from request body; when limit is applied, read `manifest.truncated` flag and propagate to `QueryResult.truncated`
- [ ] T023 [US4] Implement truncation output formatting in src/output.rs — when `QueryResult.truncated` is true, append truncation metadata after TOON table: `truncated: true` and `message: Showing {N} rows. Use --no-limit to return all rows.` per contracts/cli-interface.md output contract; when not truncated, omit truncation metadata; handle `--no-limit` CLI flag: pass `limit = None` to backend

**Checkpoint**: `dbtoon exec-read ... --limit 10 "SELECT * FROM large_table"` returns 10 rows with truncation message. `--no-limit` returns all rows without message. Default limit is 500.

---

## Phase 7: User Story 5 — Write Results to File (Priority: P5)

**Goal**: `dbtoon exec-read --output results.toon ...` writes TOON to a file and prints a summary to stdout.

**Independent Test**: Run query with --output; verify file contains TOON data and stdout shows summary.

### Implementation for User Story 5

- [ ] T024 [US5] Implement file output routing in src/output.rs — when `config.output_file` is `Some(path)`: validate parent directory exists (return `DbtoonError::Io` if not); write TOON string to file; print summary to stdout in TOON format: `rows_written: {N}`, `file: {path}`, `truncated: {bool}` per contracts/cli-interface.md file output contract; update `exec_read` and `exec_write` in main.rs to check output_file and route accordingly

**Checkpoint**: `dbtoon exec-read --output /tmp/test.toon ... "SELECT 1"` creates file with TOON content and prints summary to stdout. Invalid path returns IO error.

---

## Phase 8: User Story 6 — Discover Databricks SQL Warehouses (Priority: P6)

**Goal**: `dbtoon list-warehouses` displays available Databricks SQL warehouses with their identifiers.

**Independent Test**: Run list-warehouses with valid credentials; verify warehouse list in TOON format.

### Implementation for User Story 6

- [ ] T025 [P] [US6] Implement warehouse listing API in src/backend/databricks.rs — `pub async fn list_warehouses(host: &str, token: &SecretString) -> Result<Vec<WarehouseInfo>, DbtoonError>`: GET `https://{host}/api/2.0/sql/warehouses/` with bearer token; parse `{ warehouses: [...] }` response; extract `WarehouseInfo { id, name, state, cluster_size, warehouse_type }` per data-model.md; handle HTTP 401 → `DbtoonError::Auth`, other errors → `DbtoonError::Connection`
- [ ] T026 [US6] Wire list-warehouses subcommand in src/main.rs — implement `async fn list_warehouses(config: AppConfig)`: resolve Databricks host and token from config; call `databricks::list_warehouses(host, token).await?`; convert `Vec<WarehouseInfo>` to `QueryResult` (columns: id, name, state, cluster_size, type; rows from warehouse fields); format as TOON via `format::to_toon()`; output to stdout

**Checkpoint**: `dbtoon list-warehouses --host <host> --token <token>` displays TOON-formatted warehouse table. Invalid token returns auth error.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Diagnostics, edge case hardening, and validation

- [ ] T027 [P] Add verbose diagnostic output to all execution paths — when `config.verbose` is true, emit timestamped `[dbtoon] ...` messages to stderr: connecting to backend (host/server), connection established (duration), validating query (mode, statement count), validation result (pass/fail, duration), executing query, query complete (duration, row count), formatting output, writing file — per contracts/cli-interface.md verbose diagnostics format
- [ ] T028 [P] Harden edge case handling across all modules — connection drop mid-query: backends must return `DbtoonError::Connection` not partial results; no backend/connection config: config.rs returns `DbtoonError::Config("no backend specified")`; query timeout: ensure both backends respect timeout_secs and return `DbtoonError::Timeout`; Databricks warehouse stopped/starting: emit `[dbtoon] warehouse starting, waiting...` on stderr if verbose; Windows Auth from non-domain machine: passthrough ODBC driver error as `DbtoonError::Auth`; multi-statement batch edge case already handled by validation
- [ ] T029 Validate against quickstart.md scenarios — manually run each command from quickstart.md (SQL Server Windows Auth, SQL Server SQL Auth, Databricks query, config profiles, warehouse discovery, file output, write access, verbose mode) and verify output matches documented behavior; fix any discrepancies

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Depends on Phase 1 — BLOCKS all user stories
- **Phase 3 (US1)**: Depends on Phase 2 completion
- **Phase 4 (US2)**: Depends on Phase 2 completion (can run parallel with US1 if desired, but sequential is recommended since US1 establishes the exec-read pipeline)
- **Phase 5 (US3)**: Depends on Phase 3 or Phase 4 (needs at least one working backend)
- **Phase 6 (US4)**: Depends on Phase 3 or Phase 4 (needs backend execute working)
- **Phase 7 (US5)**: Depends on Phase 3 or Phase 4 (needs format + output working)
- **Phase 8 (US6)**: Depends on Phase 4 (needs Databricks backend)
- **Phase 9 (Polish)**: Depends on all desired user stories being complete

### Recommended Execution Order (Single Developer)

```
Phase 1 → Phase 2 → Phase 3 (US1 MVP) → Phase 4 (US2) → Phase 5 (US3) → Phase 6 (US4) → Phase 7 (US5) → Phase 8 (US6) → Phase 9
```

### User Story Dependencies

- **US1 (P1)**: After Phase 2 — no other story dependencies
- **US2 (P2)**: After Phase 2 — independent of US1 (same pipeline, different backend) but benefits from US1 having established the exec-read wiring
- **US3 (P3)**: After US1 or US2 — needs at least one backend working
- **US4 (P4)**: After US1 or US2 — needs backend execute working to test truncation
- **US5 (P5)**: After US1 or US2 — needs format + output pipeline working
- **US6 (P6)**: After US2 — depends on Databricks backend module existing

### Within Each User Story

- Type definitions before implementations
- Backend implementation before main.rs wiring
- Implementation before tests (tests validate the implementation)

### Parallel Opportunities

**Phase 2** (within phase):
```
Parallel: T003 (error.rs) | T004 (masking.rs) | T005 (backend/mod.rs) | T007 (cli.rs)
Then: T006 (format.rs, needs T005) | T008 (config.rs, needs T003+T004) | T009 (output.rs)
Then: T010 (main.rs, needs T007+T008+T009)
```

**Phase 3** (within phase):
```
Parallel: T011 (validation.rs) | T012 (sqlserver.rs)
Then: T013 (main.rs wiring, needs T011+T012)
Parallel: T014 | T015 | T016 | T017 (all test files, independent)
```

**Phase 6** (within phase):
```
Parallel: T021 (sqlserver limit) | T022 (databricks limit)
Then: T023 (truncation output)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (2 tasks)
2. Complete Phase 2: Foundational (8 tasks)
3. Complete Phase 3: User Story 1 (7 tasks)
4. **STOP AND VALIDATE**: `cargo test` passes, exec-read works against SQL Server
5. Total MVP: 17 tasks

### Incremental Delivery

1. Setup + Foundational → project compiles
2. + US1 → SQL Server read queries work (MVP)
3. + US2 → Databricks read queries work (multi-database)
4. + US3 → Write queries work (full CRUD)
5. + US4 → Row limits protect context windows (agent-safe)
6. + US5 → File output enables large dataset workflows
7. + US6 → Warehouse discovery aids onboarding
8. + Polish → Production-ready diagnostics and edge case handling

---

## Summary

| Metric | Value |
|--------|-------|
| Total tasks | 29 |
| Phase 1 (Setup) | 2 |
| Phase 2 (Foundational) | 8 |
| US1 (SQL Server read) | 7 (incl. 4 test tasks) |
| US2 (Databricks read) | 2 |
| US3 (Write queries) | 1 |
| US4 (Row limits) | 3 |
| US5 (File output) | 1 |
| US6 (Warehouse discovery) | 2 |
| Polish | 3 |
| MVP scope | 17 tasks (Phases 1-3) |
| Parallel opportunities | 3 phases with internal parallelism |

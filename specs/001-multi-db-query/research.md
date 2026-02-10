# Research: Multi-Database Query CLI

**Feature Branch**: `001-multi-db-query` | **Date**: 2026-02-10

## 1. TOON Format & toon-rust Crate

**Decision**: Use `toon-format` v0.4 (library only, no `cli` feature) for TOON output serialization.

**Rationale**: The crate provides a serde-based `encode` API that automatically detects tabular arrays (arrays of uniform, primitive-valued objects) and emits the compact columnar TOON format. SQL result sets are inherently flat/primitive, so tabular detection will always trigger. No low-level row-writing API is needed — we build a `Vec<serde_json::Value>` (array of objects from column names + row data) and call `encode_default()`.

**Alternatives considered**:
- Hand-rolling TOON output: Rejected. The format has quoting/escaping rules, delimiter handling, and array-length declarations that the crate handles correctly. Reimplementing would duplicate effort and risk spec drift.

**Integration pattern**:
```
Column names + row vectors → Vec<serde_json::Map> → toon_format::encode_default() → String
```

**TOON tabular output example** (for a 3-row SQL result):
```
[3]{id,name,email}:
  1,Alice,alice@co.com
  2,Bob,bob@co.com
  3,Carol,carol@co.com
```

**Key detail**: All values in `data_array` must be primitives (string, number, boolean, null) for tabular format. SQL results satisfy this inherently. NULL renders as literal `null`.

**Dependency**: `toon-format = { version = "0.4", default-features = false }`

---

## 2. CLI Framework

**Decision**: `clap` v4.5 with derive macros.

**Rationale**: De facto standard for Rust CLIs. Derive API makes subcommand definitions declarative. Supports global flags, auto-generated help, shell completions. No serious competitor in the ecosystem.

**Alternatives considered**:
- `argh` (Google): Fraction of the ecosystem support and features. No benefit over clap.

**Dependency**: `clap = { version = "4.5", features = ["derive"] }`

---

## 3. Configuration

**Decision**: TOML config file via `toml` v0.8 + `serde`. Platform-appropriate paths via `directories` v6.

**Rationale**: TOML is Rust-idiomatic (Cargo.toml itself), maps cleanly to connection profiles, avoids YAML's type ambiguity. The `directories` crate resolves XDG-compliant paths cross-platform.

**Config locations**:
- Linux: `~/.config/dbtoon/config.toml`
- macOS: `~/Library/Application Support/dbtoon/config.toml`
- Windows: `%APPDATA%\dbtoon\config.toml`

**Precedence** (per FR-015): CLI flags > environment variables > config file.

**Alternatives considered**:
- YAML (`serde_yaml`): Type ambiguity issues (e.g., `no` parsed as boolean). Less idiomatic in Rust.
- JSON: No comments, verbose. Poor for human-edited config.

**Dependencies**: `toml = "0.8"`, `serde = { version = "1", features = ["derive"] }`, `directories = "6"`

---

## 4. Error Handling

**Decision**: `thiserror` v2 for domain error enums + `anyhow` v1 for CLI glue.

**Rationale**: `thiserror` gives structured error types with clean `Display` implementations for user-facing messages. `anyhow` provides ergonomic `?` propagation with `.context()` in the CLI entrypoint. This is the standard Rust pattern — not either/or.

**Alternatives considered**:
- `color-eyre`: Adds colorized backtraces and span traces. Heavier, more suited to developer tools than end-user CLIs. Unnecessary for this tool.
- `anyhow` alone: Loses the ability to match on specific error variants for producing targeted error messages.

**Dependencies**: `thiserror = "2"`, `anyhow = "1"`

---

## 5. Credential Masking

**Decision**: `secrecy` v0.10 for secret value types.

**Rationale**: `SecretString` wraps sensitive values with `Debug`/`Display` that emit `[REDACTED]`, preventing accidental logging. Forces explicit `.expose_secret()` for access. Zeroizes memory on drop. This prevents leaks at the type level rather than scrubbing after the fact (FR-017).

**Override mechanism**: When `--show-secrets` flag or `DBTOON_SHOW_SECRETS=true` is set, use `.expose_secret()` in verbose output. Otherwise, secrets never appear in any output.

**Alternatives considered**:
- Custom wrapper type: More code, no zeroization, no ecosystem recognition.

**Dependency**: `secrecy = "0.10"`

---

## 6. SQL Server Backend (odbc-api)

**Decision**: `odbc-api` v20+ with columnar bulk fetch. Pre-decided in PLAN_INPUT.md.

**Rationale**: See PLAN_INPUT.md for full justification (Tiberius Kerberos issues, sync advantage for CLI).

**Key integration details**:
- `Environment::new()` → singleton ODBC 3.8 environment
- `env.connect_with_connection_string(conn_str, options)` → connection
- `conn.execute(sql, (), Some(timeout_secs))` → `Option<CursorImpl>`
- `cursor.column_names()` → `Vec<String>` for TOON column headers
- `ColumnarAnyBuffer` with `BufferDesc::from_data_type()` for bulk fetch (5000 rows/batch)
- Fall back to `BufferDesc::Text { max_str_len: 255 }` for unmapped types
- Column indices: 1-based for `get_data()`/`describe_col()`, 0-based for `batch.column()`
- Query timeout: 3rd argument to `execute()` (`Some(seconds)`)
- Login timeout: `ConnectionOptions { login_timeout_sec: Some(30) }`
- Use `escape_attribute_value()` for passwords in connection strings

**Connection strings**:
- Windows Auth: `Driver={ODBC Driver 18 for SQL Server};Server=...;Database=...;Trusted_Connection=yes;`
- SQL Auth: `Driver={ODBC Driver 18 for SQL Server};Server=...;Database=...;UID=...;PWD=...;`

**Error handling**: Match on `Error::Diagnostics { record, function }` for SQLSTATE-level errors. `record.state`, `record.message`, `record.native_error` provide diagnostic detail.

**Dependency**: `odbc-api = "20"`

---

## 7. Databricks Backend (REST API)

**Decision**: SQL Statement Execution REST API (`POST /api/2.0/sql/statements/`). Pre-decided in PLAN_INPUT.md.

**Rationale**: See PLAN_INPUT.md for full justification (MCP endpoint is wrong API).

**Key integration details**:

**Execution flow**:
1. POST with `warehouse_id`, `statement`, `wait_timeout: "50s"`, `on_wait_timeout: "CONTINUE"`, `row_limit`
2. If response `status.state` is `SUCCEEDED` → extract `manifest` + `result`
3. If `PENDING`/`RUNNING` → poll `GET /api/2.0/sql/statements/{id}` at 1-2s intervals
4. If own 60s timeout exceeded → `POST .../cancel` and report timeout
5. `result.data_array` is `Vec<Vec<Option<String>>>` — all values are strings, use `manifest.schema.columns[].type_name` for type interpretation

**Row limit**: Send `row_limit: 500` in request body. When `manifest.truncated` is `true`, emit truncation message. API does not report total available rows — only that truncation occurred.

**Warehouse listing**: `GET /api/2.0/sql/warehouses/` returns `{ warehouses: [...] }`. Key fields to display: `id`, `name`, `state`, `cluster_size`, `warehouse_type`.

**Error handling**:
- HTTP 401: `UNAUTHENTICATED` — clear auth error
- HTTP 403: `PERMISSION_DENIED` — insufficient warehouse access
- HTTP 404: `NOT_FOUND` — warehouse doesn't exist
- Statement-level: `status.state == "FAILED"` with `status.error.error_code` and `status.error.message`
- Warehouse stopped: Query sits in `PENDING` while auto-starting (1-5+ min). Emit "Warehouse starting..." on stderr if `--verbose`.

**Gotchas**:
- `wait_timeout` max is 50s, not configurable beyond that
- 25 MiB hard limit for INLINE disposition — handle `RESOURCE_EXHAUSTED` gracefully
- Last chunk fetch closes the statement permanently
- Same response struct for execute, get-statement, and get-chunk

**Dependencies**: `reqwest = { version = "0.12", features = ["json"] }`, `tokio = { version = "1", features = ["rt", "macros"] }`

---

## 8. Read-Only Query Validation (sqlparser)

**Decision**: `sqlparser` v0.61+ with allowlist approach. Pre-decided in PLAN_INPUT.md.

**Rationale**: First-class `MsSqlDialect` and `DatabricksDialect` support. Allowlist on `Statement` enum with catch-all deny is the safest approach.

**Validation algorithm** (from PLAN_INPUT.md):
1. `Parser::parse_sql(dialect, sql)` → `Vec<Statement>`
2. All statements must pass (FR-010)
3. Parse failures = rejection (FR-009)
4. Allow: `Statement::Query` (with recursion), `Explain`, `ExplainTable`, `Show*`, `Use`
5. `Statement::Query` → recurse into `Query.body` (`SetExpr`):
   - `SetExpr::Select`: safe only if `Select.into.is_none()` (denies SELECT INTO)
   - `SetExpr::Insert|Update|Delete|Merge`: CTE-wrapped writes → deny
   - `SetExpr::SetOperation`: recurse both sides
6. `EXEC`/`EXECUTE`: always denied
7. Everything else (`_ => false`): denied

**Dependency**: `sqlparser = "0.61"`

---

## 9. Environment Variable Loading

**Decision**: `dotenvy` v0.15 for `.env` file support, `std::env::var()` for direct reads.

**Rationale**: `dotenvy` is the maintained fork of the abandoned `dotenv` crate. For a CLI tool with a TOML config, `.env` is optional convenience — the primary env var interface is direct `std::env::var()`.

**Dependency**: `dotenvy = "0.15"`

---

## Complete Dependency Table

| Crate | Version | Purpose |
|-------|---------|---------|
| `odbc-api` | `20` | SQL Server backend (ODBC) |
| `reqwest` | `0.12` (features: `json`) | Databricks REST API |
| `tokio` | `1` (features: `rt`, `macros`) | Async runtime for reqwest |
| `sqlparser` | `0.61` | Read-only query validation |
| `toon-format` | `0.4` (default-features: false) | TOON output serialization |
| `clap` | `4.5` (features: `derive`) | CLI framework |
| `serde` | `1` (features: `derive`) | Serialization/deserialization |
| `serde_json` | `1` | JSON handling (TOON bridge, Databricks API) |
| `toml` | `0.8` | Config file parsing |
| `directories` | `6` | Platform-appropriate config paths |
| `thiserror` | `2` | Domain error types |
| `anyhow` | `1` | CLI error propagation |
| `secrecy` | `0.10` | Credential masking |
| `dotenvy` | `0.15` | Optional .env file loading |

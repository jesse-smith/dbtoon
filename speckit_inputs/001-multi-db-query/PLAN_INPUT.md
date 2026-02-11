Pre-decided implementation details for the dbtoon planning phase.

## Language

Rust. Avoids requiring a Python runtime for CLI distribution.

## SQL Server Backend

- **Crate**: `odbc-api` (v20+) — safe Rust wrapper around the system ODBC driver.
- Auth (including Kerberos/Windows Integrated Auth) delegated to Microsoft's ODBC Driver for SQL Server via connection string (`Trusted_Connection=yes`). Same code path as pyodbc/SQLAlchemy.
- Synchronous execution. ODBC calls can be wrapped in `spawn_blocking` if needed for async contexts.
- Requires the ODBC driver installed at runtime (guaranteed available in target environments).
- **Why not Tiberius**: Pure-Rust async TDS client, but its Kerberos support on Unix/macOS depends on `libgssapi` with known issues (memory safety panics, connection hangs, build failures requiring system Kerberos headers). Its async advantage is irrelevant for a CLI tool.

## Databricks Backend

- **API**: SQL Statement Execution REST API (`POST /api/2.0/sql/statements/`), not the MCP endpoint (`/api/2.0/mcp/sql`). The MCP endpoint is a JSON-RPC tool-discovery protocol, not a direct SQL execution API.
- **Crates**: `reqwest` + `tokio`.
- Auth: Bearer token from `DATABRICKS_TOKEN` environment variable.
- Requires a `warehouse_id` (specific SQL compute resource, obtainable from UI or `GET /api/2.0/sql/warehouses/`).
- Results: JSON with schema metadata in `manifest` + data in `result.data_array`. Inline up to 25 MiB; presigned URLs via `EXTERNAL_LINKS` disposition for larger results.
- Supports `row_limit`, `CSV`/`ARROW_STREAM` formats, and parameterized queries.

## Read-Only Query Validation

- **Crate**: `sqlparser` (v0.61+, Apache DataFusion project). First-class `MsSqlDialect` and `DatabricksDialect` support.
- **Approach**: Allowlist on the `Statement` enum with catch-all deny (`_ => false`).
  - Allow: `Statement::Query`, `Explain`, `ExplainTable`, all `Show*` variants, `Use`.
  - `Statement::Query` requires recursion into `Query.body` (`SetExpr` enum):
    - `SetExpr::Select` safe only if `Select.into.is_none()` (denies `SELECT INTO`).
    - `SetExpr::Insert`, `SetExpr::Update`, `SetExpr::Delete`, `SetExpr::Merge` are CTE-wrapped writes — deny.
    - `SetExpr::SetOperation` (UNION/EXCEPT/INTERSECT) must check both sides recursively.
  - `EXEC`/`EXECUTE` always denied (opaque stored procs).
  - `Parser::parse_sql()` returns `Vec<Statement>` — all must pass.
  - Parse failures = rejection in read mode.
- Known limitations:
  - sqlparser doesn't cover 100% of T-SQL grammar — unusual constructs may fail to parse and be rejected (safe direction).
  - `OPENROWSET` parses as a table function inside `FROM`; safe when inside a plain SELECT (no INTO). `BULK INSERT` and `xp_cmdshell` either fail to parse (rejected) or go through `EXEC` (denied).

## Security Backstop Recommendations

- SQL Server: `db_datareader` + `db_denydatawriter` roles on read connection credentials.
- Databricks: Service principal with only `SELECT` + `USE CATALOG/SCHEMA` via Unity Catalog.
- `ApplicationIntent=ReadOnly` in the ODBC connection string is only an AlwaysOn routing hint, NOT a security enforcement mechanism — do not rely on it for write prevention.

## Dependency Summary

| Crate | Purpose | Notes |
|-------|---------|-------|
| `odbc-api` (v20+) | SQL Server backend | Synchronous, requires ODBC driver at runtime |
| `reqwest` | Databricks REST API | Async HTTP client |
| `tokio` | Async runtime | Required by reqwest |
| `sqlparser` (v0.61+) | Read-only query validation | No special features needed |

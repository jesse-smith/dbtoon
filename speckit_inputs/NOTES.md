I want to build or find a tool that allows agents and humans to easily query multiple databases.

## Requirements
### Must Have
- MUST include support for AT LEAST SQL Server (incl. Windows auth) and databricks (incl. DATABRICKS_TOKEN token auth)
- Token efficiency: don't return tables as JSON (use TOON, CSV, etc)
- Separate read-only vs read/write tooling; want to disallow agent writes by default
- read-only MUST allow describe table, retrieving query plan, and other non-SELECT queries that don't involve writing/modifying state

### Nice to Have
- Default row limits/truncation/etc. when returning massive datasets, but should be able to turn off
- Write results to file/disk to allow searching without loading full result into context

### Do not need
- Separate tools for examining schema, structure, stats, etc; LLM can construct these queries easily

## Build Decisions
No existing tool covers my use cases - see RESEARCH.md for details.

Since building, adding additional constraints:

- CLI tool: this allows easy execution by both user and AI
- TOON output (efficient table + metadata format, can handle query plan output, etc.)
- Databricks via SQL Statement Execution REST API (POST /api/2.0/sql/statements/); convert JSON results to TOON format
  - NOTE: originally considered native MCP endpoint (/api/2.0/mcp/sql), but the MCP layer adds indirection without benefit here — it's an MCP tool-discovery protocol, not a direct SQL execution API. The REST endpoint gives full control over query text and result format.
- Two tools - exec_read

## Open Questions

- Have previously successfully used SQL Alchemy for SQL Server access. Considering Rust-based implementation for CLI tool to avoid need for python runtime; is there an SQL Alchemy equivalent/implementation there?
    - A: No full SQLAlchemy equivalent exists in Rust (SQLx dropped MSSQL in v0.7, Diesel/SeaORM don't support it). But for a CLI tool, **odbc-api** (v20+, actively maintained) is the right choice — it's a safe Rust wrapper around the system ODBC driver, delegating auth, TLS, and type marshaling to Microsoft's ODBC Driver for SQL Server. Authentication (including Kerberos/Windows Integrated Auth) uses the exact same code path as pyodbc/SQLAlchemy — just `Trusted_Connection=yes` in the connection string. This avoids the Kerberos fragility seen in pure-Rust TDS implementations. It's raw SQL execution, not an ORM, but that's all a CLI tool needs. Requires the ODBC driver at runtime (guaranteed available in our target environments). Alternative: **Tiberius** (v0.12.3, Prisma) is a pure-Rust async TDS client, but its Kerberos support on Unix/macOS depends on `libgssapi` which has known issues (memory safety panics, connection hangs, build failures requiring system Kerberos headers). Its async advantage is irrelevant for a CLI tool.
- What's the best way to wrap the databricks SQL MCP? Can this be done in Rust, or is there a signficant tooling gap?
    - A: Don't wrap the MCP endpoint at all. The `/api/2.0/mcp/sql` endpoint is an MCP protocol server (JSON-RPC tool discovery), not a SQL execution API — wrong abstraction for direct query execution. Instead, use the **SQL Statement Execution REST API** (`POST /api/2.0/sql/statements/`) directly via `reqwest`. Auth is identical (Bearer token from DATABRICKS_TOKEN). Requires a `warehouse_id` (specific SQL compute resource, obtained from UI or `GET /api/2.0/sql/warehouses/`). Results come back as JSON with schema metadata in `manifest` + data in `result.data_array` (inline up to 25 MiB, or presigned URLs via EXTERNAL_LINKS disposition for larger results). Supports `row_limit`, `CSV`/`ARROW_STREAM` formats, and parameterized queries. Since the Databricks backend already needs reqwest+tokio, and odbc-api is synchronous (with ODBC calls wrapped in `spawn_blocking` if needed), the two backends share minimal dependency overlap but the total footprint stays reasonable. A `list-warehouses` subcommand would help users find their warehouse_id during setup.
- How should `exec_read` validate that a query is actually read-only before executing it? Need to handle non-obvious cases like `SELECT INTO`, `WITH ... INSERT`, `EXEC` (opaque stored procs), and Databricks-specific write operations (`OPTIMIZE`, `VACUUM`, `MERGE`, `COPY INTO`).
    - A: Use **sqlparser-rs** (`sqlparser` crate, v0.61+, Apache DataFusion project) as the primary validation layer. It has first-class `MsSqlDialect` and `DatabricksDialect` support. The approach is **allowlist on the `Statement` enum with a catch-all deny**:
      - **Allow**: `Statement::Query` (with additional checks — see below), `Explain`, `ExplainTable`, all `Show*` variants, `Use`.
      - **Deny everything else by default** (`_ => false`). This catches all DDL, DML writes, `EXEC`/`CALL`, `SET`, `Grant`/`Revoke`, `OptimizeTable`, `Vacuum`, `Merge`, `Copy`, etc. — and any future variants added to the enum.
      - **`Statement::Query` is not automatically safe** — must recurse into `Query.body` (a `SetExpr` enum). `SetExpr::Select` is safe only if `Select.into.is_none()` (denies `SELECT INTO`). `SetExpr::Insert`, `SetExpr::Update`, `SetExpr::Delete`, `SetExpr::Merge` are writes wrapped in CTEs — deny. `SetExpr::SetOperation` (UNION/EXCEPT/INTERSECT) must check both sides recursively.
      - **Parse failures = rejection** in read mode. If sqlparser can't parse the SQL, we can't verify it's safe. Users can fall back to `exec_write` with appropriate credentials for unusual syntax.
      - **Multi-statement batches**: `Parser::parse_sql()` returns `Vec<Statement>` — all must pass.
      - **EXEC/EXECUTE is always denied** in read mode. Stored procs and dynamic SQL are opaque; there is no way to determine safety from the AST. This is the biggest practical limitation (many SQL Server workflows use procs for reads), but allowing it would defeat the safety mechanism entirely.
    - Known limitations and mitigations:
      - sqlparser doesn't cover 100% of T-SQL grammar — some valid but unusual constructs may fail to parse and be rejected. This is the safe direction to fail.
      - `OPENROWSET` parses as a table function inside `FROM`; it's safe when inside a plain SELECT (no INTO) since `INSERT INTO t SELECT FROM OPENROWSET(...)` would be caught as `Statement::Insert`. `BULK INSERT` and `xp_cmdshell` either fail to parse (rejected) or go through `EXEC` (denied).
      - **Database-level permissions are the recommended backstop**: for SQL Server, use `db_datareader` + `db_denydatawriter` roles on the read connection credentials; for Databricks, use a service principal with only `SELECT` + `USE CATALOG/SCHEMA` via Unity Catalog. Note that the TDS `readonly` application intent (settable via `ApplicationIntent=ReadOnly` in the ODBC connection string) is only an AlwaysOn routing hint, NOT a security enforcement mechanism — it does not prevent writes.
    - Dependency: `sqlparser = "0.61"` — no special features needed. Already shares the async runtime (tokio) with reqwest (used for Databricks). odbc-api is synchronous and doesn't need tokio.

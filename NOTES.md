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
    - A: No full SQLAlchemy equivalent exists in Rust (SQLx dropped MSSQL in v0.7, Diesel/SeaORM don't support it). But for a CLI tool, **Tiberius** (v0.12.3, maintained by Prisma) is the right choice — it's a pure-Rust async TDS client with Windows Auth support (SSPI on Windows via `winauth` feature, Kerberos on Linux/macOS via `integrated-auth-gssapi`). It's raw SQL execution, not an ORM, but that's all a CLI tool needs. Alternative: **odbc-api** (v20+) if you want to delegate auth to Microsoft's ODBC driver, but adds a system dependency.
- What's the best way to wrap the databricks SQL MCP? Can this be done in Rust, or is there a signficant tooling gap?
    - A: Don't wrap the MCP endpoint at all. The `/api/2.0/mcp/sql` endpoint is an MCP protocol server (JSON-RPC tool discovery), not a SQL execution API — wrong abstraction for direct query execution. Instead, use the **SQL Statement Execution REST API** (`POST /api/2.0/sql/statements/`) directly via `reqwest`. Auth is identical (Bearer token from DATABRICKS_TOKEN). Requires a `warehouse_id` (specific SQL compute resource, obtained from UI or `GET /api/2.0/sql/warehouses/`). Results come back as JSON with schema metadata in `manifest` + data in `result.data_array` (inline up to 25 MiB, or presigned URLs via EXTERNAL_LINKS disposition for larger results). Supports `row_limit`, `CSV`/`ARROW_STREAM` formats, and parameterized queries. Since Tiberius already needs reqwest+tokio, Databricks adds zero new dependencies. A `list-warehouses` subcommand would help users find their warehouse_id during setup.

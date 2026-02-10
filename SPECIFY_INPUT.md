A CLI tool called `dbtoon` for querying multiple databases with output in TOON format (an efficient table + metadata format). Designed for use by both humans and AI agents.

## Backends

Two database backends:

1. **SQL Server** — supports Windows Integrated Auth and standard SQL auth.
2. **Databricks** — supports token-based auth (`DATABRICKS_TOKEN`). Requires specifying a SQL warehouse as the compute target.

## Commands

Two commands enforcing read/write separation:

- **`exec_read`**: Executes read-only queries. Validates queries before execution to ensure they cannot modify state. Validation rules:
  - Standard read operations allowed: SELECT, EXPLAIN, DESCRIBE, SHOW, USE.
  - Writes denied by default (INSERT, UPDATE, DELETE, DDL, etc.).
  - Subtle write-disguised-as-read patterns denied: `SELECT INTO`, CTE-wrapped writes.
  - Stored procedure execution (`EXEC`/`EXECUTE`) always denied — proc bodies are opaque and cannot be verified as read-only.
  - Parse failures are rejected (fail safe).
  - Multi-statement batches: every statement must individually pass validation.
- **`exec_write`**: Executes arbitrary queries without validation. Requires explicit opt-in; agents should not have access to this command by default.

Read-only mode MUST allow describe table, query plan retrieval, and other non-SELECT queries that do not modify state.

## Output

All query results are returned in TOON format for token efficiency — never as JSON tables. TOON handles tabular data, query plans, and metadata output.

## Nice-to-Have Features

- Default row limits and truncation for large result sets, with an option to disable.
- Write results to file/disk so agents can search without loading the full result into context.
- A `list-warehouses` subcommand to help users discover available Databricks SQL warehouses during setup.

## Explicit Non-Requirements

- No separate tools/commands for examining schema, structure, or statistics — LLMs can construct these queries directly.

## Security

- The tool's query validation is a first line of defense, not the only one. Database-level permissions (restricted roles/principals) are the recommended backstop.

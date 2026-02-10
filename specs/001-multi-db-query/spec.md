# Feature Specification: Multi-Database Query CLI

**Feature Branch**: `001-multi-db-query`
**Created**: 2026-02-10
**Status**: Draft
**Input**: User description: "CLI tool for querying multiple databases with TOON output, designed for humans and AI agents"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Execute a Read-Only Query Against SQL Server (Priority: P1)

A user (human or AI agent) wants to run a read-only SQL query against a SQL Server database and receive the results in TOON format. The user provides connection details, specifies the SQL Server backend, and submits a query. The tool validates that the query cannot modify state, executes it, and returns results in TOON format to stdout.

**Why this priority**: This is the core value proposition — safe, token-efficient database querying. SQL Server with read-only enforcement is the highest-risk backend (Windows auth complexity, write-prevention on a permissive engine) and must work first.

**Independent Test**: Can be fully tested by running a SELECT query against a SQL Server instance and verifying TOON-formatted output on stdout.

**Acceptance Scenarios**:

1. **Given** a configured SQL Server connection with valid credentials, **When** the user runs `exec-read` with a valid SELECT query, **Then** the tool returns query results in TOON format to stdout.
2. **Given** a configured SQL Server connection, **When** the user runs `exec-read` with a DESCRIBE or EXPLAIN query, **Then** the tool returns the metadata/plan in TOON format.
3. **Given** a configured SQL Server connection, **When** the user runs `exec-read` with an INSERT statement, **Then** the tool rejects the query before execution with a clear error message explaining that the query would modify state.
4. **Given** a configured SQL Server connection, **When** the user runs `exec-read` with a `SELECT INTO` query, **Then** the tool rejects the query before execution.
5. **Given** a configured SQL Server connection, **When** the user runs `exec-read` with an `EXEC` statement, **Then** the tool rejects the query with a message explaining that stored procedures cannot be verified as read-only.
6. **Given** a configured SQL Server connection, **When** the user runs `exec-read` with a syntactically unparseable query, **Then** the tool rejects the query with an error explaining it cannot verify safety.

---

### User Story 2 - Execute a Read-Only Query Against Databricks (Priority: P2)

A user wants to run a read-only SQL query against a Databricks SQL warehouse and receive results in TOON format. The user provides a Databricks host, auth token, warehouse identifier, and a query. The tool validates the query, executes it against the Databricks backend, and returns TOON-formatted results.

**Why this priority**: Second backend. Exercises the same read-only validation and TOON formatting but over a different transport. Proves the tool is genuinely multi-database, not SQL-Server-specific.

**Independent Test**: Can be fully tested by running a SELECT query against a Databricks SQL warehouse and verifying TOON-formatted output.

**Acceptance Scenarios**:

1. **Given** a configured Databricks connection with a valid token and warehouse, **When** the user runs `exec-read` with a valid SELECT query, **Then** the tool returns results in TOON format to stdout.
2. **Given** a configured Databricks connection, **When** the user runs `exec-read` with a Databricks-specific write operation (e.g., OPTIMIZE, VACUUM, MERGE), **Then** the tool rejects the query before execution.
3. **Given** an invalid or expired Databricks token, **When** the user runs `exec-read`, **Then** the tool returns a clear authentication error.
4. **Given** a configured Databricks connection, **When** the user submits a multi-statement batch where one statement is a write, **Then** the tool rejects the entire batch.

---

### User Story 3 - Execute a Write Query (Priority: P3)

A user with explicit write permissions wants to execute a state-modifying query against either backend. The user explicitly invokes the write command, confirming they intend to modify data. No query validation is performed — the query is executed as-is.

**Why this priority**: Write access is intentionally secondary. Most usage (especially by agents) is read-only. Write support rounds out the tool for human power-user workflows.

**Independent Test**: Can be fully tested by running an INSERT or CREATE TABLE statement against either backend and verifying the modification took effect.

**Acceptance Scenarios**:

1. **Given** a configured SQL Server connection, **When** the user runs `exec-write` with an INSERT statement, **Then** the tool executes the query and returns results (e.g., rows affected) in TOON format.
2. **Given** a configured Databricks connection, **When** the user runs `exec-write` with a CREATE TABLE statement, **Then** the tool executes the query and returns confirmation in TOON format.
3. **Given** no explicit write access configured, **When** the user runs `exec-write`, **Then** the tool denies the operation with a message explaining that write access requires explicit opt-in.

---

### User Story 4 - Row Limits and Result Truncation (Priority: P4)

A user queries a table that returns a very large result set. By default, the tool limits the number of rows returned to prevent overwhelming the terminal or consuming excessive tokens. The user can override this limit when they need the full result set.

**Why this priority**: Nice-to-have that prevents accidental context-window exhaustion for agent users. Low complexity but high practical value.

**Independent Test**: Can be tested by querying a large table and verifying the result is truncated with a message indicating more rows are available, then re-running with the limit disabled and verifying full results.

**Acceptance Scenarios**:

1. **Given** a query that would return 10,000 rows, **When** the user runs `exec-read` with default settings, **Then** the tool returns a truncated result set with a message indicating total rows available and how many were returned.
2. **Given** a query that would return 10,000 rows, **When** the user runs `exec-read` with the limit disabled, **Then** the tool returns all rows.
3. **Given** a query that returns fewer rows than the default limit, **When** the user runs `exec-read`, **Then** all rows are returned with no truncation message.

---

### User Story 5 - Write Results to File (Priority: P5)

A user wants to save query results to a file on disk instead of (or in addition to) printing to stdout. This enables agents to query large datasets and then search through the file without loading the entire result into context.

**Why this priority**: Nice-to-have that enables workflows where agents process large results incrementally. Depends on core query execution working first.

**Independent Test**: Can be tested by running a query with a file output option and verifying the file is created with correct TOON-formatted content.

**Acceptance Scenarios**:

1. **Given** a valid query and a specified output file path, **When** the user runs `exec-read` with the file output option, **Then** results are written to the specified file in TOON format.
2. **Given** a valid query and a specified output file path, **When** the user runs `exec-read` with file output, **Then** a summary (row count, file path) is printed to stdout instead of the full result.
3. **Given** an output file path in a non-existent directory, **When** the user runs with file output, **Then** the tool returns a clear error about the invalid path.

---

### User Story 6 - Discover Databricks SQL Warehouses (Priority: P6)

A user setting up dbtoon for the first time with Databricks needs to find the identifier for their SQL warehouse. A discovery subcommand lists available warehouses and their identifiers so the user can configure their connection.

**Why this priority**: Nice-to-have setup convenience. Only needed once during initial configuration.

**Independent Test**: Can be tested by running the discovery subcommand with valid Databricks credentials and verifying a list of warehouses is returned.

**Acceptance Scenarios**:

1. **Given** valid Databricks credentials, **When** the user runs the warehouse discovery subcommand, **Then** the tool displays a list of available warehouses with their identifiers and names.
2. **Given** invalid Databricks credentials, **When** the user runs the discovery subcommand, **Then** the tool returns a clear authentication error.

---

### Edge Cases

- What happens when the database connection drops mid-query? The tool MUST return a clear connection error, not a partial result or a hang.
- What happens when a query returns zero rows? The tool MUST return a valid TOON output with column headers but no data rows.
- What happens when the user provides no backend/connection configuration? The tool MUST return a clear error explaining required configuration.
- How does the tool handle query timeouts? The tool MUST support a configurable timeout (default: 60 seconds) and return a clear timeout error when exceeded.
- What happens when SQL Server Windows Auth is attempted from a non-domain-joined machine? The tool MUST return a clear authentication error from the underlying driver, not an opaque crash.
- What happens when the Databricks warehouse is stopped or starting up? The tool MUST return a clear status message rather than an opaque HTTP error.
- What happens when a multi-statement batch contains a mix of read and write statements in `exec-read` mode? The tool MUST reject the entire batch if any single statement fails validation.

## Clarifications

### Session 2026-02-10

- Q: Does a TOON format specification currently exist that this tool can produce against? → A: Yes — spec at https://toonformat.dev/guide/format-overview.html (links to complete specs at bottom), Rust implementation at https://github.com/toon-format/toon-rust
- Q: What should the default row limit be? → A: 500 rows
- Q: What should the configuration precedence order be? → A: CLI flags > env vars > config file (standard CLI convention)
- Q: What should the default query timeout be? → A: 60 seconds
- Q: Should the tool support a verbose/debug flag for diagnostics? → A: Yes, --verbose flag emitting diagnostics (connection attempts, query timing, validation steps) to stderr
- Q: What exit code convention should the tool use? → A: 0 for success, 1 for any error (standard Unix convention)
- Q: What is the write access opt-in mechanism? → A: Config/env flag required (e.g., DBTOON_ALLOW_WRITE=true) in addition to using exec-write command
- Q: Should the tool mask credentials in verbose and error output? → A: Mask credentials in all output by default; provide flag/env var to override for debugging
- Q: How should "standard connection" be defined for SC-006? → A: Local network or same-region cloud connection (< 10ms network RTT)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tool MUST support querying SQL Server databases using Windows Integrated Auth and standard SQL auth.
- **FR-002**: The tool MUST support querying Databricks SQL warehouses using token-based authentication.
- **FR-003**: The tool MUST provide a read-only execution command (`exec-read`) that validates queries cannot modify state before executing them.
- **FR-004**: The tool MUST provide a write execution command (`exec-write`) that executes queries without validation, gated behind explicit opt-in via a configuration or environment flag (e.g., `DBTOON_ALLOW_WRITE=true`). Both the flag and the `exec-write` command are required; the command alone is insufficient.
- **FR-005**: All query results MUST be returned in TOON format — never as JSON tables.
- **FR-006**: Read-only validation MUST allow: SELECT, EXPLAIN, DESCRIBE, SHOW, USE.
- **FR-007**: Read-only validation MUST deny by default: INSERT, UPDATE, DELETE, DDL, stored procedure execution, and any unrecognized statement type.
- **FR-008**: Read-only validation MUST detect and deny write operations disguised as reads: `SELECT INTO`, CTE-wrapped writes (WITH ... INSERT/UPDATE/DELETE).
- **FR-009**: Read-only validation MUST reject queries that cannot be parsed (fail safe).
- **FR-010**: Read-only validation MUST validate every statement in a multi-statement batch individually.
- **FR-011**: The tool MUST return errors to stderr and results to stdout. Exit code MUST be 0 on success and 1 on any error.
- **FR-012**: The tool MUST support a configurable default row limit of 500 rows for large result sets, with an option to disable it.
- **FR-013**: The tool MUST support writing results to a file on disk as an alternative to stdout.
- **FR-014**: The tool MUST provide a subcommand to list available Databricks SQL warehouses.
- **FR-015**: The tool MUST accept connection configuration via environment variables, command-line flags, or configuration file, with precedence: CLI flags > environment variables > config file.
- **FR-016**: The tool MUST support a `--verbose` flag that emits diagnostic information (connection attempts, query timing, validation steps) to stderr without affecting stdout output.
- **FR-017**: The tool MUST mask credentials (passwords, tokens) in all output (verbose, errors) by default. A flag or environment variable MUST be available to disable masking for manual debugging.

### Key Entities

- **Backend**: A database system the tool can connect to. Has a type (SQL Server or Databricks), connection parameters, and authentication credentials.
- **Query**: A SQL statement or batch submitted by the user. Has a text body, an execution mode (read or write), and belongs to a specific backend.
- **Query Result**: The output of executing a query. Contains column metadata and row data, serialized to TOON format.
- **Validation Result**: The outcome of read-only query analysis. Either passes (all statements are safe) or fails with a specific reason per failing statement.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can execute a read-only query against SQL Server and receive TOON-formatted results in a single command invocation.
- **SC-002**: Users can execute a read-only query against Databricks and receive TOON-formatted results in a single command invocation.
- **SC-003**: 100% of known write-pattern queries (INSERT, UPDATE, DELETE, SELECT INTO, CTE-wrapped writes, EXEC) are rejected by the read-only command before reaching the database.
- **SC-004**: Queries that fail to parse are never executed in read-only mode.
- **SC-005**: An AI agent using `exec-read` cannot accidentally modify database state through any query it constructs.
- **SC-006**: Query results for a 1,000-row table are returned in under 5 seconds on a local network or same-region cloud connection (< 10ms network RTT), excluding database-side query execution time.
- **SC-007**: A first-time user can configure the tool and run their first query within 5 minutes using only the tool's help output and error messages.
- **SC-008**: TOON-formatted output uses fewer tokens than equivalent JSON table output for the same result set.

### Assumptions

- The user's environment has the necessary database drivers installed (e.g., ODBC driver for SQL Server).
- Databricks SQL warehouses are running or auto-start is enabled.
- The TOON output format is specified at https://toonformat.dev/guide/format-overview.html (complete specs linked from that page). A Rust implementation exists at https://github.com/toon-format/toon-rust. This tool is a producer of TOON, not the format's definition.
- Connection credentials are managed by the user; the tool does not store or manage secrets beyond reading them from the environment or configuration.

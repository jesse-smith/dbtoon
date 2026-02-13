# Feature Specification: Self-Contained SQL Server Backend

**Feature Branch**: `007-tiberius-mssql`
**Created**: 2026-02-13
**Status**: Draft
**Input**: User description: "Migrate SQL Server backend from odbc-api to tiberius for self-contained binary with native GSSAPI integrated auth on macOS/Linux"

## Clarifications

### Session 2026-02-13

- Q: Should the old ODBC backend be completely removed or retained as a feature-flagged fallback? → A: Remove entirely — delete `odbc-api` dependency and all ODBC-specific code.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Integrated Auth on macOS Without Driver Install (Priority: P1)

A macOS user with a valid Kerberos ticket (`kinit`) connects to SQL Server using integrated authentication. They install dbtoon (e.g., via the cargo-dist installer) and immediately run queries without installing any additional packages — no `brew install msodbcsql18`, no ODBC driver, no Homebrew dependencies.

**Why this priority**: This is the core motivation for the migration. The current requirement to install the Microsoft ODBC Driver is the single largest onboarding friction point for the primary user base (macOS + integrated auth).

**Independent Test**: Can be validated by running `dbtoon exec-read --backend sqlserver --server <host> --windows-auth "SELECT 1"` on a stock macOS machine (with Kerberos ticket only) and receiving results.

**Acceptance Scenarios**:

1. **Given** a macOS machine with no ODBC driver installed and a valid Kerberos ticket, **When** the user runs a query with `--windows-auth`, **Then** the query executes and returns results identically to the current behavior.
2. **Given** a macOS machine with no Kerberos ticket, **When** the user runs a query with `--windows-auth`, **Then** the system returns a clear error message indicating that Kerberos authentication failed and suggesting `kinit`.
3. **Given** a macOS machine with an expired Kerberos ticket, **When** the user runs a query with `--windows-auth`, **Then** the system returns a clear error message indicating the ticket has expired.

---

### User Story 2 - SQL Login Auth Without Driver Install (Priority: P2)

A user on any platform (macOS, Linux, Windows) connects to SQL Server using SQL login credentials (username/password). They do not need to install any external driver or library.

**Why this priority**: SQL login is the second most common authentication mode and benefits equally from removing the ODBC driver dependency, but is lower priority than integrated auth since integrated auth is the core requirement.

**Independent Test**: Can be validated by running `dbtoon exec-read --backend sqlserver --server <host> --username <user> --password <pass> "SELECT 1"` on a clean machine and receiving results.

**Acceptance Scenarios**:

1. **Given** a machine with no ODBC driver installed and valid SQL login credentials, **When** the user runs a query with username/password, **Then** the query executes and returns results.
2. **Given** invalid SQL login credentials, **When** the user runs a query, **Then** the system returns a clear authentication error.
3. **Given** credentials provided via environment variables or config file, **When** the user runs a query, **Then** credential resolution follows the same precedence as today (CLI > env > config file).

---

### User Story 3 - Integrated Auth on Linux (Priority: P3)

A Linux user with a valid Kerberos ticket connects to SQL Server using integrated authentication. The user has system Kerberos libraries installed (standard on most enterprise Linux distributions), but does not need the Microsoft ODBC driver.

**Why this priority**: Extends the zero-ODBC-driver benefit to Linux users. Lower priority than macOS because macOS is the primary user base, but the same mechanism (GSSAPI) serves both platforms.

**Independent Test**: Can be validated by running `dbtoon exec-read --backend sqlserver --server <host> --windows-auth "SELECT 1"` on a Linux machine with `libgssapi-krb5` installed and a valid Kerberos ticket.

**Acceptance Scenarios**:

1. **Given** a Linux machine with system Kerberos libraries and a valid ticket, **When** the user runs a query with `--windows-auth`, **Then** the query executes and returns results.
2. **Given** a Linux machine without GSSAPI libraries installed, **When** the user runs a query with `--windows-auth`, **Then** the system returns a clear error indicating the missing dependency.

---

### User Story 4 - Seamless Migration for Existing Users (Priority: P1)

An existing dbtoon user upgrades to the new version. All their existing CLI flags, configuration files, environment variables, and workflows continue to work without modification. Query results are identical in content and format.

**Why this priority**: Equal to P1 because breaking existing users' workflows would negate the benefit of the migration. The migration must be invisible to users.

**Independent Test**: Can be validated by running the existing test suite and confirming all configuration, output format, and error behavior tests pass without modification.

**Acceptance Scenarios**:

1. **Given** an existing `config.toml` with `windows_auth = true`, **When** the user upgrades and runs their usual query, **Then** the query succeeds with identical results.
2. **Given** an existing `config.toml` with SQL login credentials, **When** the user upgrades, **Then** queries succeed with identical results.
3. **Given** a query that returns specific column types (e.g., NVARCHAR, DATETIME2, DECIMAL), **When** run against the new backend, **Then** column type names in output match the previous behavior.
4. **Given** a query with `--limit` that triggers truncation, **When** run against the new backend, **Then** truncation behavior and metadata are identical.
5. **Given** a query that exceeds `--timeout`, **When** run against the new backend, **Then** the timeout error is raised at the same threshold.

---

### Edge Cases

- What happens when the SQL Server requires a specific TLS/encryption mode? The `--trust-server-certificate` flag must continue to control encryption behavior equivalently.
- What happens when connecting to a SQL Server named instance (e.g., `hostname\INSTANCENAME`)? Named instance resolution must work.
- What happens with very large result sets (millions of rows)? Streaming behavior must not cause excessive memory usage — results should be fetched incrementally, not loaded entirely into memory.
- What happens with SQL Server column types that have no direct mapping (e.g., `sql_variant`, `geography`, `hierarchyid`)? These should fall back to a text representation, consistent with current behavior.
- What happens on Windows? Windows users should continue to work via the platform's native authentication mechanism (SSPI), maintaining feature parity.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST connect to SQL Server and execute queries without requiring an external ODBC driver on any supported platform.
- **FR-002**: System MUST support integrated authentication (Kerberos/GSSAPI) on macOS using the operating system's built-in GSS framework — no additional packages required.
- **FR-003**: System MUST support integrated authentication (Kerberos/GSSAPI) on Linux using system-provided GSSAPI libraries.
- **FR-004**: System MUST support integrated authentication (SSPI) on Windows using the operating system's built-in security provider.
- **FR-005**: System MUST support SQL login authentication (username/password) on all platforms.
- **FR-006**: System MUST preserve all existing CLI flags, environment variables, and configuration file keys — no user-facing interface changes.
- **FR-007**: System MUST produce identical column type names for all SQL Server data types covered by the current type normalizer (23+ type variants).
- **FR-008**: System MUST support query timeouts with the same `--timeout` flag and behavior.
- **FR-009**: System MUST support the `--trust-server-certificate` flag for TLS configuration.
- **FR-010**: System MUST fetch results incrementally (not loading entire result sets into memory) to maintain performance characteristics for large queries.
- **FR-011**: System MUST support SQL Server named instances.
- **FR-012**: System MUST mask credentials identically to the current behavior (passwords and connection details redacted by default, exposed with `--show-secrets`).
- **FR-013**: System MUST NOT affect the Databricks backend — changes are scoped exclusively to the SQL Server backend.
- **FR-014**: System MUST completely remove the ODBC driver dependency — no feature-flagged fallback, no optional ODBC code path. The `odbc-api` crate and all ODBC-specific code are deleted.

## Assumptions

- The `--windows-auth` CLI flag and `windows_auth` config key will be retained as-is. Although the underlying mechanism is now Kerberos/GSSAPI (not Windows-specific), renaming the flag is a separate UX concern and out of scope for this migration.
- The server address format provided by users (e.g., `hostname,port` or `hostname\instance`) will be parsed internally to match the new connection mechanism. Users do not need to change how they specify servers.
- macOS's built-in GSS.framework is sufficient for GSSAPI at runtime — this was validated in pre-specification research.
- Column types that cannot be mapped to a known SQL type will fall back to a text/unknown representation, consistent with current ODBC behavior.
- Default TLS behavior will match ODBC Driver 18: encryption is mandatory unless `--trust-server-certificate` is specified. This preserves the current security posture.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user on stock macOS (no Homebrew packages) with a valid Kerberos ticket can successfully execute queries using integrated auth.
- **SC-002**: A user on any platform with SQL login credentials can successfully execute queries without installing any external driver or library.
- **SC-003**: All existing unit and integration tests pass without modification to test assertions.
- **SC-004**: Column type names in query output match the previous ODBC-based output for all 23+ mapped SQL Server types.
- **SC-005**: Query timeout behavior triggers within the same tolerance (same `--timeout` value produces the same result).
- **SC-006**: Memory usage during large result set retrieval does not exceed previous ODBC-based behavior by more than 20%.
- **SC-007**: The distributed binary size does not increase by more than 50% compared to the current ODBC-based build (the binary no longer depends on an external driver, so a modest size increase is acceptable).

# Feature Specification: Rewrite Query Validation as Deny-List with Safe EXEC Allowlist

**Feature Branch**: `008-write-query-detection`
**Created**: 2026-02-19
**Status**: Draft
**Input**: speckit_inputs/008-write-query-detection/SPECIFY_INPUT.md

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Legitimate read-only patterns no longer rejected (Priority: P1)

A user submits a query that uses transaction wrappers, SET statements, variable declarations, or other non-mutating SQL constructs alongside a SELECT. The system validates the query and allows it to execute, rather than rejecting it as unsafe.

**Why this priority**: This is the core problem motivating the feature. The current allowlist causes false positives on common, legitimate SQL patterns, blocking users from running valid read-only queries.

**Independent Test**: Can be fully tested by submitting queries containing BEGIN TRAN/COMMIT, SET NOCOUNT ON, DECLARE statements, and verifying they pass validation.

**Acceptance Scenarios**:

1. **Given** a query containing `SET NOCOUNT ON; SELECT * FROM users`, **When** validated, **Then** the system reports it as safe.
2. **Given** a query containing `BEGIN TRAN; SELECT * FROM orders; COMMIT`, **When** validated, **Then** the system reports it as safe.
3. **Given** a query containing `DECLARE @id INT = 1; SELECT * FROM users WHERE id = @id`, **When** validated, **Then** the system reports it as safe.

---

### User Story 2 - Write operations still blocked (Priority: P1)

A user submits a query containing INSERT, UPDATE, DELETE, DROP, or other data/schema-modifying statements. The system rejects the query with a clear denial reason.

**Why this priority**: Equal priority to Story 1 — the deny-list must maintain the same write-prevention guarantees as the current allowlist. Safety is not negotiable.

**Independent Test**: Can be fully tested by submitting known write operations and verifying they are denied with appropriate reasons.

**Acceptance Scenarios**:

1. **Given** a query containing `INSERT INTO users (name) VALUES ('alice')`, **When** validated, **Then** the system denies it as a write statement.
2. **Given** a query containing `DROP TABLE users`, **When** validated, **Then** the system denies it as a write statement.
3. **Given** a query containing `GRANT SELECT ON users TO public_role`, **When** validated, **Then** the system denies it as a write statement.
4. **Given** a query containing `WITH cte AS (SELECT 1) INSERT INTO users SELECT * FROM cte`, **When** validated, **Then** the system denies it as a CTE-wrapped write.
5. **Given** a query containing `SELECT * INTO new_table FROM users`, **When** validated, **Then** the system denies it as a SELECT INTO.

---

### User Story 3 - Schema exploration via safe system procedures (Priority: P2)

A user (or LLM agent) runs EXEC with a known-safe SQL Server system stored procedure to explore database schema and metadata. The system allows these specific procedures while continuing to block arbitrary EXEC calls.

**Why this priority**: Enables LLM agents and power users to explore schema metadata without requiring direct catalog view access. Lower priority than the core deny-list rewrite because it's additive functionality.

**Independent Test**: Can be fully tested by submitting EXEC calls for each allowlisted procedure and verifying they pass, then submitting non-allowlisted EXEC calls and verifying they are denied.

**Acceptance Scenarios**:

1. **Given** a query `EXEC sp_help 'users'`, **When** validated for SQL Server, **Then** the system reports it as safe.
2. **Given** a query `EXEC sp_columns 'orders'`, **When** validated for SQL Server, **Then** the system reports it as safe.
3. **Given** a query `EXEC sp_executesql N'SELECT 1'`, **When** validated, **Then** the system denies it (opaque dynamic SQL).
4. **Given** a query `EXEC my_custom_proc`, **When** validated, **Then** the system denies it (not in allowlist).
5. **Given** a query `EXEC SP_HELP 'users'` (uppercase), **When** validated, **Then** the system reports it as safe (case-insensitive matching).

---

### Edge Cases

- What happens when a query fails to parse? Denied, same as current behavior — parse failures are never assumed safe.
- What happens with multi-statement batches mixing safe and unsafe? The entire batch is denied if any single statement is denied.
- What happens with EXEC on Databricks dialect? Databricks does not use EXEC; if sqlparser ever parses one for that dialect, it would be denied (no allowlist applies outside SQL Server).
- What happens with a procedure name that is a prefix of an allowlisted name (e.g., `sp_help_evil`)? Not matched — comparison is exact, not prefix-based.
- What happens with schema-qualified procedure names (e.g., `dbo.sp_help`)? Should still match the procedure name portion.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST deny queries containing data modification statements (INSERT, UPDATE, DELETE, MERGE).
- **FR-002**: System MUST deny queries containing schema modification statements (CREATE, ALTER, DROP, TRUNCATE).
- **FR-003**: System MUST deny queries containing privilege modification statements (GRANT, REVOKE, DENY).
- **FR-004**: System MUST deny queries containing operational statements (BACKUP, RESTORE, DBCC, BULK INSERT).
- **FR-005**: System MUST deny EXEC/EXECUTE statements by default when the target procedure is not in the safe allowlist.
- **FR-006**: System MUST deny queries containing SELECT INTO (writing to a new table).
- **FR-007**: System MUST allow all non-denied statement types, including but not limited to: SELECT, EXPLAIN, SHOW, USE, SET, BEGIN/COMMIT/ROLLBACK, DECLARE, PRINT, IF/WHILE, and transaction control.
- **FR-008**: System MUST deny queries that fail to parse, preserving current fail-closed behavior.
- **FR-016**: Denial reasons MUST identify the denied category and statement type (e.g., "Denied: DML statement (INSERT)").
- **FR-009**: System MUST allow EXEC/EXECUTE of specifically allowlisted SQL Server system stored procedures (catalog procedures and common metadata procedures).
- **FR-010**: System MUST match procedure names case-insensitively when checking the allowlist.
- **FR-015**: System MUST match the final segment of multi-part procedure names against the allowlist (e.g., `master.dbo.sp_help` → matches `sp_help`).
- **FR-011**: System MUST explicitly exclude `sp_executesql` from the allowlist regardless of its arguments.
- **FR-012**: The safe procedure allowlist MUST be compile-time only (not user-configurable) in this iteration.
- **FR-013**: The safe procedure allowlist MUST only apply when the backend dialect is SQL Server.
- **FR-014**: System MUST retain the current public validation API (`validate()`, `ValidationResult`, `BackendDialect`) without signature changes. Note: `DenialKind` variant renames (e.g., `WriteStatement` → `Dml`) are permitted as required by FR-016's category-specific denial reasons.

### Key Entities

- **Denied Statement Categories**: DML, DDL, DCL, Ops, EXEC (default), SELECT INTO — the finite set of SQL statement types that can modify state.
- **Safe Procedure Allowlist**: A curated set of SQL Server system stored procedures known to be read-only. Sourced from Microsoft's documented catalog stored procedures plus common metadata/utility procedures. The canonical list from Microsoft documentation includes:
  - **Catalog procedures** (ODBC data dictionary): `sp_column_privileges`, `sp_columns`, `sp_databases`, `sp_fkeys`, `sp_pkeys`, `sp_server_info`, `sp_special_columns`, `sp_sproc_columns`, `sp_statistics`, `sp_stored_procedures`, `sp_table_privileges`, `sp_tables`
  - **Object/metadata procedures**: `sp_help`, `sp_helptext`, `sp_helpindex`, `sp_helpconstraint`
  - **Session/server info**: `sp_who`, `sp_who2`, `sp_spaceused`
  - **Result set metadata**: `sp_describe_first_result_set`, `sp_describe_undeclared_parameters`

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All previously-denied write operations (INSERT, UPDATE, DELETE, DROP, MERGE, SELECT INTO, CTE-wrapped writes, non-allowlisted EXEC) remain denied with no regressions.
- **SC-002**: Common read-only SQL patterns (transaction wrappers, SET statements, DECLARE, variable usage) that were previously false-positived are now accepted.
- **SC-003**: All allowlisted system procedures pass validation when invoked via EXEC on SQL Server dialect.
- **SC-004**: `sp_executesql` and arbitrary user procedures are denied via EXEC.
- **SC-005**: Zero changes to the public validation API — callers are unaffected.

### Assumptions

- The set of SQL statements that can modify database state is finite and well-specified by the T-SQL and Spark SQL language references. A deny-list covering DML + DDL + DCL + Ops + EXEC is comprehensive.
- CLR user-defined functions with side effects (callable from SELECT) are out of scope — they require pre-existing malicious setup that cannot be initiated through dbtoon.
- The safe procedure allowlist is intentionally conservative. Procedures not on the list are denied, and users can request additions in future iterations.

## Clarifications

### Session 2026-02-19

- Q: How should multi-part procedure names be matched against the allowlist? → A: Match the final segment of any multi-part name (e.g., `master.dbo.sp_help` → match `sp_help`).
- Q: Should denial reasons identify the specific category that triggered rejection? → A: Yes — include denied category and statement type (e.g., "Denied: DML statement (INSERT)").

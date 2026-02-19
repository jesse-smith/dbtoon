# Research: 008-write-query-detection

## R1: Deny-List vs Allowlist Approach for SQL Statement Classification

**Decision**: Deny-list — explicitly enumerate dangerous statement types and allow everything else.

**Rationale**: The set of SQL statements that can modify database state is finite and well-documented (DML, DDL, DCL, operational). The set of read-only/harmless statements is open-ended and growing (sqlparser 0.61 has 100+ Statement variants, many dialect-specific). An allowlist must be updated every time sqlparser adds a new harmless variant; a deny-list only needs updating when a new write-capable statement type appears (rare).

**Alternatives considered**:
- Expanded allowlist: Enumerate every safe variant. Rejected because it requires tracking sqlparser's evolving AST and any missed variant results in a false positive.
- Hybrid (allowlist + catch-all deny): Same maintenance burden as pure allowlist.

## R2: sqlparser 0.61 Statement Variants Requiring Denial

**Decision**: Deny the following categories, mapped to sqlparser `Statement` variants:

### DML (Data Modification)
- `Insert`, `Update`, `Delete`, `Merge`
- `Query` with `SELECT INTO` (via `SetExpr::Select` with `into.is_some()`)
- `Query` with CTE-wrapped writes (via `SetExpr::Insert`, `SetExpr::Update`, `SetExpr::Delete`, `SetExpr::Merge`)

### DDL (Schema Modification)
- `CreateTable`, `CreateView`, `CreateIndex`, `CreateFunction`, `CreateProcedure`, `CreateTrigger`, `CreateSequence`, `CreateSchema`, `CreateDatabase`, `CreateType`, `CreateDomain`, `CreateExtension`, `CreateVirtualTable`, `CreateMacro`, `CreateSecret`, `CreateStage`, `CreateConnector`, `CreatePolicy`, `CreateRole`
- `AlterTable`, `AlterView`, `AlterSchema`, `AlterIndex`, `AlterType`, `AlterConnector`, `AlterPolicy`, `AlterRole`
- `Drop` (covers all object types via `object_type` field), `DropFunction`, `DropProcedure`, `DropTrigger`, `DropExtension`, `DropSecret`, `DropConnector`, `DropPolicy`, `DropOperator`, `DropOperatorFamily`, `DropOperatorClass`
- `Truncate`, `RenameTable`

### DCL (Privilege Modification)
- `Grant`, `Revoke`, `Deny`
- `CreateUser`, `AlterUser`

### Operational / Dangerous
- `Copy` (COPY TO/FROM), `CopyIntoSnowflake`
- `LoadData`
- `Unload`
- `Kill`
- `Flush`
- `Install` (DuckDB extension install)
- `AttachDatabase`, `AttachDuckDBDatabase`, `DetachDuckDBDatabase`

### EXEC (Conditional)
- `Execute` — denied by default; allowed only for safe procedure allowlist on SQL Server dialect

**Rationale**: This covers all write-capable operations in sqlparser 0.61. Dialect-specific variants (Snowflake, DuckDB, Hive, ClickHouse) are included for defense-in-depth even though dbtoon currently only supports SQL Server and Databricks.

**Note on exhaustiveness**: Rather than enumerating every deny variant in the match, the implementation will use a `is_denied_statement()` function. The key insight is that we can use a catch-all `_ => false` (not denied = allowed), which is the entire point of the deny-list approach. However, we should explicitly match known write variants to provide specific denial messages rather than relying solely on the catch-all.

## R3: Safe EXEC Procedure Allowlist for SQL Server

**Decision**: Compile-time constant array of procedure names, matched case-insensitively against the final segment of the `ObjectName`.

**The allowlist** (from Microsoft documentation + spec):
- Catalog procedures (ODBC): `sp_column_privileges`, `sp_columns`, `sp_databases`, `sp_fkeys`, `sp_pkeys`, `sp_server_info`, `sp_special_columns`, `sp_sproc_columns`, `sp_statistics`, `sp_stored_procedures`, `sp_table_privileges`, `sp_tables`
- Object/metadata: `sp_help`, `sp_helptext`, `sp_helpindex`, `sp_helpconstraint`
- Session/server info: `sp_who`, `sp_who2`, `sp_spaceused`
- Result set metadata: `sp_describe_first_result_set`, `sp_describe_undeclared_parameters`

**Explicitly excluded**: `sp_executesql` (opaque dynamic SQL — FR-011)

**Matching rules**:
- Case-insensitive (`SP_HELP` matches `sp_help`) — FR-010
- Final segment of multi-part name (`master.dbo.sp_help` → `sp_help`) — FR-015
- Exact match only (`sp_help_evil` does NOT match `sp_help`) — Edge case from spec

**Rationale**: These procedures are documented by Microsoft as read-only metadata retrieval. The list is conservative (FR-012 says compile-time only, not user-configurable).

**Alternatives considered**:
- Regex-based matching (`sp_*` prefix): Rejected — too permissive, would match user-defined procedures starting with `sp_`.
- User-configurable allowlist via TOML config: Deferred to future iteration per FR-012.

## R4: Handling Transaction Control and Procedural Statements

**Decision**: Allow all transaction control and procedural statements — they do not modify data by themselves.

**Allowed variants**:
- Transaction: `StartTransaction`, `Commit`, `Rollback`, `Savepoint`, `ReleaseSavepoint`
- Procedural: `If`, `While`, `Case`, `Return`, `Print`, `RaisError`, `Raise`, `Assert`
- Variable: `Set`, `Declare`

**Concern — nested writes inside transaction/control flow blocks**: `StartTransaction` in sqlparser 0.61 has a `statements` field that can contain nested statements. Similarly, `If` and `While` can contain nested statement bodies. These nested statements MUST be recursively validated.

**Decision**: Recursively validate nested statements within `StartTransaction`, `If`, `While`, and `Case` blocks. If any nested statement is denied, the entire outer statement is denied.

**Rationale**: `BEGIN TRAN; DROP TABLE users; COMMIT` must be denied even though BEGIN/COMMIT are safe. The deny-list must inspect the full statement tree.

## R5: DenialKind Enum Evolution

**Decision**: Extend `DenialKind` to provide category-specific denial reasons per FR-016.

**Current**:
```rust
pub enum DenialKind {
    WriteStatement,
    SelectInto,
    CteWrappedWrite,
    StoredProcedure,
    ParseFailure,
    Unrecognized,
}
```

**Proposed**:
```rust
pub enum DenialKind {
    Dml,              // INSERT, UPDATE, DELETE, MERGE
    Ddl,              // CREATE, ALTER, DROP, TRUNCATE, RENAME
    Dcl,              // GRANT, REVOKE, DENY
    Operational,      // BACKUP, RESTORE, COPY, LOAD, KILL, etc.
    StoredProcedure,  // Non-allowlisted EXEC
    SelectInto,       // SELECT INTO (preserved for backward compat)
    CteWrappedWrite,  // CTE-wrapped DML (preserved for backward compat)
    ParseFailure,     // Parse error (preserved)
}
```

**Changes**: `WriteStatement` splits into `Dml`, `Ddl`, `Dcl`, `Operational`. `Unrecognized` removed (deny-list approach means unknown = allowed, not denied). `SelectInto`, `CteWrappedWrite`, `StoredProcedure`, `ParseFailure` preserved.

**Concern — backward compatibility of DenialKind**: FR-014 preserves `validate()`, `ValidationResult`, and `BackendDialect` signatures. `DenialKind` is a public enum used via `DenialReason.kind`. Changing variant names is a breaking change for callers that match on `DenialKind::WriteStatement`.

**Resolution**: This is acceptable. The spec explicitly requires category-specific denial reasons (FR-016: "Denied: DML statement (INSERT)"). `DenialKind` variant changes are required to fulfill this. Current callers (only `main.rs`) use the `detail` string, not the enum variants directly. The `DenialReason.detail` format changing from "query would modify state: INSERT" to "Denied: DML statement (INSERT)" is also spec-mandated.

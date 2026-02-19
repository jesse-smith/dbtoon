# Data Model: 008-write-query-detection

## Entities

### DenialKind (enum — public)

Categorizes why a SQL statement was denied. Consumed via `DenialReason.kind`.

| Variant | Meaning | Example Triggers |
|---------|---------|------------------|
| `Dml` | Data modification | INSERT, UPDATE, DELETE, MERGE |
| `Ddl` | Schema modification | CREATE TABLE, ALTER TABLE, DROP, TRUNCATE, RENAME |
| `Dcl` | Privilege modification | GRANT, REVOKE, DENY |
| `Operational` | Dangerous operational command | COPY, LOAD, KILL, INSTALL, ATTACH |
| `StoredProcedure` | Non-allowlisted EXEC/EXECUTE | `EXEC my_proc`, `EXEC sp_executesql` |
| `SelectInto` | SELECT INTO (creates table) | `SELECT * INTO new_table FROM t` |
| `CteWrappedWrite` | DML nested inside CTE | `WITH cte AS (...) INSERT INTO t ...` |
| `ParseFailure` | SQL failed to parse | Invalid syntax |

### DenialReason (struct — public, unchanged)

```rust
pub struct DenialReason {
    pub statement_index: usize,  // 0-based index in multi-statement batch
    pub kind: DenialKind,
    pub detail: String,          // Human-readable: "Denied: DML statement (INSERT)"
}
```

### ValidationResult (enum — public, unchanged)

```rust
pub enum ValidationResult {
    Safe,
    Denied { reasons: Vec<DenialReason> },
}
```

### BackendDialect (enum — public, unchanged)

```rust
pub enum BackendDialect {
    SqlServer,
    Databricks,
}
```

### SAFE_PROCEDURES (const — private)

Compile-time allowlist of SQL Server system stored procedures known to be read-only.

```rust
const SAFE_PROCEDURES: &[&str] = &[
    // Catalog procedures (ODBC data dictionary)
    "sp_column_privileges",
    "sp_columns",
    "sp_databases",
    "sp_fkeys",
    "sp_pkeys",
    "sp_server_info",
    "sp_special_columns",
    "sp_sproc_columns",
    "sp_statistics",
    "sp_stored_procedures",
    "sp_table_privileges",
    "sp_tables",
    // Object/metadata procedures
    "sp_help",
    "sp_helptext",
    "sp_helpindex",
    "sp_helpconstraint",
    // Session/server info
    "sp_who",
    "sp_who2",
    "sp_spaceused",
    // Result set metadata
    "sp_describe_first_result_set",
    "sp_describe_undeclared_parameters",
];
```

## Relationships

```
validate(sql, dialect)
  → Parser::parse_sql(dialect, sql) → Vec<Statement>
  → for each Statement:
      → is_denied(stmt, dialect) → Option<(DenialKind, String)>
          → if Execute: check_exec_allowlist(name, dialect)
          → if Query: check_query_body(body) for SELECT INTO / CTE-wrapped writes
          → if StartTransaction/If/While: recursively check nested statements
  → collect denials → ValidationResult::Safe | ValidationResult::Denied
```

## State Transitions

N/A — validation is stateless and pure-functional. Each call to `validate()` is independent.

## Validation Rules

1. Parse failure → `DenialKind::ParseFailure` (fail-closed)
2. Any denied statement in batch → entire batch denied (collect all reasons)
3. EXEC allowlist only applies when `dialect == BackendDialect::SqlServer`
4. Multi-part procedure names: match final segment only, case-insensitive
5. `sp_executesql` explicitly excluded from allowlist regardless of arguments
6. Nested statements (in transactions, IF, WHILE) recursively validated

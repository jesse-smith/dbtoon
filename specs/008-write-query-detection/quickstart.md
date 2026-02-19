# Quickstart: 008-write-query-detection

## What Changes

One file changes: `src/validation.rs`. Tests extend: `tests/unit/validation_test.rs`.

The public API (`validate()`, `ValidationResult`, `BackendDialect`) is unchanged. `DenialKind` variants change from `WriteStatement`/`Unrecognized` to `Dml`/`Ddl`/`Dcl`/`Operational` (more specific). The only internal caller (`src/main.rs`) uses `detail` strings, not enum variants, so no cascading changes.

## Implementation Strategy

### Step 1: Update DenialKind enum

Replace `WriteStatement` and `Unrecognized` with `Dml`, `Ddl`, `Dcl`, `Operational`. Keep `StoredProcedure`, `SelectInto`, `CteWrappedWrite`, `ParseFailure`.

### Step 2: Add SAFE_PROCEDURES constant

Add the compile-time `&[&str]` array of allowlisted procedure names (21 entries).

### Step 3: Flip from allowlist to deny-list

Replace `is_safe_statement()` with `is_denied_statement()` that returns `Option<(DenialKind, String)>`:
- Match known-denied variants → return `Some((kind, detail))`
- Match `Execute` → check procedure name against allowlist (SQL Server only)
- Match `Query` → delegate to existing `is_safe_query_body()` / `classify_query_denial()`
- Match `StartTransaction`/`If`/`While` → recursively check nested statements
- Default `_` → return `None` (allowed)

### Step 4: Update denial message format

Change from `"query would modify state: INSERT"` to `"Denied: DML statement (INSERT)"` per FR-016.

### Step 5: Update validate() loop

Change from `if !is_safe_statement(stmt)` to calling `is_denied_statement()` and collecting results.

## Key Design Decisions

1. **Deny-list default = allow**: The `_ => None` arm means any sqlparser variant not explicitly denied is allowed. This is the fundamental shift.

2. **Recursive nested validation**: Transactions, IF, and WHILE blocks can contain nested statements. These must be walked recursively.

3. **EXEC allowlist is SQL Server-only**: On Databricks dialect, all EXEC is denied (Databricks doesn't use EXEC, so this is defensive).

4. **`classify_denial` simplifies**: Instead of a separate `classify_denial` function, `is_denied_statement` directly returns both the kind and detail string.

## Test Plan

Existing tests that should still pass (may need `DenialKind` variant name updates):
- All SELECT, EXPLAIN, SHOW, USE tests → still Safe
- All INSERT, UPDATE, DELETE, DROP, MERGE tests → still Denied (now `Dml` instead of `WriteStatement`)
- SELECT INTO → still Denied (`SelectInto`)
- CTE-wrapped writes → still Denied (`CteWrappedWrite`)
- Parse failures → still Denied (`ParseFailure`)

New tests needed:
- SET NOCOUNT ON → Safe
- BEGIN TRAN / COMMIT / ROLLBACK → Safe
- DECLARE @var → Safe
- SET + SELECT batch → Safe
- BEGIN TRAN wrapping SELECT → Safe
- BEGIN TRAN wrapping DROP → Denied
- GRANT/REVOKE/DENY → Denied (`Dcl`)
- CREATE INDEX/SCHEMA/etc. → Denied (`Ddl`)
- EXEC sp_help → Safe (SQL Server)
- EXEC sp_columns 'table' → Safe (SQL Server)
- EXEC SP_HELP (uppercase) → Safe (case-insensitive)
- EXEC master.dbo.sp_help → Safe (multi-part name)
- EXEC sp_executesql → Denied (explicitly excluded)
- EXEC my_custom_proc → Denied (not in allowlist)
- EXEC sp_help on Databricks → Denied (allowlist is SQL Server-only)
- EXEC sp_help_evil → Denied (exact match, not prefix)

## Commands

```bash
cargo test                          # Run all tests
cargo test validation               # Run validation tests only
cargo clippy -- -D warnings         # Zero warnings
```

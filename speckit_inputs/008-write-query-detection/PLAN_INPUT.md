
## Deny-list categories

zPlase 

- **DML:** INSERT, UPDATE, DELETE, MERGE
- **DDL:** CREATE (TABLE, VIEW, INDEX, etc.), ALTER, DROP, TRUNCATE
- **DCL:** GRANT, REVOKE, DENY
- **Ops:** BACKUP, RESTORE, DBCC, BULK INSERT
- **EXEC:** Blocked by default, with exceptions via the safe procedure allowlist
- **SELECT INTO:** Continue checking `select.into.is_some()` on `SetExpr::Select` query bodies

Everything else passes through. This includes SET, BEGIN/COMMIT/ROLLBACK, DECLARE, PRINT, IF/WHILE, transaction wrappers, etc.

## EXEC matching rules

- Match the procedure name from the `Statement::Execute` AST node.
- Case-insensitive comparison against the allowlist.
- If the procedure name is not in the allowlist, deny with `DenialKind::StoredProcedure`.
- The allowlist is a `const` array or `phf` set — no runtime configuration.

## DenialKind changes

- Remove `Unrecognized` — no longer applicable in a deny-list model.
- Keep: `WriteStatement`, `SelectInto`, `CteWrappedWrite`, `StoredProcedure`, `ParseFailure`.

## Function inversion

Replace `is_safe_statement()` / `is_safe_query_body()` with `is_denied_statement()` / `is_denied_query_body()` (or equivalent). The walk loop in `validate()` inverts accordingly.

## Test coverage additions

- Transaction wrappers (`BEGIN TRAN ... COMMIT`) — now allowed
- SET statements — now allowed
- DECLARE / variable usage — now allowed
- Safe EXEC procedures (e.g. `EXEC sp_help 'users'`) — now allowed
- Non-allowlisted EXEC (e.g. `EXEC my_write_proc`) — still denied
- `sp_executesql` — still denied
- All previously-denied writes — still denied

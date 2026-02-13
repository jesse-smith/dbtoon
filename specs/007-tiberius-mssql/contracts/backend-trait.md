# Contract: Backend Trait (Unchanged)

**Status**: No changes — preserved exactly as-is.

The `Backend` trait in `src/backend/mod.rs` is the central abstraction boundary. The tiberius migration replaces only the `SqlServerBackend` implementation; the trait and all shared types remain identical.

## Trait Signature

```rust
pub trait Backend {
    fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> impl Future<Output = Result<QueryResult, DbtoonError>> + Send;
}
```

## Contract Guarantees (unchanged)

1. **`sql`**: The raw SQL string as provided by the user (already validated by `validation.rs` for read-only mode).
2. **`limit`**: `Some(n)` means return at most `n` rows and set `truncated = true` if more exist. `None` means return all rows.
3. **`timeout_secs`**: Maximum time for query execution. Implementations MUST raise `DbtoonError::Timeout` if exceeded.
4. **Return value**: `QueryResult` with columns (name + SQL type string), rows (text or null), truncation status.

## Behavioral Contract for SqlServerBackend

These behaviors MUST be preserved across the migration:

| Behavior | ODBC Implementation | tiberius Implementation |
|----------|---------------------|------------------------|
| Connection per query | Fresh ODBC env + connection | Fresh TCP + Client |
| Auth dispatch | ODBC connection string flags | tiberius `AuthMethod` enum |
| Type names | `normalize_odbc_type(DataType)` | DMV `system_type_name` column |
| Value encoding | ODBC `as_text_view()` → UTF-8 lossy | `ColumnData` → `column_data_to_string()` |
| Row limit | Check after each batch row | Check after each streamed row |
| Timeout | ODBC statement timeout param | `tokio::time::timeout` wrapper |
| Error mapping | ODBC errors → `DbtoonError` variants | tiberius errors → `DbtoonError` variants |

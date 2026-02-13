# Data Model: Self-Contained SQL Server Backend (007-tiberius-mssql)

**Date**: 2026-02-13

## Overview

This feature replaces the SQL Server backend implementation. No new entities are introduced — the existing `Backend` trait, `QueryResult`, `ColumnMeta`, `CellValue`, and `SqlServerAuth` types are preserved. The changes are internal to `SqlServerBackend`.

## Unchanged Types (no modifications)

### `ColumnMeta` (`src/backend/mod.rs`)
```rust
pub struct ColumnMeta {
    pub name: String,
    pub type_name: String,  // e.g., "NVARCHAR(255)", "INT", "DECIMAL(18,2)"
}
```

### `CellValue` (`src/backend/mod.rs`)
```rust
pub enum CellValue {
    Text(String),
    Null,
}
```

### `QueryResult` (`src/backend/mod.rs`)
```rust
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<CellValue>>,
    pub total_rows: Option<usize>,
    pub truncated: bool,
}
```

### `Backend` trait (`src/backend/mod.rs`)
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

### `SqlServerAuth` (`src/config.rs`)
```rust
pub enum SqlServerAuth {
    WindowsIntegrated,
    SqlLogin {
        username: String,
        password: SecretString,
    },
}
```

### `DbtoonError` (`src/error.rs`)
```rust
pub enum DbtoonError {
    Validation { reason: String },
    Connection { message: String },
    Query { message: String },
    Timeout { seconds: u64 },
    Config { message: String },
    Auth { message: String },
    Io(std::io::Error),
    Format { message: String },
}
```
No new variants needed. tiberius errors map to existing `Connection`, `Query`, `Auth`, and `Timeout` variants.

## Modified Type: `SqlServerBackend` (`src/backend/sqlserver.rs`)

### Before (ODBC)
```rust
pub struct SqlServerBackend {
    server: String,
    database: Option<String>,
    auth: SqlServerAuth,
    trust_server_certificate: bool,
}
```
- Builds ODBC connection string
- Creates `odbc_api::Environment` + `Connection` in `spawn_blocking`
- Extracts types via `describe_col()` → `normalize_odbc_type()`

### After (tiberius)
```rust
pub struct SqlServerBackend {
    server: String,          // unchanged field
    database: Option<String>, // unchanged field
    auth: SqlServerAuth,     // unchanged field
    trust_server_certificate: bool, // unchanged field
}
```
- Fields are identical (the public constructor API is unchanged)
- Internally builds `tiberius::Config` instead of ODBC connection string
- Connects via `TcpStream` + `Client::connect()`
- Extracts types via DMV pre-query (`sys.dm_exec_describe_first_result_set`)
- Streams rows via `QueryStream::try_next()`

## New Internal Functions (in `src/backend/sqlserver.rs`)

### `parse_server_address`
```rust
/// Parse user-provided server string into (host, port, instance_name).
/// Formats: "host", "host,port", "host\instance", "host\instance,port"
/// Returns Err(DbtoonError::Config) for invalid port values.
fn parse_server_address(server: &str) -> Result<(String, Option<u16>, Option<String>), DbtoonError>
```

### `build_tiberius_config`
```rust
/// Build a tiberius::Config from SqlServerBackend fields.
fn build_tiberius_config(&self) -> Result<tiberius::Config, DbtoonError>
```

### `describe_result_columns` (replaces `normalize_odbc_type`)
```rust
/// Query sys.dm_exec_describe_first_result_set to get column type names.
/// Falls back to ColumnType-based mapping on failure.
async fn describe_result_columns(
    client: &mut Client<Compat<TcpStream>>,
    sql: &str,
) -> Result<Vec<ColumnMeta>, DbtoonError>
```

### `normalize_tiberius_type` (fallback mapper)
```rust
/// Best-effort mapping from tiberius ColumnType to SQL type string.
/// Used when DMV-based describe fails. Omits precision/scale/length.
fn normalize_tiberius_type(col_type: ColumnType) -> String
```

### `column_data_to_string`
```rust
/// Convert a tiberius ColumnData value to a CellValue string.
fn column_data_to_string(data: &ColumnData<'_>) -> CellValue
```

## Dependency Changes

### Removed
- `odbc-api` (from `[dependencies]` and `[dev-dependencies]`)

### Added
- `tiberius` 0.12 (features: `tds73`, `native-tls`, `integrated-auth-gssapi`, `sql-browser-tokio`)
- `tokio-util` 0.7 (features: `compat`)
- `futures-util` 0.3 (for `TryStreamExt`)

### Platform-Specific Features
- `integrated-auth-gssapi`: enabled on `cfg(not(windows))` for macOS/Linux Kerberos
- `winauth`: enabled on `cfg(windows)` for SSPI (tiberius default)

## State Transitions

No new state machines. The connection lifecycle is:

```
Config::new() → TcpStream::connect() → Client::connect() → query() → stream rows → drop Client
```

Each query creates a fresh connection (same as current ODBC behavior — no pooling).

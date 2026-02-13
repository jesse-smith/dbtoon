# Research: Self-Contained SQL Server Backend (007-tiberius-mssql)

**Date**: 2026-02-13
**Status**: Complete

## R1: tiberius Column Metadata Limitations

**Decision**: Use `sys.dm_exec_describe_first_result_set` DMV to obtain detailed column type names, with a direct `ColumnType`-based fallback.

**Rationale**: The tiberius `Column` struct only exposes `name()` and `column_type() -> ColumnType`. The `ColumnType` enum (35 variants) does NOT carry precision, scale, or max_length — these are held in the internal `TypeInfo` struct which is `pub(crate)` and inaccessible. FR-007 requires identical type names (e.g., `NVARCHAR(255)`, `DECIMAL(18,2)`, `DATETIME2(7)`), which cannot be produced from `ColumnType` alone.

The SQL Server DMV `sys.dm_exec_describe_first_result_set(@sql, NULL, 0)` returns a `system_type_name` column that produces exactly the format we need (e.g., `nvarchar(255)`, `decimal(18,2)`, `int`) without executing the query. This approach:
- Avoids patching/forking tiberius
- Produces the exact output format we need
- Adds one metadata-only round-trip per query (negligible latency)
- Works for most queries including CTEs and subqueries

**Alternatives considered**:
1. **Fork tiberius to expose `TypeInfo`**: Rejected — maintenance burden, diverges from upstream, fragile across version upgrades.
2. **Map `ColumnType` directly**: Rejected — loses precision/scale/length for 12+ type variants. Would break FR-007.
3. **Infer from `ColumnData` values**: Rejected — requires reading data first (which changes execution order), and string types don't carry max_length in their values.
4. **Query `INFORMATION_SCHEMA.COLUMNS`**: Rejected — only works for table columns, not for computed/expression columns in arbitrary queries.

**Fallback strategy**: If the DMV query fails (e.g., insufficient permissions — requires VIEW DATABASE STATE — or dynamic SQL that the DMV can't analyze), fall back to a best-effort `ColumnType` mapping that omits precision/scale/length. Emit a diagnostic warning when this happens.

## R2: tiberius Authentication — GSSAPI on macOS

**Decision**: Use tiberius with `integrated-auth-gssapi` feature flag for macOS/Linux Kerberos auth.

**Rationale**: The `integrated-auth-gssapi` feature links against the system's GSSAPI library. On macOS, this is `GSS.framework` (built into the OS, no Homebrew required). On Linux, this requires `libgssapi-krb5` (standard on enterprise distributions). The user must have a valid Kerberos ticket (`kinit`).

The `AuthMethod::Integrated` variant triggers the GSSAPI token exchange. The spec confirms (Assumption #4) that macOS's built-in GSS.framework was validated in pre-specification research.

**Alternatives considered**:
1. **NTLM via `winauth` crate**: Only works on Windows. Not applicable for macOS/Linux.
2. **Manual GSSAPI token exchange**: Rejected — tiberius already wraps `libgssapi` correctly.

## R3: tiberius Authentication — SSPI on Windows

**Decision**: Use tiberius with `winauth` feature flag (default on Windows) for SSPI integrated auth.

**Rationale**: The `winauth` feature is enabled by default on Windows targets. `AuthMethod::Integrated` uses SSPI via the `winauth` crate. `AuthMethod::windows(user, password)` supports explicit Windows credentials via NTLMv2. No additional configuration needed — this matches the current ODBC behavior.

## R4: tiberius TLS Configuration

**Decision**: Use `native-tls` feature (default) with `config.trust_cert()` mapping to `--trust-server-certificate`.

**Rationale**: tiberius supports two TLS backends: `native-tls` (uses OS-native TLS: SecureTransport on macOS, SChannel on Windows, OpenSSL on Linux) and `rustls` (pure Rust). The `native-tls` default aligns with the ODBC driver's behavior of using the OS trust store.

Mapping:
- Default (no flag): `EncryptionLevel::Required` — encryption mandatory, verify server cert against OS trust store. Matches ODBC Driver 18 default.
- `--trust-server-certificate`: call `config.trust_cert()` — trusts any certificate. Matches ODBC `TrustServerCertificate=yes`.

**Alternatives considered**:
1. **`rustls` feature**: Rejected as default — would diverge from ODBC behavior (different trust store). Could be offered as a build-time option later but out of scope.
2. **`vendored-openssl`**: Rejected — unnecessary on macOS/Windows (native TLS works), adds build complexity on Linux.

## R5: Server Address Parsing

**Decision**: Parse user-provided server strings to extract host, port, and instance name for tiberius `Config`.

**Rationale**: Users currently specify servers in ODBC format:
- `hostname` → host=hostname, port=1433 (default)
- `hostname,port` → host=hostname, port=port
- `hostname\INSTANCE` → host=hostname, instance_name=INSTANCE
- `hostname\INSTANCE,port` → host=hostname, instance_name=INSTANCE, port=port (port overrides SQL Browser)

tiberius requires these as separate config fields. A parser function will split the user-provided server string into `(host, Option<u16>, Option<String>)`.

Named instances require the `sql-browser-tokio` feature and `TcpStream::connect_named()` instead of regular `TcpStream::connect()`.

## R6: Query Timeout Implementation

**Decision**: Use `tokio::time::timeout` wrapping the entire query+streaming operation.

**Rationale**: tiberius has NO built-in query timeout support. The current ODBC backend passes `timeout_secs` to `conn.execute()` which sets a server-side statement timeout. With tiberius, we need client-side timeouts.

Approach: wrap the full query cycle (`client.query()` + row streaming) in `tokio::time::timeout(Duration::from_secs(timeout_secs), ...)`. On timeout, drop the client (connection becomes unusable). Map the `Elapsed` error to `DbtoonError::Timeout`.

**Important caveat**: tokio timeout cancels the future but the server continues executing. Since dbtoon uses one connection per query (no pooling), dropping the client closes the TCP connection, which is the correct behavior.

**Alternatives considered**:
1. **SET LOCK_TIMEOUT / SET QUERY_GOVERNOR_COST_LIMIT**: These are server-side but don't directly implement execution timeouts.
2. **Connection-level timeout**: tiberius doesn't support this.

## R7: Streaming & Memory Efficiency

**Decision**: Use tiberius `QueryStream` with `try_next()` for row-by-row streaming, converting to `CellValue::Text` strings as rows arrive.

**Rationale**: The current ODBC backend fetches in batches of 5000 via `ColumnarAnyBuffer`. tiberius natively supports async streaming via `QueryStream`. Using `try_next()` processes one row at a time with minimal memory overhead. Since we convert all values to strings anyway (`CellValue::Text`), there's no benefit to batching.

The row limit check happens inline during streaming — once `rows.len() >= limit`, we set `truncated = true` and stop consuming the stream. Dropping the `QueryStream` cleanly signals we're done.

## R8: thiserror Version Compatibility

**Decision**: Accept dual thiserror versions (1.x from tiberius, 2.x from dbtoon). Wrap tiberius errors with `anyhow` or manual `From` impls.

**Rationale**: tiberius 0.12.3 depends on `thiserror` 1.x. dbtoon uses `thiserror` 2.x. Cargo resolves both versions simultaneously since they have different major versions. The tiberius `Error` type implements `std::error::Error` (from thiserror 1.x), which is the same trait — so `From<tiberius::error::Error>` for `DbtoonError` works via the standard error trait, not thiserror-specific machinery.

## R9: tokio-util Compat Layer

**Decision**: Add `tokio-util` with `compat` feature for the `TokioAsyncWriteCompatExt` adapter.

**Rationale**: tiberius `Client::connect()` expects a stream implementing `futures_io::AsyncRead + AsyncWrite`. Tokio's `TcpStream` implements `tokio::io::AsyncRead + AsyncWrite` (different traits). The `tokio-util` crate provides `.compat_write()` to bridge these. This is the standard pattern documented by tiberius.

## R10: Value-to-String Conversion

**Decision**: Implement a `column_data_to_string(data: &ColumnData) -> CellValue` function that mirrors the current ODBC text extraction behavior.

**Rationale**: The current ODBC backend reads all values as text via `as_text_view()`. With tiberius, values arrive as typed `ColumnData` variants. We need to convert each variant to its string representation:
- `ColumnData::I32(Some(v))` → `v.to_string()`
- `ColumnData::String(Some(s))` → `s.to_string()`
- `ColumnData::Numeric(Some(n))` → formatted decimal string
- `ColumnData::DateTime2(Some(dt))` → ISO-like datetime string
- Any `None` variant → `CellValue::Null`

The conversion must produce identical string representations to what the ODBC driver produces. Key formatting concerns:
- **Decimal/Numeric**: Must preserve trailing zeros (e.g., `1.00` not `1`)
- **DateTime2**: Must match ODBC output format (e.g., `2024-01-15 14:30:00.0000000`)
- **Date**: `YYYY-MM-DD`
- **Time**: `HH:MM:SS.nnnnnnn` (with fractional precision)
- **Bit**: `0` or `1` (not `true`/`false`)
- **GUID**: Standard UUID format with dashes

This is the highest-risk area for behavioral parity and should have exhaustive test coverage.

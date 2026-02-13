# Contract: Server Address Parsing

**Status**: New internal function — replaces ODBC connection string server parameter.

## Function Signature

```rust
fn parse_server_address(server: &str) -> Result<(String, Option<u16>, Option<String>), DbtoonError>
```

Returns `Ok((host, port, instance_name))` on success, or `Err(DbtoonError::Config)` for invalid input (e.g., non-numeric port, port out of `u16` range).

## Parsing Rules

| Input format | host | port | instance_name |
|-------------|------|------|---------------|
| `hostname` | `hostname` | `None` | `None` |
| `hostname,1433` | `hostname` | `Some(1433)` | `None` |
| `hostname\INSTANCE` | `hostname` | `None` | `Some("INSTANCE")` |
| `hostname\INSTANCE,1434` | `hostname` | `Some(1434)` | `Some("INSTANCE")` |
| `tcp:hostname` | `hostname` | `None` | `None` |
| `tcp:hostname,1433` | `hostname` | `Some(1433)` | `None` |
| `192.168.1.1` | `192.168.1.1` | `None` | `None` |
| `192.168.1.1,1433` | `192.168.1.1` | `Some(1433)` | `None` |
| `192.168.1.1\INSTANCE` | `192.168.1.1` | `None` | `Some("INSTANCE")` |

## Behavior Notes

- The `tcp:` prefix (ODBC convention) is stripped if present.
- Port defaults to 1433 at the call site when `port` is `None` and `instance_name` is `None`.
- When `instance_name` is `Some`, the caller sets `config.instance_name()` and tiberius (with `sql-browser-tokio` feature) resolves the port via SQL Browser (UDP 1434) internally during `Client::connect()`.
- When both `instance_name` and explicit `port` are provided, the explicit port takes precedence (SQL Browser is not queried).
- Invalid port values (non-numeric, out of u16 range) result in `DbtoonError::Config`.

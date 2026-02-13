# Quickstart: 007-tiberius-mssql Development

## Prerequisites

- Rust stable (2024 edition)
- A SQL Server instance for integration testing (local or remote)
- For Kerberos testing: valid `kinit` ticket on macOS/Linux
- On Linux: `libkrb5-dev` (Debian/Ubuntu) or `krb5-devel` (RHEL/Fedora) for GSSAPI compilation

## Build

```bash
cargo build
```

New dependencies are added to `Cargo.toml`; `cargo build` resolves them automatically. The `odbc-api` dependency is removed — builds no longer require an ODBC driver manager or SQL Server ODBC driver.

## Test

```bash
# Unit tests (no SQL Server required)
cargo test

# Clippy (zero warnings required)
cargo clippy -- -D warnings
```

Unit tests cover:
- Server address parsing (all format variants)
- Type normalization fallback mapping (all 27+ ColumnType variants)
- Value-to-string conversion (all ColumnData variants)
- Existing tests: config resolution, validation, output formatting (unchanged)

Integration tests (manual — require SQL Server):
- SQL login auth: `cargo run -- exec-read -b sqlserver -s <host> -u <user> -p <pass> "SELECT 1 AS x"`
- Windows auth: `cargo run -- exec-read -b sqlserver -s <host> -w "SELECT 1 AS x"`
- Named instance: `cargo run -- exec-read -b sqlserver -s "host\INSTANCE" -w "SELECT 1 AS x"`
- Trust cert: `cargo run -- exec-read -b sqlserver -s <host> --trust-server-certificate -u <user> -p <pass> "SELECT 1"`

## Key Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Remove `odbc-api`, add `tiberius`, `tokio-util`, `futures-util` |
| `src/backend/sqlserver.rs` | Replace entire implementation |
| `src/lib.rs` | No change (module declarations unchanged) |
| `src/backend/mod.rs` | No change |
| `src/config.rs` | No change |
| `src/cli.rs` | No change |
| `src/main.rs` | No change (dispatches through same trait) |
| `tests/unit/` | Add type mapping + address parsing tests |

## Architecture Notes

- The `Backend` trait is the boundary. All changes are behind `SqlServerBackend`.
- No other module (config, CLI, validation, formatting, masking) changes.
- The Databricks backend is completely untouched.
- Tests that referenced `odbc_api` types in `[dev-dependencies]` will need updating.

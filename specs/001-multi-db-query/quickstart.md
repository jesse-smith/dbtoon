# Quickstart: dbtoon

## Prerequisites

- Rust toolchain (stable, 2024 edition) — install via [rustup](https://rustup.rs/)
- **SQL Server users**: ODBC Driver 18 for SQL Server installed ([Microsoft docs](https://learn.microsoft.com/en-us/sql/connect/odbc/download-odbc-driver-for-sql-server))
- **Databricks users**: A Databricks personal access token and a running SQL warehouse

## Build

```bash
cargo build --release
```

The binary is at `target/release/dbtoon`.

## First Query (SQL Server, Windows Auth)

```bash
dbtoon exec-read \
  --backend sqlserver \
  --server localhost \
  --database mydb \
  --windows-auth \
  "SELECT TOP 10 * FROM users"
```

## First Query (SQL Server, SQL Auth)

```bash
export DBTOON_PASSWORD='your-password'

dbtoon exec-read \
  --backend sqlserver \
  --server localhost \
  --database mydb \
  --username sa \
  "SELECT TOP 10 * FROM users"
```

## First Query (Databricks)

```bash
export DBTOON_DATABRICKS_TOKEN='dapi...'

dbtoon exec-read \
  --backend databricks \
  --host adb-123456.azuredatabricks.net \
  --warehouse abc123def456 \
  "SELECT * FROM main.default.my_table LIMIT 10"
```

## Using Config Profiles

Create a config file at `~/.config/dbtoon/config.toml`:

```toml
[profiles.prod]
backend = "sqlserver"
server = "prod-db.corp.example.com"
database = "analytics"
windows_auth = true
```

Then use the profile:

```bash
dbtoon exec-read --profile prod "SELECT COUNT(*) FROM orders"
```

## Discover Databricks Warehouses

```bash
export DBTOON_DATABRICKS_TOKEN='dapi...'

dbtoon list-warehouses --host adb-123456.azuredatabricks.net
```

## Save Results to File

```bash
dbtoon exec-read --profile prod --output results.toon "SELECT * FROM orders"
```

## Enable Write Access

```bash
export DBTOON_ALLOW_WRITE=true

dbtoon exec-write --profile prod "INSERT INTO logs (msg) VALUES ('test')"
```

## Diagnostic Mode

Add `--verbose` to see connection timing, validation steps, and query timing on stderr:

```bash
dbtoon exec-read --profile prod --verbose "SELECT 1"
```

## Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `error: validation: query would modify state` | Write query in `exec-read` mode | Use `exec-write` (with `DBTOON_ALLOW_WRITE=true`) |
| `error: validation: cannot verify query safety` | Unparseable SQL in `exec-read` mode | Fix SQL syntax or use `exec-write` |
| `error: auth: connection failed` | Wrong credentials or driver missing | Check credentials; verify ODBC driver installed |
| `error: config: no backend specified` | Missing `--backend` and no profile | Add `--backend` flag or `--profile` |
| `error: connection: timeout after 60s` | Database unreachable or slow | Check network; increase `--timeout` |
| `error: auth: write access denied` | `exec-write` without opt-in | Set `DBTOON_ALLOW_WRITE=true` |

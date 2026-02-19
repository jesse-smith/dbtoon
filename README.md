# dbtoon

A multi-database query CLI that outputs results in [TOON format](https://github.com/nickolasburr/toon-format). Supports SQL Server (via ODBC) and Databricks (via REST API).

## Features

- **Profile-based connections** — manage database connections as named profiles
- **Read-only queries** with AST-based validation that rejects writes before execution
- **Write queries** via `--allow-write` flag on the `query` command
- **Row limiting** with configurable limits and `--no-limit` override
- **Multiple output formats** — TOON, CSV, Parquet, Arrow IPC via `--output`
- **Databricks warehouse discovery** via `warehouse list`
- **Config file initialization** with `dbtoon init`
- **`$VAR` env var references** in profile fields for secure credential management
- **Credential masking** by default (secrets redacted in diagnostics)

## Quick Start

```sh
# 1. Create a config file
dbtoon init

# 2. Create a profile
dbtoon profile create mydb --backend sqlserver --set server=localhost --set database=mydb

# 3. Run a query
dbtoon query -P mydb "SELECT TOP 10 * FROM users"
```

## Installation

### macOS / Linux

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.sh | sh
```

### Windows (PowerShell)

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.ps1 | iex"
```

### From source

Requires Rust (stable, 2024 edition):

```sh
cargo install dbtoon
```

### Prerequisites

- [ODBC Driver 18 for SQL Server](https://learn.microsoft.com/en-us/sql/connect/odbc/download-odbc-driver-for-sql-server) (for SQL Server backend)

## Updating

```sh
dbtoon update
```

If you installed via `cargo install`, update with `cargo install dbtoon` instead.

## Usage

```sh
# SQL Server query
dbtoon query -P dev-sql "SELECT TOP 10 * FROM users"

# Databricks query
dbtoon query -P prod-databricks "SELECT * FROM catalog.schema.table"

# Query with overrides
dbtoon query -P dev-sql -d otherdb -l 100 -t 120 "SELECT 1"

# Read SQL from file
dbtoon query -P dev-sql -f query.sql

# Write query (requires opt-in)
dbtoon query -P dev-sql --allow-write "INSERT INTO logs (msg) VALUES ('hello')"

# Output to file (format detected by extension)
dbtoon query -P dev-sql -o results.csv "SELECT 1"

# List Databricks warehouses
dbtoon warehouse list -P prod-databricks

# Profile management
dbtoon profile create mydb --backend sqlserver
dbtoon profile edit mydb --set server=newhost
dbtoon profile show mydb
dbtoon profile list
dbtoon profile rename mydb mydb-old
dbtoon profile delete mydb-old
```

## Configuration

Config file location: `~/.config/dbtoon/config.toml` (all platforms), or use `-c <path>`.

Run `dbtoon init` to create a config file with defaults and example profiles.

```toml
[defaults]
row_limit = 500
timeout = 60

[profiles.dev-sql]
backend = "sqlserver"
server = "localhost,1433"
database = "mydb"
username = "sa"
password = "$MY_SQL_PASSWORD"
trust_server_certificate = true

[profiles.prod-databricks]
backend = "databricks"
host = "$DATABRICKS_HOST"
token = "$DATABRICKS_TOKEN"
warehouse_id = "$DATABRICKS_SQL_WAREHOUSE_ID"
catalog = "$DATABRICKS_CATALOG"
schema = "$DATABRICKS_SCHEMA"
```

### `$VAR` References

String profile fields can reference environment variables using `$VAR` syntax:

- `host = "$DATABRICKS_HOST"` — resolved at use-time from the env var
- `host = "$$pecial"` — escaped to literal `$pecial`
- `host = "literal.host"` — used as-is

If a referenced env var is not set, dbtoon errors (no silent fallthrough).

### Config Resolution Hierarchy

Values are resolved in priority order:

1. **CLI flags** (`--limit`, `--timeout`, `-d`, `-s`, `--allow-write`, `--no-limit`)
2. **TOML profile** (`[profiles.<name>]` fields)
3. **TOML defaults** (`[defaults]` section)
4. **Databricks standard env vars** (lowest-priority fallback, Databricks only)

## Databricks Standard Environment Variables

For Databricks profiles, these standard env vars are used as lowest-priority fallbacks when not set in the profile or defaults:

| Variable | Maps to |
|----------|---------|
| `DATABRICKS_HOST` | `host` |
| `DATABRICKS_TOKEN` | `token` |
| `DATABRICKS_SQL_WAREHOUSE_ID` | `warehouse_id` |
| `DATABRICKS_CATALOG` | `catalog` |
| `DATABRICKS_SCHEMA` | `schema` |

## Build from source

```sh
cargo build --release
```

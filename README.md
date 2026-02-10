# dbtoon

A multi-database query CLI that outputs results in [TOON format](https://github.com/nickolasburr/toon-format). Supports SQL Server (via ODBC) and Databricks (via REST API).

## Features

- **Read-only queries** with AST-based validation that rejects writes before execution
- **Write queries** via opt-in `exec-write` command (requires `DBTOON_ALLOW_WRITE=true`)
- **Row limiting** with configurable limits and `--no-limit` override
- **File output** with `--output` flag
- **Databricks warehouse discovery** via `list-warehouses`
- **TOML config profiles** with env var and CLI flag overrides
- **Credential masking** by default (secrets redacted in diagnostics)

## Prerequisites

- Rust (stable, 2024 edition)
- [ODBC Driver 18 for SQL Server](https://learn.microsoft.com/en-us/sql/connect/odbc/download-odbc-driver-for-sql-server) (for SQL Server backend)

## Build

```sh
cargo build --release
```

## Usage

```sh
# SQL Server (Windows Auth)
dbtoon exec-read -b sqlserver -s localhost -d mydb -w "SELECT TOP 10 * FROM users"

# SQL Server (SQL Auth, self-signed cert)
dbtoon exec-read -b sqlserver -s localhost -d mydb -u sa -p secret \
  --trust-server-certificate "SELECT 1"

# Databricks
dbtoon exec-read -b databricks --host adb-123.azuredatabricks.net \
  --warehouse abc123 --token dapi... "SELECT * FROM catalog.schema.table"

# List Databricks warehouses
dbtoon list-warehouses --host adb-123.azuredatabricks.net --token dapi...

# Write query (requires opt-in)
DBTOON_ALLOW_WRITE=true dbtoon exec-write -b sqlserver -s localhost -d mydb -w \
  "INSERT INTO logs (msg) VALUES ('hello')"

# Output to file
dbtoon exec-read -b sqlserver -s localhost -d mydb -w -o results.toon "SELECT 1"
```

## Configuration

Config file location: `~/.config/dbtoon/config.toml` (Linux), `~/Library/Application Support/dbtoon/config.toml` (macOS), or `--config <path>`.

```toml
[defaults]
row_limit = 500
timeout = 60

[profiles.dev-sql]
backend = "sqlserver"
server = "localhost,1433"
database = "mydb"
username = "sa"
password_env = "MY_SQL_PASSWORD"
trust_server_certificate = true

[profiles.prod-databricks]
backend = "databricks"
host = "adb-123.azuredatabricks.net"
warehouse_id = "abc123def"
token_env = "DATABRICKS_TOKEN"
catalog = "main"
schema = "default"
```

```sh
dbtoon exec-read -P dev-sql "SELECT 1"
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `DBTOON_BACKEND` | Backend type (`sqlserver` or `databricks`) |
| `DBTOON_SERVER` | SQL Server hostname |
| `DBTOON_DATABASE` | Database name |
| `DBTOON_USERNAME` | SQL Auth username |
| `DBTOON_PASSWORD` | SQL Auth password |
| `DBTOON_WINDOWS_AUTH` | Use Windows Integrated Auth |
| `DBTOON_TRUST_SERVER_CERT` | Trust SQL Server certificate |
| `DBTOON_DATABRICKS_HOST` | Databricks workspace host |
| `DBTOON_DATABRICKS_TOKEN` | Databricks bearer token |
| `DBTOON_WAREHOUSE_ID` | Databricks SQL warehouse ID |
| `DBTOON_CATALOG` | Databricks catalog |
| `DBTOON_SCHEMA` | Databricks schema |
| `DBTOON_ROW_LIMIT` | Default row limit |
| `DBTOON_TIMEOUT` | Query timeout in seconds |
| `DBTOON_ALLOW_WRITE` | Enable write queries (`true`) |
| `DBTOON_CONFIG` | Config file path |
| `DBTOON_VERBOSE` | Enable verbose diagnostics |
| `DBTOON_SHOW_SECRETS` | Disable credential masking |

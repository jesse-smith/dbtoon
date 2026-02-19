# Feature: Simplify CLI Interface (Issue #14)

Restructure the dbtoon CLI to separate connection management from query execution, replace `exec-read`/`exec-write` with a unified `query` command, and add profile management commands.

## Commands

### `dbtoon init`
Creates a config file at `~/.config/dbtoon/config.toml` (all platforms, including macOS — do not use `~/Library/`). Generated file contains:
- A `[defaults]` section with `row_limit`, `timeout`, `verbose`, `allow_write` fields
- A commented-out example Databricks profile showing all available options (`backend`, `host`/`host_env`, `token`/`token_env`, `warehouse_id`/`warehouse_id_env`, `catalog`/`catalog_env`, `schema`/`schema_env`)
- A commented-out example SQL Server profile showing all available options (`backend`, `server`/`server_env`, `database`/`database_env`, `username`/`username_env`, `password`/`password_env`, `windows_auth`, `trust_server_certificate`)

At init time, `dbtoon init` should check for Databricks standard env vars (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`). If any are set, write them as `_env` passthroughs (e.g., `host_env = "DATABRICKS_HOST"`) in the Databricks profile and uncomment it. Do not resolve env vars to literal values — keep them as references so the config stays portable.

After writing the config file, `dbtoon init` should print:
- The path to the created config file
- Instructions on how to set up profiles (`dbtoon profile create` or edit the file directly)
- For any uncommented profiles, a list of required fields that still need to be set (e.g., `warehouse_id` for Databricks, `server` for SQL Server)

If no config file exists, all commands that need one should error with a message directing the user to run `dbtoon init`.

### `dbtoon query`
Replaces both `exec-read` and `exec-write`. Requires `-P <PROFILE>` to specify connection. Flags:
- Positional `<SQL>` or `-f <FILE>` (mutually exclusive, same as today)
- `-l, --limit <N>` — override config row_limit
- `-t, --timeout <N>` — override config timeout
- `-o, --output <PATH>` — file output, format detected from extension (same as today)
- `--allow-write` — skip AST safety validation
- `--no-limit` — override any configured row_limit, return all rows
- `-d, --database <DB>` / `--catalog <CATALOG>` — these are aliases for the same concept: override the profile's database (SQL Server) or catalog (Databricks). Both flags are accepted on either backend. For SQL Server, this means a reconnect if it differs from the profile value.
- `-s, --schema <SCHEMA>` — override profile schema. For SQL Server, this is a query-level namespace (no reconnect). For Databricks, sent as a per-request parameter.

These override profile values but do not modify the profile. If neither the flag nor the profile sets a value, the backend default is used (SQL Server: login default database; Databricks: warehouse default catalog).

No connection-identity flags (server, host, token, warehouse, auth) on this command.

### `dbtoon profile create <NAME> --backend <sqlserver|databricks> [--set key=value]...`
Creates a new profile in the TOML config. If no `--set` flags are provided, generates env-var-indirection defaults appropriate to the backend (e.g., `token_env = "DATABRICKS_TOKEN"` for databricks). `--set` flags override or add fields; supports both literal values (`--set server=localhost`) and env indirection (`--set password_env=DB_PASSWORD`).

### `dbtoon profile edit <NAME> [--set key=value]...`
Updates fields on an existing profile using the same `--set` syntax as `create`.

### `dbtoon profile show <NAME>`
Displays the profile config with resolved values. For `_env` fields, shows the env var name, the resolved value (masked by default), and a warning if the var is unset.

### `dbtoon profile list`
Lists all profiles.

### `dbtoon profile test <NAME>`
Verifies connectivity using the profile's resolved config. Reports success or the specific failure.

### `dbtoon profile delete <NAME>`
Removes a profile from config.

### `dbtoon profile rename <OLD> <NEW>`
Renames a profile in config.

### `dbtoon warehouse list -P <PROFILE>`
Existing functionality, but connection details now come from the profile instead of inline flags.

### `dbtoon update`
Unchanged.

## Global flags (all commands)
- `-c, --config <PATH>` — config file path (default: `~/.config/dbtoon/config.toml`)
- `-v, --verbose` — diagnostics to stderr
- `--show-secrets` — disable credential masking

## Config resolution hierarchy
```
CLI flags > TOML [profiles.<name>] > TOML [defaults] > Databricks standard env vars
```

TOML profile values include both literal values and `_env` indirection (same priority tier — they're mutually exclusive per field). If an `_env` field references an unset var, that's an error (not a fallthrough).

## Environment variables
All `DBTOON_*` env vars are removed. The only env vars recognized are the Databricks ecosystem standard vars (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`), which sit at the lowest priority in the hierarchy.

## Removals
- `exec-read` subcommand
- `exec-write` subcommand
- Connection-identity flags from query commands (`--server`, `--host`, `--token`, `--warehouse`, `--username`, `--password`, `--windows-auth`, `--trust-server-certificate`, `--backend`). Note: `--database`, `--catalog`, and `--schema` are replaced by the new `-d`/`--database` and `-s`/`--schema` query-level flags.
- Connection flags from `warehouse list` (`--host`, `--token`)
- All `DBTOON_*` env vars (`DBTOON_SERVER`, `DBTOON_DATABASE`, `DBTOON_USERNAME`, `DBTOON_PASSWORD`, `DBTOON_BACKEND`, `DBTOON_WINDOWS_AUTH`, `DBTOON_TRUST_SERVER_CERT`, `DBTOON_DATABRICKS_HOST`, `DBTOON_DATABRICKS_TOKEN`, `DBTOON_WAREHOUSE_ID`, `DBTOON_PROFILE`, `DBTOON_ALLOW_WRITE`, `DBTOON_VERBOSE`, `DBTOON_SHOW_SECRETS`, `DBTOON_CONFIG`, `DBTOON_ROW_LIMIT`, `DBTOON_TIMEOUT`)
- Hardcoded default values for row_limit and timeout (these come from the config file generated by `init`)
- macOS `~/Library/` config path — use `~/.config/dbtoon/` on all platforms

## Notes
- `database` (SQL Server) and `catalog` (Databricks) are analogous concepts — `-d`/`--database` maps to whichever is appropriate for the backend. Both are optional in profiles and as flags.
- Databricks `catalog` and `schema` are per-request parameters (REST API body), not connection-level. SQL Server `database` is connection-level (ODBC connection string) — overriding via flag requires a reconnect, which is fine today (no pooling). Future connection pooling should key pools by `(server, database, auth)`.
- `schema` is query-level on both backends (namespace within database/catalog). No reconnect implications.
- Databricks `catalog` and `schema` are not validated until a table is actually referenced. Queries using fully-qualified names or no table references work without them.

## Config file structure (post-refactor)

Example generated by `dbtoon init` when Databricks env vars are detected:
```toml
[defaults]
row_limit = 500
timeout = 60
verbose = false
allow_write = false

# Auto-detected from environment variables
[profiles.databricks]
backend = "databricks"
host_env = "DATABRICKS_HOST"
token_env = "DATABRICKS_TOKEN"
# warehouse_id = ""    # REQUIRED — use `dbtoon warehouse list -P databricks` to find yours
# catalog = ""         # optional — only needed for unqualified table references
# schema = ""          # optional — only needed for unqualified table references

# Example SQL Server profile — uncomment and fill in to use
# [profiles.sqlserver]
# backend = "sqlserver"
# server = "localhost"           # or server_env = "MY_SERVER_VAR"
# database = "mydb"              # or database_env = "MY_DB_VAR"
# username = "sa"                # or username_env = "MY_USER_VAR"
# password = "secret"            # or password_env = "MY_PASS_VAR"
# windows_auth = false           # set true to use Windows Integrated Auth (ignores username/password)
# trust_server_certificate = false  # set true for self-signed/dev certs
```

stdout after writing the above:
```
Created config at ~/.config/dbtoon/config.toml

Profiles:
  databricks  (auto-detected from env vars)
    Still needed:
      warehouse_id  — use `dbtoon warehouse list -P databricks` to find yours
  sqlserver   (example — commented out)
    Still needed:
      server        — SQL Server hostname
      auth          — set username + password, or windows_auth = true

To create additional profiles: dbtoon profile create <name> --backend <sqlserver|databricks>
Or edit the config file directly: ~/.config/dbtoon/config.toml
```

Example generated by `dbtoon init` when no Databricks env vars are set:
```toml
[defaults]
row_limit = 500
timeout = 60
verbose = false
allow_write = false

# Example Databricks profile — uncomment and fill in to use
# [profiles.databricks]
# backend = "databricks"
# host = "your-workspace.cloud.databricks.net"  # or host_env = "DATABRICKS_HOST"
# token = "dapi..."                              # or token_env = "DATABRICKS_TOKEN"
# warehouse_id = ""    # use `dbtoon warehouse list -P databricks` to find yours
# catalog = ""         # optional — only needed for unqualified table references
# schema = ""          # optional — only needed for unqualified table references

# Example SQL Server profile — uncomment and fill in to use
# [profiles.sqlserver]
# backend = "sqlserver"
# server = "localhost"           # or server_env = "MY_SERVER_VAR"
# database = "mydb"              # or database_env = "MY_DB_VAR"
# username = "sa"                # or username_env = "MY_USER_VAR"
# password = "secret"            # or password_env = "MY_PASS_VAR"
# windows_auth = false           # set true to use Windows Integrated Auth (ignores username/password)
# trust_server_certificate = false  # set true for self-signed/dev certs
```

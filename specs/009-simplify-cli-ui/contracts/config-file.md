# Config File Contract: Simplify CLI Interface

**Feature Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19

## File Format

TOML 1.0. Located at `~/.config/dbtoon/config.toml`.

## Schema

```toml
# Global defaults — applied when not overridden by profile or CLI flags
[defaults]
row_limit = 500          # usize, optional, default 500
timeout = 60             # u64 seconds, optional, default 60
verbose = false          # bool, optional, default false
allow_write = false      # bool, optional, default false

# Named connection profiles
[profiles.<name>]
backend = "<type>"       # Required: "databricks" | "sqlserver"
# ... backend-specific fields (see below)
```

## Databricks Profile Fields

```toml
[profiles.my_databricks]
backend = "databricks"
host = "$DATABRICKS_HOST"                      # string, required
token = "$DATABRICKS_TOKEN"                    # string (secret), required
warehouse_id = "$DATABRICKS_SQL_WAREHOUSE_ID"  # string, required
catalog = "$DATABRICKS_CATALOG"                # string, optional
schema = "$DATABRICKS_SCHEMA"                  # string, optional
```

## SQL Server Profile Fields

```toml
[profiles.my_sqlserver]
backend = "sqlserver"
server = "localhost"                  # string, required
database = "mydb"                    # string, optional
username = "sa"                      # string, required if !windows_auth
password = "$SA_PASSWORD"            # string (secret), required if !windows_auth
windows_auth = false                 # bool, optional, default false
trust_server_certificate = false     # bool, optional, default false
```

## $VAR Resolution Rules

1. A string value starting with `$` (not `$$`) is an env var reference
2. The env var must be set and non-empty, or it is an error
3. `$$` at the start escapes to a literal `$` (e.g., `$$pecial` → `$pecial`)
4. Resolution occurs at **use-time** (when the profile is used for a query), not at parse-time
5. Only applies to **string-typed** fields. `windows_auth` and `trust_server_certificate` (bool) are always literal.

## Invariants

- Profile names are case-sensitive, valid TOML keys
- No duplicate profile names
- `backend` field is mandatory in every profile
- Comments and formatting are preserved across `profile` subcommand edits (via `toml_edit`)

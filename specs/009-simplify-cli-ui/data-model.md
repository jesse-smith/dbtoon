# Data Model: Simplify CLI Interface

**Feature Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19

## Entities

### Config File

**Location**: `~/.config/dbtoon/config.toml` (all platforms)
**Format**: TOML
**Lifecycle**: Created by `dbtoon init`, modified by `profile` subcommands, read by `query`/`warehouse`

```toml
[defaults]
row_limit = 500        # usize, optional (default: 500)
timeout = 60           # u64 seconds, optional (default: 60)
verbose = false        # bool, optional (default: false)
allow_write = false    # bool, optional (default: false)

[profiles.<name>]
backend = "databricks"                # Required: "databricks" | "sqlserver"
# ... backend-specific fields below
```

### Profile (Databricks)

| Field | Type | Required | `$VAR` | Default `$VAR` |
|-------|------|----------|--------|-----------------|
| `backend` | string | yes | no | — |
| `host` | string | yes | yes | `$DATABRICKS_HOST` |
| `token` | string (secret) | yes | yes | `$DATABRICKS_TOKEN` |
| `warehouse_id` | string | yes | yes | `$DATABRICKS_SQL_WAREHOUSE_ID` |
| `catalog` | string | no | yes | `$DATABRICKS_CATALOG` |
| `schema` | string | no | yes | `$DATABRICKS_SCHEMA` |

### Profile (SQL Server)

| Field | Type | Required | `$VAR` | Default `$VAR` |
|-------|------|----------|--------|-----------------|
| `backend` | string | yes | no | — |
| `server` | string | yes | yes | — |
| `database` | string | no | yes | — |
| `username` | string | cond.* | yes | — |
| `password` | string (secret) | cond.* | yes | — |
| `windows_auth` | bool | no | no | `false` |
| `trust_server_certificate` | bool | no | no | `false` |

*\*Required when `windows_auth` is not `true`.*

### Defaults

Global fallbacks in `[defaults]`. Applied when not overridden by profile or CLI flags.

| Field | Type | Default |
|-------|------|---------|
| `row_limit` | usize | 500 |
| `timeout` | u64 | 60 |
| `verbose` | bool | false |
| `allow_write` | bool | false |

## Configuration Resolution Hierarchy

Priority (highest first):
1. **CLI flags** (`--limit`, `--timeout`, `-d`, `-s`, `--allow-write`, `--no-limit`)
2. **TOML profile** (`[profiles.<name>]` fields)
3. **TOML defaults** (`[defaults]` section)
4. **Databricks standard env vars** (lowest-priority fallback, Databricks backend only)

A `$VAR` reference to an unset variable is an **error** — no fallthrough to the next level.

## State Transitions

### Config File Lifecycle

```
[not exists] --init--> [exists with defaults + example profiles]
[exists]     --profile create--> [profile added]
[exists]     --profile edit--> [profile field modified/added/removed]
[exists]     --profile delete--> [profile removed]
[exists]     --profile rename--> [profile renamed]
```

### $VAR Resolution

```
"$VARNAME" --resolve--> env var value (or error if unset)
"$$literal" --resolve--> "$literal"
"plain"    --resolve--> "plain" (unchanged)
```

## Validation Rules

- Profile names: valid TOML key names, case-sensitive
- Profile names: must not already exist (for `create`), must exist (for `edit`/`delete`/`rename`/`show`/`test`)
- Backend: must be `"databricks"` or `"sqlserver"`
- `$VAR` resolution: only on string-typed fields; bool/numeric fields are always literals
- `--database`/`--catalog`: true aliases, mutually exclusive (clap conflict group)
- SQL input: exactly one of positional SQL or `-f` (clap `conflicts_with`)

## Struct Changes (Rust)

### New: `TomlProfile` updates

Remove `password_env` and `token_env` fields (replaced by `$VAR` syntax within `password`/`token` fields).

### New: CLI struct hierarchy

```
Cli
├── --config, --verbose, --show-secrets (global)
├── init (InitArgs) — new
├── query (QueryArgs) — replaces ExecRead/ExecWrite
│   ├── -P/--profile (required)
│   ├── SQL (positional) | -f/--file
│   ├── -d/--database, --catalog (aliases, conflict group)
│   ├── -s/--schema
│   ├── -l/--limit, --no-limit, -t/--timeout
│   ├── -o/--output
│   └── --allow-write
├── profile (ProfileCommand) — new
│   ├── create <name> --backend [--set key=value...]
│   ├── edit <name> [--set key=value...] [--unset key...]
│   ├── show <name>
│   ├── list
│   ├── test <name>
│   ├── delete <name>
│   └── rename <old> <new>
├── warehouse
│   └── list -P <profile>
└── update
```

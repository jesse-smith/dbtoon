# CLI Interface Contract: Simplify CLI Interface

**Feature Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19

This document defines the complete CLI interface contract after the restructuring. It serves as the authoritative reference for clap struct definitions.

## Global Flags

| Flag | Short | Type | Default | Env | Description |
|------|-------|------|---------|-----|-------------|
| `--config` | `-c` | `PathBuf` | `~/.config/dbtoon/config.toml` | — | Config file path |
| `--verbose` | `-v` | `bool` | `false` | — | Emit diagnostics to stderr |
| `--show-secrets` | — | `bool` | `false` | — | Disable credential masking |

**Note**: No `DBTOON_*` env vars. Global flags are CLI-only.

## Commands

### `dbtoon init`

Creates a config file with defaults and example profiles.

| Arg/Flag | Type | Description |
|----------|------|-------------|
| *(none)* | — | Uses global `--config` or default path |

**Exit codes**: 0 = created, 1 = already exists or write error

---

### `dbtoon query`

Execute a SQL query against a profile.

| Arg/Flag | Short | Type | Required | Description |
|----------|-------|------|----------|-------------|
| `<SQL>` | — | positional | one of SQL/`-f` | SQL query text |
| `--file` | `-f` | `PathBuf` | one of SQL/`-f` | Read SQL from file |
| `--profile` | `-P` | `String` | **yes** | Profile name |
| `--database` | `-d` | `String` | no | Override catalog/database (alias: `--catalog`) |
| `--catalog` | — | `String` | no | Override catalog/database (alias: `--database`) |
| `--schema` | `-s` | `String` | no | Override schema |
| `--limit` | `-l` | `usize` | no | Override row limit |
| `--no-limit` | — | `bool` | no | Disable row limit |
| `--timeout` | `-t` | `u64` | no | Override timeout (seconds) |
| `--output` | `-o` | `PathBuf` | no | Write to file (format by extension) |
| `--allow-write` | — | `bool` | no | Bypass read-only safety |

**Conflicts**: `<SQL>` vs `--file`, `--database` vs `--catalog`

---

### `dbtoon profile create <NAME>`

Create a new profile.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<NAME>` | positional | **yes** | Profile name |
| `--backend` | `String` | **yes** | `databricks` or `sqlserver` |
| `--set` | `Vec<String>` | no | `key=value` pairs (repeatable) |

---

### `dbtoon profile edit <NAME>`

Edit an existing profile.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<NAME>` | positional | **yes** | Profile name |
| `--set` | `Vec<String>` | no | `key=value` pairs; `key=` removes field (repeatable) |
| `--unset` | `Vec<String>` | no | Field names to remove (repeatable) |

---

### `dbtoon profile show <NAME>`

Display profile with resolved values and masking.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<NAME>` | positional | **yes** | Profile name |

---

### `dbtoon profile list`

List all profile names. No arguments.

---

### `dbtoon profile test <NAME>`

Test connectivity for a profile.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<NAME>` | positional | **yes** | Profile name |

---

### `dbtoon profile delete <NAME>`

Delete a profile from config.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<NAME>` | positional | **yes** | Profile name |

---

### `dbtoon profile rename <OLD> <NEW>`

Rename a profile.

| Arg/Flag | Type | Required | Description |
|----------|------|----------|-------------|
| `<OLD>` | positional | **yes** | Current name |
| `<NEW>` | positional | **yes** | New name |

---

### `dbtoon warehouse list`

List Databricks SQL warehouses.

| Arg/Flag | Short | Type | Required | Description |
|----------|-------|------|----------|-------------|
| `--profile` | `-P` | `String` | **yes** | Databricks profile name |

---

### `dbtoon update`

Self-update to latest release. No arguments.

## Removed (Compared to Current CLI)

### Removed Commands
- `exec-read` → use `query`
- `exec-write` → use `query --allow-write`
- `list-warehouses` → use `warehouse list`

### Removed Flags (from query/warehouse)
- `--backend`, `--server`, `--host`, `--token`, `--warehouse`
- `--username`, `--password`, `--windows-auth`, `--trust-server-certificate`

### Removed Env Vars
- All `DBTOON_*` env vars (`DBTOON_CONFIG`, `DBTOON_VERBOSE`, `DBTOON_SHOW_SECRETS`, `DBTOON_BACKEND`, `DBTOON_SERVER`, `DBTOON_DATABASE`, `DBTOON_USERNAME`, `DBTOON_PASSWORD`, `DBTOON_WINDOWS_AUTH`, `DBTOON_TRUST_SERVER_CERT`, `DBTOON_DATABRICKS_HOST`, `DBTOON_DATABRICKS_TOKEN`, `DBTOON_WAREHOUSE_ID`, `DBTOON_CATALOG`, `DBTOON_SCHEMA`, `DBTOON_ROW_LIMIT`, `DBTOON_TIMEOUT`, `DBTOON_PROFILE`, `DBTOON_ALLOW_WRITE`)

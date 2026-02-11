# Quickstart: Standard Databricks Environment Variable Fallback

**Feature**: 002-std-env-vars | **Date**: 2026-02-11

## What changed

dbtoon now recognizes standard Databricks environment variables as a fallback when dbtoon-specific variables are not set. If you already have `DATABRICKS_HOST`, `DATABRICKS_TOKEN`, etc. configured for the Databricks CLI or SDK, dbtoon will use them automatically.

## Before (old behavior)

```bash
# Had to set dbtoon-specific vars even if standard ones existed
export DATABRICKS_HOST="https://my-workspace.azuredatabricks.net"
export DATABRICKS_TOKEN="dapiXXXXXXXX"

# This would fail — dbtoon didn't see the standard vars
dbtoon exec-read -b databricks --warehouse abc123 "SELECT 1"

# Had to duplicate:
export DBTOON_DATABRICKS_HOST="https://my-workspace.azuredatabricks.net"
export DBTOON_DATABRICKS_TOKEN="dapiXXXXXXXX"
```

## After (new behavior)

```bash
# Standard Databricks vars just work
export DATABRICKS_HOST="https://my-workspace.azuredatabricks.net"
export DATABRICKS_TOKEN="dapiXXXXXXXX"
export DATABRICKS_SQL_WAREHOUSE_ID="abc123"

dbtoon exec-read -b databricks "SELECT 1"  # Works!
```

## Priority order

When multiple sources define the same value, dbtoon uses this priority (highest wins):

1. **CLI flag** (e.g., `--host my-host`)
2. **dbtoon-specific env var** (e.g., `DBTOON_DATABRICKS_HOST`)
3. **TOML config profile field** (e.g., `host = "my-host"` in profile)
4. **Standard Databricks env var** (e.g., `DATABRICKS_HOST`)
5. **TOML defaults section** (for non-connection settings like `row_limit`)

## Supported standard variables

| Standard Variable | Equivalent dbtoon Variable | CLI Flag |
|---|---|---|
| `DATABRICKS_HOST` | `DBTOON_DATABRICKS_HOST` | `--host` |
| `DATABRICKS_TOKEN` | `DBTOON_DATABRICKS_TOKEN` | `--token` |
| `DATABRICKS_SQL_WAREHOUSE_ID` | `DBTOON_WAREHOUSE_ID` | `--warehouse` |
| `DATABRICKS_CATALOG` | `DBTOON_CATALOG` | `--catalog` |
| `DATABRICKS_SCHEMA` | `DBTOON_SCHEMA` | `--schema` |

## Empty string handling

Environment variables set to an empty string are treated as unset. This applies to both dbtoon-specific and standard variables:

```bash
export DBTOON_DATABRICKS_HOST=""        # Treated as unset
export DATABRICKS_HOST="my-host.net"    # This value is used
```

## No breaking changes

- All existing CLI flags, env var names, and TOML config fields continue to work exactly as before.
- dbtoon-specific env vars still take precedence over standard ones.
- TOML profile values still take precedence over all env vars.

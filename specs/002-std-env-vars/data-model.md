# Data Model: Standard Databricks Environment Variable Fallback

**Feature**: 002-std-env-vars | **Date**: 2026-02-11

## Configuration Resolution Model

No new data structures are introduced. The existing `AppConfig`, `BackendConfig`, `TomlConfig`, and `TomlProfile` structs are unchanged. The change is purely in the *resolution logic* that populates these structs.

### Entity: Configuration Field (Databricks)

Each Databricks configuration field is resolved independently through a priority ladder.

| Field | Type | Required | Tier 1: CLI/dbtoon env | Tier 2: TOML profile | Tier 3: Standard env var | Tier 4: Default |
|-------|------|----------|----------------------|---------------------|-------------------------|-----------------|
| host | `String` | Yes | `args.host` / `DBTOON_DATABRICKS_HOST` | `profile.host` | `DATABRICKS_HOST` | Error |
| token | `SecretString` | Yes | `args.token` / `DBTOON_DATABRICKS_TOKEN` | `profile.token_env` → `profile.token` | `DATABRICKS_TOKEN` | Error |
| warehouse_id | `String` | Yes (exec) | `args.warehouse` / `DBTOON_WAREHOUSE_ID` | `profile.warehouse_id` | `DATABRICKS_SQL_WAREHOUSE_ID` | Error |
| catalog | `Option<String>` | No | `args.catalog` / `DBTOON_CATALOG` | `profile.catalog` | `DATABRICKS_CATALOG` | None |
| schema | `Option<String>` | No | `args.schema` / `DBTOON_SCHEMA` | `profile.schema` | `DATABRICKS_SCHEMA` | None |

### Token resolution detail

Token has a more complex Tier 1+2 because of the `resolve_secret` function and `token_env` indirection:

```
Tier 1a: args.token (CLI --token / DBTOON_DATABRICKS_TOKEN)
Tier 1b: profile.token_env indirection (reads env var named by TOML field)
Tier 1c: DBTOON_DATABRICKS_TOKEN direct fallback (in resolve_secret)
Tier 2:  profile.token (TOML literal)
Tier 3:  DATABRICKS_TOKEN (standard env var)  ← new
Tier 4:  Error
```

### Validation rules

- Empty strings are treated as absent at every tier (FR-004).
- Each field resolves independently — a value from Tier 3 for one field does not affect another field's resolution.
- Required fields that exhaust all tiers produce `DbtoonError::Config` with a descriptive message.

### Functions modified

| Function | File | Change |
|----------|------|--------|
| `load_from_exec_args` | `src/config.rs` | Add Tier 3 fallback for all 5 Databricks fields; wrap existing tiers in `non_empty()` |
| `load_from_list_warehouses_args` | `src/config.rs` | Add Tier 3 fallback for host, token, warehouse_id, catalog, schema |
| — | `src/config.rs` | Add `non_empty()` and `env_non_empty()` helper functions |

### Functions unchanged

| Function | Why |
|----------|-----|
| `resolve_secret` | Already handles empty strings; standard env var is chained *after* its return, not inside it |
| `resolve_config_path` | Unrelated to Databricks env vars |
| `load_toml_config` | Unrelated |

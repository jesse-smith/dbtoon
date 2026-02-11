# Research: Standard Databricks Environment Variable Fallback

**Feature**: 002-std-env-vars | **Date**: 2026-02-11

## R1: Standard Databricks Environment Variable Names

**Decision**: Use the exact names from the Databricks SDK/CLI ecosystem.

**Rationale**: These are the canonical names used by the Databricks CLI, Python SDK, and other official tooling. Users who have these set already expect them to work.

**Source**: [GitHub Issue #2](https://github.com/jesse-smith/dbtoon/issues/2) — confirmed by the Databricks documentation naming convention.

| Standard Variable | Purpose |
|---|---|
| `DATABRICKS_HOST` | Workspace URL (e.g., `https://adb-1234.azuredatabricks.net`) |
| `DATABRICKS_TOKEN` | Personal access token |
| `DATABRICKS_SQL_WAREHOUSE_ID` | SQL warehouse identifier |
| `DATABRICKS_CATALOG` | Unity Catalog catalog name |
| `DATABRICKS_SCHEMA` | Unity Catalog schema name |

**Alternatives considered**: None — these are the standard names; inventing different ones would defeat the purpose.

## R2: Insertion point in the resolution chain

**Decision**: Standard env vars go after TOML profile values and before the terminal condition (Error or None).

**Rationale**: The existing chain is `CLI/dbtoon-env (clap) → TOML profile → Error/None`. Standard env vars are less specific than a user's explicit dbtoon config or TOML profile, so they rank lower. But they are more authoritative than "no value at all."

The key insight: clap merges CLI flags and dbtoon-specific env vars into a single tier internally (`args.host` is populated from either `--host` or `DBTOON_DATABRICKS_HOST`). This means the effective resolution is 4 tiers, not 5:

1. CLI flag / dbtoon-specific env var (clap)
2. TOML profile field
3. Standard Databricks env var ← new
4. Error (required) or None (optional)

**Alternatives considered**:
- Putting standard env vars *above* TOML profile: Rejected — TOML profiles are user-intentional config; they should override environment defaults.
- Adding standard env vars as additional clap `env` values: Rejected — clap's `env` attribute only takes one var name. Using it would make CLI and standard env vars the same tier, preventing TOML from overriding standard env vars.

## R3: Empty-string handling approach

**Decision**: Add two helpers (`non_empty`, `env_non_empty`) to filter empty strings for non-secret fields. Apply to all tiers consistently.

**Rationale**: `resolve_secret` already filters empty strings for secret fields. Non-secret fields (host, warehouse_id, catalog, schema) currently pass empty strings through, which could lead to confusing errors downstream (e.g., connecting to an empty hostname). FR-004 makes this behavior explicit.

**Alternatives considered**:
- Only filter on the new standard env var tier: Rejected — inconsistent; an empty `DBTOON_DATABRICKS_HOST` would still pass through and block fallback to `DATABRICKS_HOST`.
- Modify clap to reject empty strings: Rejected — clap doesn't support empty-string rejection natively for `Option<String>` fields; would require custom validators on every field.

## R4: Test serialization strategy

**Decision**: Use `std::sync::Mutex<()>` static + `Drop`-based guard for env var cleanup. No new crate dependencies.

**Rationale**: Process env vars are global state. Rust's default test parallelism would cause race conditions. A static mutex serializes env-var tests without adding external dependencies.

**Alternatives considered**:
- `serial_test` crate: Clean API but adds a dev dependency for a single use case.
- `temp_env` crate: Scoped env var changes, but also an extra dependency.
- Dependency injection (pass env reader as parameter): Over-engineering per constitution principle III — the indirection isn't needed for correctness; just for test convenience.

## R5: Scope of changes to `load_from_list_warehouses_args`

**Decision**: Apply the same standard env var fallback to the `list-warehouses` subcommand.

**Rationale**: `list-warehouses` also resolves Databricks host and token. Without the fallback, users with only `DATABRICKS_HOST`/`DATABRICKS_TOKEN` set would get errors from `list-warehouses` while `exec-read` works — that would violate Least Surprise.

The function currently also reads `warehouse_id`, `catalog`, and `schema` from the profile only. Standard env var fallback is added for these too, consistent with exec-args behavior.

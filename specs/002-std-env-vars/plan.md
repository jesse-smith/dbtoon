# Implementation Plan: Standard Databricks Environment Variable Fallback

**Branch**: `002-std-env-vars` | **Date**: 2026-02-11 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-std-env-vars/spec.md`

## Summary

Add standard Databricks environment variables (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`) as a fallback tier in the configuration resolution chain. When dbtoon-specific env vars and TOML profile values are absent, the system checks standard Databricks env vars before erroring or defaulting to None. Also add empty-string-as-unset filtering for all env var tiers (currently only handled in `resolve_secret`), and comprehensive tests covering the priority ladder.

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `clap` 4.5 (CLI/env parsing), `serde`/`toml` 0.8 (config), `secrecy` 0.10 (credential masking), `dotenvy` 0.15 (.env loading)
**Storage**: TOML config file (read-only); no database changes
**Testing**: `cargo test` — unit tests in `tests/unit/`; `std::sync::Mutex` for env-var test serialization
**Target Platform**: Cross-platform CLI (Linux, macOS, Windows)
**Project Type**: Single Rust binary crate
**Performance Goals**: N/A — config resolution is instantaneous
**Constraints**: Non-breaking change; all existing CLI flags, env var names, and TOML fields preserved
**Scale/Scope**: 1 source file modified (`config.rs`), 1 test file expanded (`config_test.rs`)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | Pass | Adds `.or()` fallback calls to existing chains — easy to explain |
| II. Engineering Fundamentals — DRY | Pass | Two small helper functions (`non_empty`, `env_non_empty`) eliminate repeated empty-string checks across 5+ call sites |
| II. Engineering Fundamentals — YAGNI | Pass | Only adding what issue #2 requests; no speculative features |
| II. Engineering Fundamentals — KISS | Pass | Straightforward Option chaining; no new abstractions |
| II. Engineering Fundamentals — Least Surprise | Pass | Databricks users expect `DATABRICKS_HOST` to work; this matches that expectation |
| II. Engineering Fundamentals — Fail Fast | Pass | Required fields still error when all tiers are exhausted |
| III. Over-Engineering Guards — Rule of Three | Pass | Helpers used 5+ times each; well past the threshold |
| IV. TDD | Enforced | Tests written before implementation changes |
| V. Incremental Delivery | Enforced | Five commits per tasks.md — see commit boundaries in tasks.md for authoritative breakdown |

No violations. No complexity tracking needed.

## Project Structure

### Documentation (this feature)

```text
specs/002-std-env-vars/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── config.rs            # Modified: add standard env var fallback + helpers
├── cli.rs               # Unchanged (clap env bindings stay as-is)
└── ...                  # All other files unchanged

tests/
└── unit/
    └── config_test.rs   # Modified: add priority-ladder and empty-string tests
```

**Structure Decision**: Single-project layout; changes confined to `config.rs` (logic) and `config_test.rs` (tests). No new files in `src/`.

## Design Decisions

### D1: Where standard env vars slot in the resolution chain

**Current chain** (Databricks `host` as example):
```
args.host (CLI flag / DBTOON_DATABRICKS_HOST via clap) → profile.host (TOML) → Error
```

**New chain**:
```
args.host (CLI / DBTOON_DATABRICKS_HOST) → profile.host (TOML) → DATABRICKS_HOST (std env) → Error
```

This applies to all 5 Databricks fields in both `load_from_exec_args` and `load_from_list_warehouses_args`.

### D2: Token resolution with `resolve_secret`

The existing `resolve_secret` function handles a 3-tier ladder: direct CLI value → `token_env` indirection → `DBTOON_DATABRICKS_TOKEN` fallback. The new `DATABRICKS_TOKEN` standard env var is added *after* the existing chain (including `profile.token` literal), as a final `.or_else()`.

```
resolve_secret(args.token, profile.token_env, "DBTOON_DATABRICKS_TOKEN")
  → profile.token (TOML literal)
  → DATABRICKS_TOKEN (std env)     ← new
  → Error
```

### D3: Empty-string filtering (FR-004)

`resolve_secret` already filters empty strings. Non-secret fields (`host`, `warehouse_id`, `catalog`, `schema`) currently do **not** filter empty strings. Two helpers address this:

- `non_empty(s: Option<&str>) -> Option<&str>`: Filters `Some("")` to `None`.
- `env_non_empty(key: &str) -> Option<String>`: Reads env var, returns `None` if unset or empty.

Applied to all tiers including args (clap may set `Some("")` from an empty env var).

### D4: Test serialization for env vars

Process env vars are global mutable state. Tests that set/read env vars must not run in parallel. Use a `static Mutex<()>` in the test module — each env-var test acquires the lock before setting vars. A `Drop`-based guard struct ensures vars are cleaned up even on panic.

### D5: Mapping table — dbtoon vars to standard vars

| CLI flag | dbtoon env var | Standard fallback env var |
|----------|---------------|--------------------------|
| `--host` | `DBTOON_DATABRICKS_HOST` | `DATABRICKS_HOST` |
| `--token` | `DBTOON_DATABRICKS_TOKEN` | `DATABRICKS_TOKEN` |
| `--warehouse` | `DBTOON_WAREHOUSE_ID` | `DATABRICKS_SQL_WAREHOUSE_ID` |
| `--catalog` | `DBTOON_CATALOG` | `DATABRICKS_CATALOG` |
| `--schema` | `DBTOON_SCHEMA` | `DATABRICKS_SCHEMA` |

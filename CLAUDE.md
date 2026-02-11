# dbtoon Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-10

## Active Technologies

- Rust (stable, 2024 edition) + `odbc-api` 20 (SQL Server), `reqwest` 0.12 + `tokio` 1 (Databricks), `sqlparser` 0.61 (validation), `toon-format` 0.4 (output), `clap` 4.5 (CLI), `serde`/`toml` (config), `thiserror` 2 + `anyhow` 1 (errors), `secrecy` 0.10 (credential masking) (001-multi-db-query)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust (stable, 2024 edition): Follow standard conventions

## Recent Changes

- 001-multi-db-query: Added Rust (stable, 2024 edition) + `odbc-api` 20 (SQL Server), `reqwest` 0.12 + `tokio` 1 (Databricks), `sqlparser` 0.61 (validation), `toon-format` 0.4 (output), `clap` 4.5 (CLI), `serde`/`toml` (config), `thiserror` 2 + `anyhow` 1 (errors), `secrecy` 0.10 (credential masking)

<!-- MANUAL ADDITIONS START -->
## Feature Completion Workflow

When a feature branch is merged to main:

1. Run `/speckit.featcomp.complete` to update status tracking:
   - Adds completion headers to spec.md, plan.md, tasks.md
   - Updates the central `specs/STATUS.md` registry
2. Commit the status updates

See `specs/STATUS.md` for the current feature registry.
<!-- MANUAL ADDITIONS END -->

# dbtoon Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-10

## Active Technologies
- Rust (stable, 2024 edition) + `clap` 4.5 (CLI/env parsing), `serde`/`toml` 0.8 (config), `secrecy` 0.10 (credential masking), `dotenvy` 0.15 (.env loading) (002-std-env-vars)
- TOML config file (read-only); no database changes (002-std-env-vars)
- Rust (stable, 2024 edition) + `odbc-api` 20 (SQL Server ODBC — source of `DataType` enum), `toon-format` 0.4 (output encoding), `serde_json` 1 (intermediate JSON representation) (003-column-type-metadata)
- Rust (stable 1.91.1, 2024 edition) + `csv` 1.4 (CSV writing), `arrow` 57 (Arrow arrays + IPC writer), `parquet` 57 (Parquet writer); existing: `toon-format` 0.4, `odbc-api` 20, `serde_json` 1, `clap` 4.5 (004-output-formats)
- File output (CSV, Parquet, Arrow IPC, TOON); no database changes (004-output-formats)
- Rust (stable 1.91.1, 2024 edition) + Existing: `toon-format` 0.4 (TOON encoding), `arrow` 57 (Arrow schema metadata + IPC), `parquet` 57 (Parquet writer), `serde_json` 1 (JSON construction), `csv` 1.4; no new dependencies (005-truncation-metadata)
- File output (TOON, CSV, Parquet, Arrow IPC); no database changes (005-truncation-metadata)
- Rust (stable, 2024 edition) + Existing (`clap` 4.5, `tokio` 1, `anyhow` 1, `thiserror` 2) + New (`axoupdater` 0.9 for self-update); cargo-dist 0.30.3 (build tooling, not a runtime dep) (006-cargo-dist-release)
- N/A (install receipts managed by cargo-dist installer, not by dbtoon) (006-cargo-dist-release)
- Rust (stable, 2024 edition) + `sqlparser` 0.61 (SQL parsing + AST — already integrated), `clap` 4.5, `thiserror` 2, `anyhow` 1 (008-write-query-detection)
- N/A — pure validation logic, no persistence changes (008-write-query-detection)
- Rust (stable, 2024 edition) + `clap` 4.5 (CLI), `toml` 0.8 (config read), `toml_edit` 0.25 (config write — NEW), `secrecy` 0.10 (masking), `serde` 1 (deserialization), `sqlparser` 0.61 (validation) (009-simplify-cli-ui)
- TOML config file at `~/.config/dbtoon/config.toml`; no database changes (009-simplify-cli-ui)

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
- 009-simplify-cli-ui: Added Rust (stable, 2024 edition) + `clap` 4.5 (CLI), `toml` 0.8 (config read), `toml_edit` 0.25 (config write — NEW), `secrecy` 0.10 (masking), `serde` 1 (deserialization), `sqlparser` 0.61 (validation)
- 008-write-query-detection: Added Rust (stable, 2024 edition) + `sqlparser` 0.61 (SQL parsing + AST — already integrated), `clap` 4.5, `thiserror` 2, `anyhow` 1
- 006-cargo-dist-release: Added Rust (stable, 2024 edition) + Existing (`clap` 4.5, `tokio` 1, `anyhow` 1, `thiserror` 2) + New (`axoupdater` 0.9 for self-update); cargo-dist 0.30.3 (build tooling, not a runtime dep)


<!-- MANUAL ADDITIONS START -->
## Feature Completion Workflow

When a feature branch is merged to main:

1. Run `/speckit.featcomp.complete` to update status tracking:
   - Adds completion headers to spec.md, plan.md, tasks.md
   - Updates the central `specs/STATUS.md` registry
2. Commit the status updates

See `specs/STATUS.md` for the current feature registry.

## Release Workflow

Releases use `cargo-release` + `cargo-dist`. Never manually edit the version in `Cargo.toml` or create tags by hand.

To release, run one of:
```bash
cargo release patch --execute   # 0.2.0 → 0.2.1
cargo release minor --execute   # 0.2.0 → 0.3.0
cargo release major --execute   # 0.2.0 → 1.0.0
```

This automatically: bumps `Cargo.toml` version, updates `Cargo.lock`, commits, tags (`v{version}`), and pushes. The pushed tag triggers the cargo-dist GitHub Actions workflow to build platform binaries and create a GitHub Release.

- Always run from the `main` branch (enforced by config).
- Omit `--execute` for a dry run first.
- Ensure all tests and clippy pass before releasing.
<!-- MANUAL ADDITIONS END -->

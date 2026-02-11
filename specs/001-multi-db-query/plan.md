# Implementation Plan: Multi-Database Query CLI

> **STATUS: COMPLETE** | Merged: 2026-02-11 | Branch: `001-multi-db-query`

**Branch**: `001-multi-db-query` | **Date**: 2026-02-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-multi-db-query/spec.md`

## Summary

A Rust CLI tool (`dbtoon`) for executing SQL queries against SQL Server (via ODBC) and Databricks (via REST API), returning results in TOON format. The tool enforces read-only safety through SQL parsing (allowlist on `sqlparser` AST), supports write access behind explicit opt-in, and is designed for both human operators and AI agents. Key design drivers: fail-safe query validation, token-efficient output via TOON tabular format, and zero-runtime-dependency distribution (aside from ODBC driver for SQL Server).

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `odbc-api` 20 (SQL Server), `reqwest` 0.12 + `tokio` 1 (Databricks), `sqlparser` 0.61 (validation), `toon-format` 0.4 (output), `clap` 4.5 (CLI), `serde`/`toml` (config), `thiserror` 2 + `anyhow` 1 (errors), `secrecy` 0.10 (credential masking)
**Storage**: N/A — stateless CLI tool, no persistent storage
**Testing**: `cargo test` — unit tests for validation logic, integration tests against live backends (gated behind feature flags or env vars)
**Target Platform**: Cross-platform CLI binary (macOS, Linux, Windows)
**Project Type**: Single project (Rust binary crate with library modules)
**Performance Goals**: <5s for 1,000-row result on local network / same-region cloud (<10ms RTT), excluding database-side execution time (SC-006)
**Constraints**: Requires ODBC Driver 18 for SQL Server at runtime; Databricks requires network access + bearer token; default 500-row limit to prevent context-window exhaustion
**Scale/Scope**: CLI tool, 2 database backends, 3 subcommands, ~15 CLI flags

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity First** | PASS | Two backends with a shared trait. No plugin system, no dynamic loading. CLI with 3 subcommands. Config is flat TOML profiles. |
| **II. Engineering Fundamentals** | PASS | DI via `Backend` trait (depend on abstraction). SRP: validation, execution, formatting are separate modules. Fail Fast: parse failures rejected immediately. Immutability: `QueryResult` and `ValidationResult` are owned, immutable values. |
| **III. Over-Engineering Guards** | PASS | Two backends — no premature abstraction (not three yet, but a trait is warranted since both backends share the exact same execute/column-metadata contract). No config file watcher, no connection pooling, no plugin system. |
| **IV. TDD** | PASS | Validation logic is pure-function testable (SQL string → ValidationResult). TOON formatting is testable against known inputs. Backend execution requires integration test setup. |
| **V. Incremental Delivery** | PASS | User stories are prioritized P1-P6. Natural incremental path: validation → SQL Server backend → Databricks backend → write mode → row limits → file output → warehouse discovery. Each is independently testable and committable. |

### Post-Phase 1 Re-Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity First** | PASS | Data model has 12 types, all flat. No inheritance hierarchies. `CellValue` is intentionally stringly-typed to avoid a complex type-mapping layer. |
| **II. Engineering Fundamentals** | PASS | Backend trait provides Dependency Inversion. `ValidationResult`/`DenialReason` are explicit about failure modes. Config precedence is explicit (CLI > env > file). |
| **III. Over-Engineering Guards** | PASS | `Backend` trait is justified by two concrete implementations sharing the same contract (not premature — the abstraction exists because there are already two backends). No generic "database driver" framework. |
| **IV. TDD** | PASS | Validation module is pure-function: `fn validate(sql: &str, dialect: Dialect) -> ValidationResult`. Highly testable with table-driven tests covering all denial kinds. |
| **V. Incremental Delivery** | PASS | Module boundaries align with user story boundaries. P1 (SQL Server read) can be delivered without P2 (Databricks), P3 (write), etc. |

**No violations. No complexity tracking entries needed.**

## Project Structure

### Documentation (this feature)

```text
specs/001-multi-db-query/
├── plan.md              # This file
├── research.md          # Phase 0 output — technology decisions and rationale
├── data-model.md        # Phase 1 output — entity definitions
├── quickstart.md        # Phase 1 output — first-time user guide
├── contracts/
│   ├── cli-interface.md # CLI subcommands, flags, output contracts
│   └── config-schema.toml # Config file format with examples
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
Cargo.toml
src/
├── main.rs              # Entrypoint, clap parsing, dispatch
├── cli.rs               # Clap derive structs (subcommands, args)
├── config.rs            # Config file loading, env var resolution, precedence merge
├── backend/
│   ├── mod.rs           # Backend trait definition
│   ├── sqlserver.rs     # SQL Server backend (odbc-api)
│   └── databricks.rs    # Databricks backend (reqwest + REST API)
├── validation.rs        # Read-only query validation (sqlparser)
├── format.rs            # QueryResult → TOON serialization (toon-format)
├── output.rs            # Output routing (stdout vs file, truncation messages)
├── error.rs             # Error types (thiserror enums)
└── masking.rs           # SecretString helpers, verbose output with masking

tests/
├── unit/
│   ├── validation_test.rs    # Table-driven tests for read-only validation
│   ├── format_test.rs        # TOON output correctness
│   ├── config_test.rs        # Config precedence, profile resolution
│   └── masking_test.rs       # Credential masking behavior
└── integration/
    ├── sqlserver_test.rs     # Live SQL Server queries (gated: requires ODBC + server)
    └── databricks_test.rs    # Live Databricks queries (gated: requires token + warehouse)
```

**Structure Decision**: Single Rust binary crate. Modules under `src/` follow Separation of Concerns: CLI parsing, config resolution, backend execution, query validation, output formatting, and error handling are each isolated. The `backend/` directory uses a trait + two implementations rather than a generic plugin system — appropriate for exactly two backends. Tests split into unit (no external dependencies) and integration (require live backends, gated by env vars or cargo features).

## Complexity Tracking

> No violations detected. Table left empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| *(none)* | — | — |

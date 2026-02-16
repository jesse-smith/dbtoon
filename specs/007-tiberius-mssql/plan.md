# Implementation Plan: Self-Contained SQL Server Backend

**Branch**: `007-tiberius-mssql` | **Date**: 2026-02-13 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/007-tiberius-mssql/spec.md`
**Status**: Archived (2026-02-16)
**Archive Tag**: `archive/007-tiberius-mssql`
**Archive Reason**: Tiberius 0.12 has unresolved macOS TLS + GSSAPI issues; ODBC backend retained. See [KNOWN-ISSUES.md](KNOWN-ISSUES.md).

## Summary

Migrate the SQL Server backend from `odbc-api` (requires external ODBC driver) to `tiberius` (native TDS protocol over TCP). This eliminates the ODBC driver dependency, enabling zero-install SQL Server connectivity on macOS/Linux. Integrated authentication uses GSSAPI (macOS GSS.framework / Linux libgssapi) instead of the ODBC driver's built-in mechanism. Column type metadata is obtained via `sys.dm_exec_describe_first_result_set` DMV to preserve identical type name output. All user-facing interfaces (CLI flags, config, output formats) remain unchanged.

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `tiberius` 0.12 (TDS client), `tokio-util` 0.7 (compat layer), `futures-util` 0.3 (stream helpers); existing: `tokio` 1, `clap` 4.5, `serde`/`toml`, `secrecy` 0.10, `thiserror` 2, `anyhow` 1
**Storage**: N/A — no database changes, no local storage changes
**Testing**: `cargo test` (unit), `cargo clippy` (lint); manual integration tests against SQL Server
**Target Platform**: macOS (primary), Linux, Windows — cross-platform binary
**Project Type**: Single CLI application
**Performance Goals**: Memory usage during large result sets must not exceed ODBC baseline by >20% (SC-006)
**Constraints**: Binary size increase <50% vs current (SC-007); no external driver dependencies on any platform
**Scale/Scope**: ~1 file rewritten (`sqlserver.rs`), ~2 files updated (`Cargo.toml`, dev-dep test files), ~1 new test module

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Research Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity First** | PASS | Replacing ODBC with native TDS simplifies the dependency chain and user experience. Internal complexity is comparable. |
| **II. Engineering Fundamentals** | PASS | Single Responsibility (backend module owns connection logic); Fail Fast (errors map to existing variants); DRY (shared `Backend` trait unchanged). |
| **III. Over-Engineering Guards** | PASS | No new abstractions introduced. Server address parsing is a single function, not a parser framework. DMV approach is direct, not over-abstracted. |
| **IV. TDD** | PASS | All new code (address parser, type mapping, value conversion) will have tests written first. |
| **V. Incremental Delivery** | PASS | Plan decomposes into small, independently testable units: address parsing → config building → connection → type metadata → value conversion → streaming → integration. |

### Commit Discipline | PASS
Each task produces one commit: internally consistent, tests pass, codebase works.

### Post-Design Re-Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity First** | PASS | DMV pre-query adds one concept but avoids the far-more-complex alternative of forking tiberius. |
| **II. Engineering Fundamentals** | PASS | Separation of Concerns preserved: parsing, config, connection, type resolution, value conversion are separate functions. |
| **III. Over-Engineering Guards** | PASS | No new traits, no generic abstractions. Fallback type mapper is a simple match expression. |
| **IV. TDD** | PASS | Test plan covers all new functions with property-based edge cases. |
| **V. Incremental Delivery** | PASS | 7-8 tasks, each a single commit. |

## Project Structure

### Documentation (this feature)

```text
specs/007-tiberius-mssql/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── backend-trait.md
│   ├── type-normalization.md
│   └── server-address-parsing.md
└── tasks.md             # Phase 2 output (NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs                  # No changes
├── lib.rs                   # No changes
├── cli.rs                   # No changes
├── config.rs                # No changes
├── error.rs                 # No changes
├── validation.rs            # No changes
├── backend/
│   ├── mod.rs               # No changes (trait + shared types)
│   ├── sqlserver.rs          # REWRITTEN: tiberius implementation
│   └── databricks.rs        # No changes
├── format.rs                # No changes
├── format_csv.rs            # No changes
├── format_parquet.rs        # No changes
├── format_arrow.rs          # No changes
├── format_columnar.rs       # No changes
├── format_detect.rs         # No changes
├── output.rs                # No changes
├── masking.rs               # No changes
├── verbose.rs               # No changes
└── update.rs                # No changes

tests/
└── unit/
    ├── sqlserver_test.rs     # NEW: address parsing, type mapping, value conversion tests
    ├── config_test.rs        # UPDATE: remove odbc-api dev-dep references if any
    └── [all others]          # No changes
```

**Structure Decision**: Existing single-project structure. Only `src/backend/sqlserver.rs` is rewritten. One new test file added. The blast radius is intentionally minimal — all changes are behind the `Backend` trait boundary.

## Complexity Tracking

No constitution violations. No complexity justifications needed.

## Key Design Decisions

### 1. DMV-Based Column Type Resolution (FR-007)

**Why**: tiberius `ColumnType` enum lacks precision/scale/length. The `sys.dm_exec_describe_first_result_set` DMV returns `system_type_name` which is exactly the SQL type string we need (e.g., `nvarchar(255)`, `decimal(18,2)`).

**Trade-off**: Extra round-trip per query vs. forking tiberius to expose internal `TypeInfo`. The round-trip is metadata-only and negligible vs. query execution time.

**Fallback**: If DMV fails (permissions, unsupported query), fall back to `ColumnType`-based mapping without precision/scale. Emit diagnostic warning.

### 2. Server Address Parsing (FR-011)

**Why**: ODBC uses `Server=hostname,port` or `Server=hostname\INSTANCE` in connection strings. tiberius needs `config.host()`, `config.port()`, `config.instance_name()` as separate calls. A parser bridges the gap.

**Scope**: Single `parse_server_address` function. Not a general-purpose parser — handles exactly the 4 known formats.

### 3. Client-Side Timeout (FR-008)

**Why**: tiberius has no built-in timeout. `tokio::time::timeout` wraps the entire query+stream cycle.

**Trade-off**: Server continues executing after client timeout (vs. ODBC's server-side statement timeout). Acceptable because dbtoon creates one connection per query — dropping the client closes the TCP connection, which the server eventually notices.

### 4. Platform-Conditional Auth Features (FR-002, FR-003, FR-004)

**Why**: GSSAPI libraries are macOS/Linux-only; SSPI is Windows-only. Cargo features are conditionally enabled per target.

**Implementation**: `Cargo.toml` uses `[target.'cfg(...)'.dependencies]` to conditionally include `integrated-auth-gssapi` on non-Windows and `winauth` on Windows (tiberius default).

### 5. Value-to-String Conversion Parity (FR-007)

**Why**: ODBC's `as_text_view()` produces specific string formats for each type. The tiberius equivalent must match exactly, especially for:
- Decimal: trailing zeros preserved
- DateTime: specific format with fractional seconds
- Bit: `0`/`1` not `true`/`false`

**Risk**: This is the highest-risk area. Requires exhaustive test coverage comparing ODBC output format with tiberius output format for all type variants.

# Implementation Plan: Add Column Types to Output Metadata

**Branch**: `003-column-type-metadata` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-column-type-metadata/spec.md`

## Summary

Add SQL column type metadata to every query output. The output format changes from a bare TOON tabular array to a root object containing a `types` array (standard SQL type strings) and a `rows` array (existing tabular data). SQL Server types are normalized from ODBC debug format to standard SQL notation via exhaustive enum matching. Databricks types pass through unchanged. Zero-row results include types.

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `odbc-api` 20 (SQL Server ODBC — source of `DataType` enum), `toon-format` 0.4 (output encoding), `serde_json` 1 (intermediate JSON representation)
**Storage**: N/A
**Testing**: `cargo test` (unit tests in `tests/unit/`)
**Target Platform**: CLI (cross-platform)
**Project Type**: Single project
**Performance Goals**: N/A — metadata extraction adds negligible overhead to query execution
**Constraints**: Output must remain valid TOON (FR-004)
**Scale/Scope**: 3 files modified, ~1 new function, ~60 lines of new logic

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| **I. Simplicity First** | PASS | Single normalization function + one output structure change. No new abstractions. |
| **II. Engineering Fundamentals** | PASS | DRY: one normalization function called from one place. SRP: type normalization is its own function. Explicit: type mapping is a visible match expression. Meaningful names: `normalize_odbc_type()`. |
| **III. Over-Engineering Guards** | PASS | No abstractions (Rule of Three not triggered). No indirection layers. Approach is the minimum needed. |
| **IV. TDD** | PASS | Tests written before implementation. Type normalization tests cover all variants. Format tests updated to expect new structure. |
| **V. Incremental Delivery** | PASS | Natural task ordering: (1) normalization function + tests, (2) wire into backend, (3) output structure change + tests. Each is a minimum viable unit. |
| **Commit Discipline** | PASS | Each task = one commit. Tests first within each commit. |

**Gate result**: PASS — no violations.

### Post-Phase 1 Re-check

| Principle | Status | Notes |
|---|---|---|
| **I. Simplicity First** | PASS | Design uses direct enum matching (simplest approach). Output is a standard JSON object. |
| **II. Engineering Fundamentals** | PASS | `normalize_odbc_type()` has single responsibility. `to_toon()` changes are localized. No coupling added between modules. |
| **III. Over-Engineering Guards** | PASS | No new traits, no generics, no abstraction layers. The normalization function is a plain `match` expression. |
| **IV. TDD** | PASS | All changes have corresponding test-first workflow. |
| **V. Incremental Delivery** | PASS | 3 natural increments, each independently committable. |

**Post-design gate result**: PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/003-column-type-metadata/
├── plan.md              # This file
├── research.md          # Phase 0: TOON format, ODBC types, output impact
├── data-model.md        # Phase 1: Entity definitions, output structure
├── quickstart.md        # Phase 1: Implementation overview
├── contracts/
│   └── output-format.md # Phase 1: Output format contract
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── backend/
│   ├── mod.rs           # ColumnMeta, QueryResult (no changes)
│   ├── sqlserver.rs     # Add normalize_odbc_type(); use it at line 126
│   └── databricks.rs    # No changes (types already standard)
├── format.rs            # Update to_toon() for root object output
├── cli.rs               # No changes
├── config.rs            # No changes
├── error.rs             # No changes
├── masking.rs           # No changes
├── output.rs            # No changes
├── validation.rs        # No changes
├── verbose.rs           # No changes
├── lib.rs               # No changes
└── main.rs              # No changes

tests/
└── unit/
    ├── mod.rs            # No changes
    └── format_test.rs    # Update all format tests for new output structure
```

**Structure Decision**: Existing single-project structure. No new files or modules. Changes are localized to 2 source files and 1 test file.

## Complexity Tracking

No violations to justify. All changes use the simplest possible approach.

# Implementation Plan: Multiple Output File Formats

**Branch**: `004-output-formats` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/004-output-formats/spec.md`

## Summary

Add CSV, Parquet, and Arrow IPC output format support to dbtoon, detected by file extension on the existing `--output` / `-o` flag. CSV is unconditional (P1). Parquet and Arrow IPC are conditional on the type-mapping glue code staying under 200 LOC — research confirms ~120-150 LOC, so all three formats are included. The existing TOON default behavior is preserved with zero breaking changes.

## Technical Context

**Language/Version**: Rust (stable 1.91.1, 2024 edition)
**Primary Dependencies**: `csv` 1.4 (CSV writing), `arrow` 57 (Arrow arrays + IPC writer), `parquet` 57 (Parquet writer); existing: `toon-format` 0.4, `odbc-api` 20, `serde_json` 1, `clap` 4.5
**Storage**: File output (CSV, Parquet, Arrow IPC, TOON); no database changes
**Testing**: `cargo test` (unit tests in `tests/unit/`)
**Target Platform**: macOS/Linux CLI (same as existing)
**Project Type**: Single project (Rust binary crate)
**Performance Goals**: N/A — output formatting is not a bottleneck for typical query result sizes
**Constraints**: Parquet/Arrow type-mapping glue code must be <200 LOC (FR-006 feasibility gate)
**Scale/Scope**: 3 new output formats, ~5 modified/new source files, ~300-400 LOC net addition

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | Format detected from file extension (no new flags). Writer functions are independent and straightforward. |
| II. Engineering Fundamentals — SoC | PASS | Format detection, type mapping, and format writing are separate concerns in separate modules. |
| II. Engineering Fundamentals — YAGNI | PASS | Only formats explicitly requested. No configurable delimiters, compression settings, or schema evolution. |
| II. Engineering Fundamentals — DRY | PASS | Parquet and Arrow share the same type-mapping and array-building code. CSV has its own path (different concern). |
| II. Engineering Fundamentals — Dependency Inversion | PASS | A format dispatch trait/enum abstracts over concrete writers. `output_result()` dispatches by enum, not by direct coupling. |
| III. Over-Engineering Guards — Rule of Three | WATCH | Three output formats (CSV, Parquet, Arrow) plus existing TOON = 4 formats. A `WriterFormat` enum is justified at this count. No premature trait abstractions. |
| IV. TDD | PASS | Tests will be written before implementation for each format writer. |
| V. Incremental Delivery | PASS | Feature decomposes into independent deliverables: format detection, CSV writer, type mapping, Parquet writer, Arrow writer. Each can be committed independently. |

**Pre-design verdict**: PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/004-output-formats/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── output-format-contract.md
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Modified: output_result() dispatches by format
├── lib.rs               # Modified: add `format_detect` module
├── format.rs            # Existing: TOON formatting (unchanged)
├── format_detect.rs     # NEW: file extension → OutputFormat enum, path normalization
├── format_csv.rs        # NEW: CSV writer (csv crate)
├── format_columnar.rs   # NEW: shared Arrow type mapping + array building (Parquet & Arrow)
├── format_parquet.rs    # NEW: Parquet writer (parquet crate, uses format_columnar)
├── format_arrow.rs      # NEW: Arrow IPC writer (arrow-ipc, uses format_columnar)
├── output.rs            # Unchanged (format writers write directly via their crate APIs)
├── error.rs             # Modified: add arrow/parquet error variants
├── cli.rs               # Unchanged
├── config.rs            # Unchanged
├── backend/             # Unchanged
│   ├── mod.rs
│   ├── sqlserver.rs
│   └── databricks.rs
└── ...

tests/
└── unit/
    ├── mod.rs                 # Modified: register new test modules
    ├── format_detect_test.rs  # NEW: extension detection tests
    ├── format_csv_test.rs     # NEW: CSV output tests
    ├── format_columnar_test.rs # NEW: type mapping + array building tests
    ├── format_parquet_test.rs # NEW: Parquet output tests
    ├── format_arrow_test.rs   # NEW: Arrow IPC output tests
    └── ...                    # Existing test files unchanged
```

**Structure Decision**: Single project structure (existing). New format modules are flat in `src/` following the existing pattern (`format.rs`, `output.rs`, `validation.rs`). No subdirectory needed — the `format_` prefix groups them logically while keeping the module tree shallow.

## Constitution Re-Check (Post-Design)

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | No new CLI flags. Format detection is a 30-line function. Each writer is self-contained. |
| II. DRY | PASS | `format_columnar.rs` shared between Parquet and Arrow — no duplication of type mapping. |
| II. Separation of Concerns | PASS | Detection, type mapping, and writing are in separate modules. |
| III. Rule of Three | PASS | `format_columnar.rs` abstraction is justified (used by 2 callers: Parquet + Arrow). Format dispatch enum is justified (4 variants). |
| IV. TDD | PASS | Each module has a corresponding test file written before implementation. |
| V. Incremental Delivery | PASS | 5 independently committable units identified (see tasks.md). |

**Post-design verdict**: PASS — no violations. No complexity tracking entries needed.

## Complexity Tracking

> No violations to justify.

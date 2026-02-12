# Implementation Plan: Truncation Metadata

**Branch**: `005-truncation-metadata` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-truncation-metadata/spec.md`

## Summary

Embed truncation metadata (`truncated`, `message`) directly into every output format's native data channel, eliminating out-of-band truncation signals on stdout. TOON output (stdout and file) gets root-level `"truncated"` and `"message"` keys. Parquet and Arrow IPC files get namespaced key-value metadata (`dbtoon:truncated`, `dbtoon:message`). The print summary for all file outputs becomes a valid TOON object (replacing the ad-hoc `key: value` format). A stderr warning replaces the old stdout truncation message for interactive visibility.

## Technical Context

**Language/Version**: Rust (stable 1.91.1, 2024 edition)
**Primary Dependencies**: Existing: `toon-format` 0.4 (TOON encoding), `arrow` 57 (Arrow schema metadata + IPC), `parquet` 57 (Parquet writer), `serde_json` 1 (JSON construction), `csv` 1.4; no new dependencies
**Storage**: File output (TOON, CSV, Parquet, Arrow IPC); no database changes
**Testing**: `cargo test` (unit tests in `tests/unit/`)
**Target Platform**: macOS/Linux CLI (same as existing)
**Project Type**: Single project (Rust binary crate)
**Performance Goals**: N/A — metadata addition is trivial overhead
**Constraints**: No breaking changes to existing CLI interface; stdout must contain only valid TOON
**Scale/Scope**: ~6 modified source files, ~100-150 LOC net change (mostly replacement of existing code)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | No new flags, no new modules. Truncation metadata embedded in existing output mechanisms. |
| II. Engineering Fundamentals — SoC | PASS | Each format writer handles its own metadata embedding. Message construction is centralized in the output dispatch. |
| II. Engineering Fundamentals — YAGNI | PASS | Only the metadata explicitly specified (truncated + message). No configurable metadata keys, no extensible metadata framework. |
| II. Engineering Fundamentals — DRY | PASS | Schema metadata helper shared between Parquet and Arrow (same knowledge: `dbtoon:` key prefix and insertion logic). Message construction done once in `output_result()`. |
| II. Engineering Fundamentals — Least Surprise | PASS | Truncation metadata appears where users expect data — in the output itself, not in a side channel. |
| III. Over-Engineering Guards — Rule of Three | PASS | Shared schema metadata helper used by 2 callers (Parquet + Arrow). Justified as DRY on *knowledge* (same key names), not premature abstraction on *code shape*. |
| IV. TDD | PASS | Tests written before implementation for each changed module. |
| V. Incremental Delivery | PASS | Feature decomposes into independent committable units: TOON metadata, file metadata, print summary, stderr warning, cleanup. |

**Pre-design verdict**: PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/005-truncation-metadata/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── truncation-metadata-contract.md
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # Modified: output_result() builds message, passes truncation info, emits stderr warning
├── format.rs            # Modified: to_toon() accepts truncation info, embeds in root object; remove to_toon_kv()
├── output.rs            # Modified: print_summary() produces valid TOON; remove print_truncation_message(); add print_truncation_warning()
├── format_parquet.rs    # Modified: write_parquet() accepts truncation info, adds schema metadata
├── format_arrow.rs      # Modified: write_arrow() accepts truncation info, adds schema metadata
├── format_columnar.rs   # Modified: add with_truncation_metadata() helper for shared schema metadata logic
├── format_csv.rs        # Unchanged
├── format_detect.rs     # Unchanged
├── lib.rs               # Unchanged
├── cli.rs               # Unchanged
├── config.rs            # Unchanged
├── error.rs             # Unchanged
├── backend/             # Unchanged
│   ├── mod.rs
│   ├── sqlserver.rs
│   └── databricks.rs
└── ...

tests/
└── unit/
    ├── format_test.rs          # Modified: test truncation keys in TOON output
    ├── format_parquet_test.rs  # Modified: test dbtoon: metadata in Parquet files
    ├── format_arrow_test.rs    # Modified: test dbtoon: metadata in Arrow IPC files
    ├── output_test.rs          # NEW: test print_summary produces valid TOON; test stderr warning
    └── ...                     # Existing test files unchanged
```

**Structure Decision**: Single project structure (existing). No new modules — modifications to existing files only. One new test file (`output_test.rs`) for the print summary and warning functions, since these were previously untested.

## Constitution Re-Check (Post-Design)

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | No new CLI flags, no new modules, no new dependencies. Changes are additions to existing functions. |
| II. DRY | PASS | `with_truncation_metadata()` in `format_columnar.rs` shared between Parquet and Arrow. Message string constructed once in `output_result()`. |
| II. Separation of Concerns | PASS | Format writers handle their own metadata embedding. Output dispatch handles message construction and stderr. |
| II. Least Surprise | PASS | `truncated` key always present in TOON (avoids ambiguity with older versions). `message` only when truncated (keeps clean output clean). |
| III. Rule of Three | PASS | Shared metadata helper justified by DRY on knowledge (2 callers, same key names). |
| IV. TDD | PASS | Each change has corresponding test additions. |
| V. Incremental Delivery | PASS | 5 independently committable units: (1) TOON root keys, (2) Parquet/Arrow metadata, (3) valid TOON print summary, (4) stderr warning, (5) remove old functions. |

**Post-design verdict**: PASS — no violations. No complexity tracking entries needed.

## Complexity Tracking

> No violations to justify.

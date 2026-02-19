# Implementation Plan: Rewrite Query Validation as Deny-List with Safe EXEC Allowlist

> **STATUS: COMPLETE** | Merged: 2026-02-19 | Branch: `008-write-query-detection`

**Branch**: `008-write-query-detection` | **Date**: 2026-02-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/008-write-query-detection/spec.md`

## Summary

Replace the current allowlist-based query validation (which only permits SELECT, EXPLAIN, SHOW, USE — causing false positives on legitimate read-only patterns like SET, DECLARE, and transaction wrappers) with a deny-list approach that explicitly blocks known write operations (DML, DDL, DCL, Ops, EXEC) and allows everything else. Additionally, introduce a compile-time allowlist of safe SQL Server system stored procedures that may be invoked via EXEC.

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `sqlparser` 0.61 (SQL parsing + AST — already integrated), `clap` 4.5, `thiserror` 2, `anyhow` 1
**Storage**: N/A — pure validation logic, no persistence changes
**Testing**: `cargo test` (unit tests in `tests/unit/validation_test.rs`)
**Target Platform**: Cross-platform CLI (Linux, macOS, Windows)
**Project Type**: Single Rust binary
**Performance Goals**: N/A — validation is synchronous, single-query, already sub-millisecond
**Constraints**: Zero changes to public API (`validate()`, `ValidationResult`, `BackendDialect` signatures preserved per FR-014)
**Scale/Scope**: ~200 lines of validation logic to rewrite; ~220 lines of tests to extend

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | Deny-list is simpler to reason about than allowlist for this domain — "what's dangerous?" is a smaller, more stable set than "what's safe?" |
| II. Engineering Fundamentals | PASS | Single module, single concern (validation), loose coupling (no dependencies on backend execution), explicit deny reasons |
| III. Over-Engineering Guards | PASS | No new abstractions — same function signature, same module. Compile-time allowlist avoids config complexity. No premature generalization across dialects. |
| IV. TDD (NON-NEGOTIABLE) | PASS | Tests already exist; will write new tests first for each category, then flip implementation |
| V. Incremental Delivery | PASS | Natural decomposition: (1) deny-list core, (2) safe EXEC allowlist, (3) enriched denial messages — each independently testable and committable |

No violations. No complexity justification needed.

## Project Structure

### Documentation (this feature)

```text
specs/008-write-query-detection/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output (type/enum design)
├── quickstart.md        # Phase 1 output (implementation guide)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
└── validation.rs        # Primary file — rewrite internals, preserve public API

tests/
└── unit/
    └── validation_test.rs  # Extend with new safe/denied test cases
```

**Structure Decision**: Single-file change in `src/validation.rs` with extended tests in the existing test file. No new modules, files, or directories needed. The validation module is already cleanly separated from the rest of the codebase.

## Complexity Tracking

> No Constitution Check violations. Table intentionally left empty.

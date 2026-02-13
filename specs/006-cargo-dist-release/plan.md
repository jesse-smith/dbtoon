# Implementation Plan: Cross-Platform Binary Distribution & Self-Update

> **STATUS: COMPLETE** | Merged: 2026-02-13 | Branch: `006-cargo-dist-release`

**Branch**: `006-cargo-dist-release` | **Date**: 2026-02-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/006-cargo-dist-release/spec.md`

## Summary

Set up cargo-dist to build cross-platform binaries (Windows x64, Linux x64, macOS ARM64, macOS x64) and publish them to GitHub Releases on version tag push. Add a `dbtoon update` subcommand using axoupdater for self-update from GitHub Releases. Update README with install and update instructions.

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: Existing (`clap` 4.5, `tokio` 1, `anyhow` 1, `thiserror` 2) + New (`axoupdater` 0.9 for self-update); cargo-dist 0.30.3 (build tooling, not a runtime dep)
**Storage**: N/A (install receipts managed by cargo-dist installer, not by dbtoon)
**Testing**: `cargo test` (unit tests for update command error paths; release workflow validated by `cargo dist plan`)
**Target Platform**: Cross-platform CLI — Windows x64, Linux x64, macOS ARM64, macOS x64
**Project Type**: Single project (existing structure)
**Performance Goals**: N/A (installer downloads and updates are network-bound, not CPU-bound)
**Constraints**: Update must complete in <30 seconds on reasonable internet (SC-003); install must complete in <60 seconds (SC-001)
**Scale/Scope**: 4 target platforms, 1 new subcommand, 1 new CI workflow, README updates

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-research check

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | cargo-dist is the simplest path to cross-platform distribution — one config file, one generated workflow. The `update` subcommand is a thin wrapper around axoupdater. |
| II. Engineering Fundamentals — YAGNI | PASS | Only building what the spec requires: 4 targets, 2 installers, 1 update command. No Homebrew formula, MSI installer, or npm wrapper. |
| II. Engineering Fundamentals — Separation of Concerns | PASS | Update logic is isolated in its own module (`src/update.rs`). Release workflow is a separate file from CI. |
| II. Engineering Fundamentals — Fail Fast | PASS | Update command checks for receipt first, then network, then version — fails at each stage with a clear message. |
| III. Over-Engineering Guards — Rule of Three | PASS | No premature abstractions. The update module has one public function. |
| III. Over-Engineering Guards — Reversibility | PASS | cargo-dist config is a single file; the generated workflow can be regenerated. axoupdater is one dependency that can be removed if approach changes. |
| IV. TDD | PASS | Unit tests for the update command's error paths (no receipt, already current). The release workflow is validated by `cargo dist plan` in CI. |
| V. Incremental Delivery | PASS | Natural task decomposition: (1) cargo-dist setup, (2) update subcommand, (3) README — each independently committable. |
| Commit Discipline | PASS | Each task produces a self-contained, testable commit. |

### Post-design re-check

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Simplicity First | PASS | No custom build scripts, no manual GitHub Actions matrix. cargo-dist generates everything. axoupdater is a single function call. |
| II. Engineering Fundamentals — DRY | PASS | Version is defined once in `Cargo.toml`; cargo-dist reads it for releases, `env!("CARGO_PKG_VERSION")` provides it at runtime. |
| II. Engineering Fundamentals — Least Surprise | PASS | `dbtoon update` behaves like other CLI update commands. Installer places binary in `~/.cargo/bin/` which is standard for Rust tools. |
| III. Over-Engineering Guards | PASS | No abstraction layers, no feature flags, no configuration options for the update command. |
| IV. TDD | PASS | Update module is testable: `AxoUpdater` methods can be tested for error handling without network calls (receipt loading is file-based). |

**GATE RESULT: PASS** — No violations.

## Project Structure

### Documentation (this feature)

```text
specs/006-cargo-dist-release/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── cli-update.md    # Update subcommand contract
│   └── release-workflow.md  # Release CI contract
└── tasks.md             # Phase 2 output (NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
# Files modified
Cargo.toml                          # +axoupdater dep, +[profile.dist]
src/cli.rs                          # +Update variant in Command enum
src/main.rs                         # +match arm for Update command
README.md                           # +install/update instructions

# Files added
src/update.rs                       # Update subcommand implementation
dist-workspace.toml                 # cargo-dist configuration (generated by cargo dist init)
.github/workflows/release.yml       # Release workflow (generated by cargo dist generate)
```

**Structure Decision**: Existing single-project layout. One new module (`src/update.rs`) following the existing pattern of one file per concern. No new directories needed. The `dist-workspace.toml` and `release.yml` are generated by cargo-dist tooling, not hand-written.

## Complexity Tracking

> No violations to justify — all principles pass.

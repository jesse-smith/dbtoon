# Implementation Plan: Simplify CLI Interface

> **STATUS: COMPLETE** | Merged: 2026-02-19 | Branch: `009-simplify-cli-ui`

**Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/009-simplify-cli-ui/spec.md`

## Summary

Restructure the dbtoon CLI to separate connection management from query execution. Replace `exec-read`/`exec-write` with a unified `query` command requiring `-P <profile>`, add `profile` management subcommands (create/edit/show/list/test/delete/rename), add `dbtoon init` for config bootstrapping, and remove all `DBTOON_*` environment variables in favor of `$VAR` references within TOML profiles and Databricks standard env vars as lowest-priority fallbacks.

**Key technical decisions** (from [research.md](research.md)):
- `toml_edit` 0.25 for format-preserving config writes
- Simple `resolve_env_var()` function for `$VAR`/`$$` resolution (~15 lines)
- `HOME`-based config path instead of `directories` crate (force `~/.config/dbtoon/` on macOS)

## Technical Context

**Language/Version**: Rust (stable, 2024 edition)
**Primary Dependencies**: `clap` 4.5 (CLI), `toml` 0.8 (config read), `toml_edit` 0.25 (config write — NEW), `secrecy` 0.10 (masking), `serde` 1 (deserialization), `sqlparser` 0.61 (validation)
**Storage**: TOML config file at `~/.config/dbtoon/config.toml`; no database changes
**Testing**: `cargo test` (unit tests in `tests/unit/`)
**Target Platform**: macOS, Linux (cross-platform via `HOME` env var)
**Project Type**: Single Rust binary (CLI tool)
**Performance Goals**: N/A — config operations are sub-second
**Constraints**: Must preserve existing query execution behavior exactly; only the config/CLI surface changes
**Scale/Scope**: ~18 source files, ~5 new/modified test files

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Research Gate

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity First** | PASS | CLI restructure explicitly simplifies the user-facing interface (3 commands → clear subcommand hierarchy) |
| **II. Engineering Fundamentals** | PASS | DRY: unified `query` replaces duplicated `exec-read`/`exec-write`. SoC: profile management separated from query execution. Least Surprise: `-P` profile pattern is conventional. |
| **III. Over-Engineering Guards** | PASS | No premature abstractions. `$VAR` resolution is a 15-line function. Profile CRUD uses `toml_edit` directly. |
| **IV. TDD** | GATE | Tests must be written before implementation for each unit. |
| **V. Incremental Delivery** | GATE | Each task must produce a compilable, test-passing commit. |

### Post-Design Re-Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Simplicity** | PASS | Data model has 2 profile types with small, well-defined field sets. No abstraction layers. |
| **II. Fundamentals** | PASS | Single `resolve_env_var()` for all `$VAR` fields (DRY). Config resolution hierarchy is explicit with clear precedence (Explicit > Implicit). |
| **III. Guards** | PASS | No new abstractions beyond what the spec requires. `toml_edit` is used directly, not wrapped. |
| **IV. TDD** | PASS | Testing strategy defined: unit tests for resolution/validation, integration tests for CLI parsing and config round-trips. |
| **V. Incremental** | PASS | Implementation order in quickstart.md defines 11 independently deliverable steps. |

## Project Structure

### Documentation (this feature)

```text
specs/009-simplify-cli-ui/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── cli-interface.md # Complete CLI contract
│   └── config-file.md   # Config file schema contract
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── cli.rs               # MODIFY: Replace Command enum, new QueryArgs/ProfileCommand/etc.
├── config.rs            # MODIFY: $VAR resolution, HOME-based path, remove directories dep
├── init.rs              # NEW: dbtoon init command logic
├── profile.rs           # NEW: Profile CRUD via toml_edit
├── main.rs              # MODIFY: New command dispatch
├── error.rs             # UNCHANGED
├── lib.rs               # MODIFY: Add pub mod init, profile
├── backend/             # UNCHANGED
├── format*.rs           # UNCHANGED
├── masking.rs           # UNCHANGED
├── output.rs            # UNCHANGED
├── update.rs            # UNCHANGED
├── validation.rs        # UNCHANGED
└── verbose.rs           # UNCHANGED

tests/
└── unit/
    ├── config_test.rs         # MODIFY: $VAR resolution tests, config path tests
    ├── init_test.rs           # NEW: init command tests
    ├── profile_test.rs        # NEW: profile CRUD tests
    ├── cli_test.rs            # NEW: CLI argument parsing tests
    └── ... (existing unchanged)
```

**Structure Decision**: Single project structure (existing). Two new source modules (`init.rs`, `profile.rs`) and corresponding test files. No structural reorganization — the flat `src/` layout matches the existing codebase.

## Complexity Tracking

No constitution violations to justify. The design uses:
- One new dependency (`toml_edit`) — necessary for comment-preserving TOML edits
- Two new source modules — each addresses a single concern (init, profile management)
- One removed dependency (`directories`) — replaced by simpler `HOME`-based path

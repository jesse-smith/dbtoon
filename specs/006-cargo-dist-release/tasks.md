# Tasks: Cross-Platform Binary Distribution & Self-Update

> **STATUS: COMPLETE** | Merged: 2026-02-13 | Branch: `006-cargo-dist-release`

**Input**: Design documents from `/specs/006-cargo-dist-release/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/, quickstart.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize cargo-dist tooling and add the new runtime dependency

- [x] T001 Run `cargo dist init` selecting V1 config format (dist-workspace.toml), GitHub CI provider, 4 targets (x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-apple-darwin), and shell + powershell installers — creates dist-workspace.toml, .github/workflows/release.yml, and adds `[profile.dist]` to Cargo.toml. After init, verify `install-path = "CARGO_HOME"` is set in dist-workspace.toml (per research R-002)
- [x] T002 Add `axoupdater = { version = "0.9", default-features = false, features = ["github_releases", "blocking"] }` to `[dependencies]` in Cargo.toml

---

## Phase 2: US1/US2 — Release Pipeline & Installation (Priority: P1) 🎯 MVP

**Goal**: Automated CI pipeline that builds cross-platform binaries and publishes them with installer scripts to GitHub Releases on version tag push. Delivers both **User Story 1** (install from release) and **User Story 2** (automated release pipeline) — these are inseparable because the pipeline produces the installers.

**Independent Test**: Push a version tag and verify a GitHub Release appears with archives for all 4 targets plus shell and PowerShell installer scripts. Run the shell installer on a clean machine and confirm `dbtoon --version` works.

### Implementation for US1/US2

- [x] T003 [US2] Add ODBC system build dependencies (`[dist.dependencies.apt]` unixodbc-dev = { stage = ["build"] }, `[dist.dependencies.homebrew]` unixodbc = { stage = ["build"] }) to dist-workspace.toml
- [x] T004 [US2] Regenerate release workflow to apply configuration changes via `cargo dist generate` in .github/workflows/release.yml
- [x] T005 [US1] Validate release plan produces all 4 target artifacts and 2 installer scripts via `cargo dist plan`. Also verify `pr-run-mode = "plan"` is set in dist-workspace.toml to ensure the release workflow only runs a lightweight check on PRs (FR-010)

**Checkpoint**: At this point, pushing a version tag should build and publish binaries for all 4 platforms with working installer scripts. Both US1 and US2 are delivered.

---

## Phase 3: US3 — Self-Update Installed Binary (Priority: P2)

**Goal**: `dbtoon update` subcommand that checks for newer releases via axoupdater, downloads and installs updates, and handles all error cases per the CLI contract (contracts/cli-update.md).

**Independent Test**: Install an older version via the shell installer, run `dbtoon update`, and verify `dbtoon --version` shows the newer version. Also test: running when already current (reports up to date), and running a cargo-install'd binary (reports not installed via installer).

### Implementation for US3

- [x] T006 [P] [US3] Define `pub fn run_update() -> Result<()>` interface with `todo!()` stub and write failing unit tests for no-receipt and already-current error paths in src/update.rs (`#[cfg(test)]` module) — RED phase
- [x] T007 [P] [US3] Add `Update` variant with `/// Update dbtoon to the latest release` doc comment to Command enum in src/cli.rs
- [x] T008 [US3] Implement update logic: receipt loading, version check, and self-update execution per contracts/cli-update.md behavior table (all 7 conditions) in src/update.rs — tests from T006 must pass (GREEN phase). Edge cases 2–3 from spec (permissions error, partial release) are handled by axoupdater/cargo-dist, not dbtoon code.
- [x] T009 [US3] Add match arm for `Command::Update` calling `update::run_update()` (no config/backend needed) in src/main.rs

**Checkpoint**: `dbtoon update` works for all 7 conditions in the CLI contract. Unit tests pass for error paths.

---

## Phase 4: US4 — Install & Update Documentation (Priority: P3)

**Goal**: README contains clear, copy-pasteable install commands for all platforms and documents the `dbtoon update` command.

**Independent Test**: Read the README and follow the documented commands on each platform.

### Implementation for US4

- [x] T010 [US4] Add Installation section with platform-specific one-liner commands (shell installer for macOS/Linux, PowerShell installer for Windows) to README.md
- [x] T011 [US4] Add Updating section documenting `dbtoon update` usage and behavior to README.md

**Checkpoint**: README has complete install and update instructions for all platforms.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all stories

- [x] T012 Run full validation: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo dist plan`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **US1/US2 (Phase 2)**: Depends on T001 (cargo-dist must be initialized)
- **US3 (Phase 3)**: Depends on T002 (axoupdater dependency must be added)
- **US4 (Phase 4)**: Depends on Phase 2 (need installer URLs) and Phase 3 (need update command to document)
- **Polish (Phase 5)**: Depends on all previous phases

### Within-Phase Dependencies

- **Phase 1**: T001 → T002 (both modify Cargo.toml — T001 adds `[profile.dist]`, T002 adds axoupdater dep)
- **Phase 2**: T003 → T004 → T005 (configure → regenerate → validate)
- **Phase 3**: T006 ∥ T007 (parallel, different files) → T008 (implements logic, tests from T006 must pass) → T009 (wires command in main.rs)
- **Phase 4**: T010 → T011 (same file, install section before update section)
- **Phase 5**: T012 (runs after everything)

### Parallel Opportunities

- T006 and T007 can run in parallel (src/update.rs and src/cli.rs are independent files)
- Phases 2 and 3 can overlap: Phase 3 only needs T002 (axoupdater dep), not Phase 2 completion. If two developers are available:
  - Developer A: Phase 2 (T003 → T004 → T005)
  - Developer B: Phase 3 (T006 ∥ T007 → T008 → T009)

---

## Parallel Example: US3

```bash
# Launch T006 and T007 in parallel (different files, no dependencies):
Task: "Define interface + write failing tests in src/update.rs" (RED)
Task: "Add Update variant to Command enum in src/cli.rs"

# Then sequentially:
Task: "Implement update logic in src/update.rs" (GREEN — tests from T006 must pass)
Task: "Wire Update command in src/main.rs" (depends on T007, T008)
```

---

## Implementation Strategy

### MVP First (US1 + US2 Only)

1. Complete Phase 1: Setup (T001–T002)
2. Complete Phase 2: US1/US2 (T003–T005)
3. **STOP and VALIDATE**: Push a test tag, verify release artifacts appear on GitHub
4. Deploy/demo if ready — team can now install dbtoon without Rust

### Incremental Delivery

1. Setup + US1/US2 → Release pipeline works, installers available (MVP!)
2. Add US3 → `dbtoon update` works → Users can self-update
3. Add US4 → README documents everything → Onboarding complete
4. Polish → Full test/lint/plan validation

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- The `cargo dist init` step (T001) is interactive — see quickstart.md for exact prompts
- Generated files (dist-workspace.toml, release.yml) should not be hand-edited after `cargo dist generate` except for the ODBC dependency block
- The update module (T008) must handle all 7 conditions from contracts/cli-update.md
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently

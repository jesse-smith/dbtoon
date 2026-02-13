# Research: Cross-Platform Binary Distribution & Self-Update

**Feature**: 006-cargo-dist-release
**Date**: 2026-02-12

## R-001: cargo-dist Setup & Configuration

**Decision**: Use cargo-dist v0.30.3 with V1 config format (`dist-workspace.toml`)

**Rationale**: cargo-dist is the canonical Rust binary distribution tool. V1 config format (`dist-workspace.toml`) is the recommended modern approach over the legacy `[workspace.metadata.dist]` in Cargo.toml. It generates GitHub Actions workflows, platform-specific builds, and installer scripts with minimal configuration.

**Alternatives considered**:
- Manual GitHub Actions with `cross` / `cargo build --target` — much more setup, no installer generation, no install receipts
- `cargo-release` — handles version bumping and tagging but not cross-platform builds or installer generation
- Goreleaser-style custom scripts — not Rust-native, no ecosystem integration

**Key findings**:
- `cargo dist init` creates `dist-workspace.toml`, adds `[profile.dist]` to Cargo.toml, and generates `.github/workflows/release.yml`
- The release workflow is triggered by pushing semver tags (e.g., `v0.2.0`)
- It automatically maps targets to CI runners (Windows → `windows-2022`, Linux → `ubuntu-22.04`, macOS ARM → `macos-14`, macOS Intel → `macos-13`)
- With `pr-run-mode = "plan"`, it runs a lightweight check on PRs without building — no conflict with the existing CI workflow

## R-002: Installer Scripts

**Decision**: Use both `shell` and `powershell` installers with `install-path = "CARGO_HOME"`

**Rationale**: These are the two installer types cargo-dist generates. The shell installer covers macOS and Linux; the PowerShell installer covers Windows. Both auto-detect platform/architecture and modify PATH automatically.

**Alternatives considered**:
- Homebrew formula — additional maintenance, not needed for initial distribution
- MSI installer — heavier, not needed for CLI tool
- npm wrapper — unnecessary indirection

**Key findings**:
- Shell installer: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.sh | sh`
- PowerShell installer: `powershell -ExecutionPolicy Bypass -c "irm https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.ps1 | iex"`
- Both installers place the binary in `~/.cargo/bin/` and add it to PATH if not already present
- Installers write an install receipt JSON used by the self-update mechanism

## R-003: ODBC System Dependencies in CI

**Decision**: Declare system build dependencies via `[dist.dependencies.apt]` and `[dist.dependencies.homebrew]` (dynamic linking, the default)

**Rationale**: `odbc-api` dynamically links against the system ODBC library. Windows has ODBC built-in (`odbc32.lib`), but Linux needs `unixodbc-dev` and macOS needs `unixodbc` (via Homebrew) at build time. cargo-dist supports declarative system dependencies, which is the cleanest approach.

**Alternatives considered**:
- `vendored-unix-odbc` feature (static linking) — avoids system dep requirements but introduces LGPL-2.1+ license compliance obligations (must provide object files for relinking); unacceptable license burden for a distributed binary
- `github-build-setup` custom script — works but less idiomatic than declarative deps

**Key findings**:
- Windows: no action needed (ODBC is built-in)
- Linux: `[dist.dependencies.apt] unixodbc-dev = { stage = ["build"] }`
- macOS: `[dist.dependencies.homebrew] unixodbc = { stage = ["build"] }`
- Runtime: users who need SQL Server already need ODBC drivers installed, which install the driver manager too
- The spec already documents this as an out-of-scope prerequisite

## R-004: Self-Update Mechanism

**Decision**: Use `axoupdater` 0.9 with `github_releases` and `blocking` features

**Rationale**: axoupdater is the official companion to cargo-dist, built by the same team. It reads the install receipt written by cargo-dist's installers to determine the current version and update source. It updates by downloading and running the same installer scripts, which means platform detection and binary replacement are handled by proven, tested code.

**Alternatives considered**:
- `self_update` crate — general-purpose but requires hardcoding repo coordinates, manual platform matching, and custom asset naming patterns; no install receipt awareness; doesn't detect `cargo install` users
- Manual implementation via reqwest + GitHub API — maximum control but significant effort to handle platform detection, archive extraction, binary replacement on Windows, and all error cases

**Key findings**:
- axoupdater reads install receipts at `~/.config/dbtoon/dbtoon-receipt.json` (Linux/macOS) or `%LOCALAPPDATA%\dbtoon\dbtoon-receipt.json` (Windows)
- `load_receipt()` returns `NoReceipt` error when binary was installed via `cargo install` — clean detection
- `check_receipt_is_for_this_executable()` guards against mismatched install paths
- `run_sync()` returns `Ok(Some(result))` on update, `Ok(None)` when already current
- Network errors surface as `AxoupdateError::Reqwest`
- The `blocking` feature provides sync wrappers so the update subcommand doesn't need special async handling

## R-005: Subcommand Naming

**Decision**: Name the subcommand `update` (not `self-update`)

**Rationale**: The spec explicitly says `dbtoon update`. This is shorter and matches user expectations. There is no naming conflict with existing subcommands (`exec-read`, `exec-write`, `list-warehouses`).

**Alternatives considered**:
- `self-update` — more explicit but longer; spec says `update`
- `upgrade` — valid synonym but spec says `update`

## R-006: Release Workflow vs Existing CI

**Decision**: Release workflow runs independently alongside existing CI; no changes to `ci.yml` needed

**Rationale**: The existing CI triggers on `push: branches: [main]` and `pull_request`. The release workflow triggers on `push: tags: **[0-9]+.[0-9]+.[0-9]+*`. Tag pushes do not trigger branch-based workflows. The only overlap is on `pull_request`, where the release workflow runs a lightweight `dist plan` check (no builds).

**Key findings**:
- FR-010 is satisfied by default — no interference
- The release workflow is fully generated by `cargo dist generate` and should not be hand-edited

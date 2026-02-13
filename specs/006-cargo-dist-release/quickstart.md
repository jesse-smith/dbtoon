# Quickstart: Cross-Platform Binary Distribution & Self-Update

**Feature**: 006-cargo-dist-release
**Date**: 2026-02-12

## Prerequisites

- Rust stable (2024 edition) toolchain
- cargo-dist v0.30.3: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/axodotdev/cargo-dist/releases/download/v0.30.3/cargo-dist-installer.sh | sh`

## Setup (one-time)

### 1. Initialize cargo-dist

```sh
cargo dist init
```

When prompted, select:
- Config format: V1 (`dist-workspace.toml`)
- CI: github
- Targets: `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`
- Installers: `shell`, `powershell`

This creates:
- `dist-workspace.toml` — cargo-dist configuration
- `.github/workflows/release.yml` — release CI workflow
- `[profile.dist]` section in `Cargo.toml`

### 2. Add system dependencies for ODBC

Edit `dist-workspace.toml` to add:

```toml
[dist.dependencies.apt]
unixodbc-dev = { stage = ["build"] }

[dist.dependencies.homebrew]
unixodbc = { stage = ["build"] }
```

Then regenerate the workflow:

```sh
cargo dist generate
```

### 3. Add axoupdater dependency

In `Cargo.toml`:

```toml
axoupdater = { version = "0.9", default-features = false, features = ["github_releases", "blocking"] }
```

### 4. Add the `update` subcommand

Add `Update` variant to the `Command` enum in `src/cli.rs`, implement the handler using `axoupdater::AxoUpdater`.

### 5. Update README

Add installation instructions (shell/PowerShell one-liners) and `dbtoon update` documentation.

## Development Workflow

### Test locally

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

### Validate release configuration

```sh
cargo dist plan
```

This shows what a release would produce without actually building.

### Build release artifacts locally (optional)

```sh
cargo dist build
```

### Create a release

```sh
# 1. Bump version in Cargo.toml
# 2. Commit
git add Cargo.toml Cargo.lock
git commit -m "release: v0.2.0"
# 3. Tag and push
git tag v0.2.0
git push origin main --tags
```

The tag push triggers the release workflow, which builds all 4 targets and publishes to GitHub Releases.

## Verification

### Install from release

```sh
# macOS/Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.sh | sh

# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/jesse-smith/dbtoon/releases/latest/download/dbtoon-installer.ps1 | iex"
```

### Verify installation

```sh
dbtoon --version
```

### Self-update

```sh
dbtoon update
```

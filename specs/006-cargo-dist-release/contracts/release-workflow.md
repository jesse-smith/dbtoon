# Contract: Release Workflow

## Trigger

Push a tag matching `**[0-9]+.[0-9]+.[0-9]+*` (e.g., `v0.2.0`).

## Outputs

A GitHub Release containing:

| Artifact | Platform | Format |
|----------|----------|--------|
| `dbtoon-x86_64-pc-windows-msvc.zip` | Windows 64-bit | zip |
| `dbtoon-x86_64-unknown-linux-gnu.tar.gz` | Linux 64-bit | tar.gz |
| `dbtoon-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon | tar.gz |
| `dbtoon-x86_64-apple-darwin.tar.gz` | macOS Intel | tar.gz |
| `dbtoon-installer.sh` | macOS + Linux | shell script |
| `dbtoon-installer.ps1` | Windows | PowerShell script |

## Workflow Stages

1. **Plan** — determines what to build from the tag and config
2. **Build local artifacts** — parallel matrix across 4 targets on platform-specific runners
3. **Build global artifacts** — generates installer scripts, checksums
4. **Host** — uploads all artifacts to the GitHub Release
5. **Announce** — final notification step

## Non-interference

- Existing `ci.yml` triggers on `push: branches: [main]` and `pull_request`
- `release.yml` triggers on `push: tags` and `pull_request` (plan-only)
- Tag pushes do NOT trigger `ci.yml` (branch filter excludes tags)
- PR events: both workflows run in parallel; release runs `dist plan` only (lightweight)

## System Dependencies (build-time only)

| Platform | Package |
|----------|---------|
| Linux (`ubuntu-22.04`) | `unixodbc-dev` (apt) |
| macOS (`macos-13`, `macos-14`) | `unixodbc` (homebrew) |
| Windows (`windows-2022`) | None (ODBC built-in) |

## Configuration

Managed by `dist-workspace.toml`. Workflow regenerated with `cargo dist generate`.

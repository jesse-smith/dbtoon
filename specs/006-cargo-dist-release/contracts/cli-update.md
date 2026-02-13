# Contract: `dbtoon update` Subcommand

## Interface

```
dbtoon update
```

No arguments. No flags beyond the existing global flags (`--verbose`, `--config`, `--show-secrets`).

## Behavior

| Condition | Output (stderr) | Exit Code |
|-----------|-----------------|-----------|
| Newer version available, update succeeds | `Checking for updates...\nUpdated dbtoon: 0.1.0 => 0.2.0` | 0 |
| Already up to date | `dbtoon v0.2.0 is already up to date.` | 0 |
| No install receipt (cargo install user) | `dbtoon was not installed via the shell/PowerShell installer.\nSelf-update is only available for installer-based installations.\nPlease update with: cargo install dbtoon` | 0 |
| Receipt exists but exe path mismatch | `This copy of dbtoon was not installed by the shell/PowerShell installer.\nPlease update with the method you originally used to install it.` | 0 |
| No internet / network error | `Error: unable to check for updates — are you connected to the internet?` | 1 |
| Installer execution failed | `Error: update failed: <details>` | 1 |
| No installer asset for platform | `Error: no installer found for your platform in the latest release.` | 1 |

## Design Decisions

- Output goes to **stderr** (not stdout) — consistent with status/diagnostic messages in the existing codebase
- Non-error cases (no receipt, already current) exit 0 — these are informational, not failures
- Network and installer failures exit 1 — these are actionable errors
- The `--verbose` global flag is accepted but has no additional effect for `update` (axoupdater handles its own logging)

## Dependencies

- `axoupdater` 0.9 (`github_releases`, `blocking` features)
- Install receipt written by cargo-dist installer at install time

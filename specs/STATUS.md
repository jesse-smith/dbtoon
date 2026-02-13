# Feature Status Registry

Central registry of all features and their current status.

| Feature ID | Name | Branch | Status | Merged Date | Notes |
|------------|------|--------|--------|-------------|-------|
| 001 | Multi-Database Query CLI | `001-multi-db-query` | Complete | 2026-02-11 | 30 tasks, 6 user stories |
| 002 | Standard Databricks Environment Variable Fallback | `002-std-env-vars` | Complete | 2026-02-12 | 24 tasks |
| 003 | Add Column Types to Output Metadata | `003-column-type-metadata` | Complete | 2026-02-12 | 8 tasks |
| 004 | Multiple Output File Formats | `004-output-formats` | Complete | 2026-02-12 | 21 tasks |
| 005 | Truncation Metadata | `005-truncation-metadata` | Complete | 2026-02-12 | 23 tasks |
| 006 | Cross-Platform Binary Distribution & Self-Update | `006-cargo-dist-release` | Complete | 2026-02-13 | 12 tasks |

## Status Definitions

- **Draft**: Initial specification, not yet approved
- **In Progress**: Active development on feature branch
- **Complete**: All tasks finished, merged to main
- **Archived**: Feature closed without full implementation (see Notes)

## Usage

When completing a feature branch merge:

1. Run `/speckit.featcomp.complete` to update this registry
2. Status headers are added to spec.md, plan.md, tasks.md
3. Commit the status updates

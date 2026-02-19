# Quickstart: Simplify CLI Interface

**Feature Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19

## Developer Setup

```bash
git checkout 009-simplify-cli-ui
cargo build
cargo test
cargo clippy
```

## New Dependency

Add `toml_edit = "0.25"` to `[dependencies]` in `Cargo.toml`. Remove `directories = "6"`.

## Key Files to Modify

| File | Change |
|------|--------|
| `Cargo.toml` | Add `toml_edit`, remove `directories` |
| `src/cli.rs` | Replace `Command` enum: remove `ExecRead`/`ExecWrite`/`ListWarehouses`, add `Init`/`Query`/`Profile`/`Warehouse` |
| `src/config.rs` | Add `$VAR` resolution, replace `ProjectDirs` with `HOME`-based path, config-missing error |
| `src/init.rs` | `dbtoon init` command logic (template generation, env var detection, directory creation) |
| `src/profile.rs` | Profile CRUD via `toml_edit` (create, edit, show, list, test, delete, rename) |
| `src/main.rs` | Update command dispatch to new enum, add `init`/`profile`/`warehouse` handlers |
| `src/error.rs` | No changes needed (existing variants cover all new errors) |
| `tests/unit/config_test.rs` | Add tests for `$VAR` resolution, config path, profile CRUD |

## New Files

| File | Purpose |
|------|---------|
| `src/profile.rs` | Profile management logic (create, edit, show, list, test, delete, rename) |
| `src/init.rs` | `dbtoon init` command logic |
| `tests/unit/profile_test.rs` | Profile management tests |
| `tests/unit/init_test.rs` | Init command tests |

## Implementation Order

1. **Config path** — Replace `directories` with `HOME`-based path
2. **$VAR resolution** — Add `resolve_env_var()` and friends
3. **CLI restructure** — New clap structs (compiles but handlers stub `todo!()`)
4. **Init command** — Template generation + env var detection
5. **Profile CRUD** — create/edit/show/list/delete/rename via `toml_edit`
6. **Query command** — Wire up new `QueryArgs` to existing execution logic
7. **Warehouse command** — Wire up `warehouse list` with `-P` required
8. **Profile test** — Connectivity check
9. **Remove legacy** — Delete old structs, remove `DBTOON_*` env vars
10. **Config resolution hierarchy** — Integrate CLI > profile > defaults > env precedence
11. **README + help text** — Update documentation

## Testing Strategy

- **Unit tests**: `$VAR` resolution, profile field validation, config path, template generation
- **Integration tests**: CLI argument parsing (clap), config file round-trips, init in temp dirs
- **No live DB tests needed**: Existing backend tests are unchanged; new tests focus on config/CLI layer

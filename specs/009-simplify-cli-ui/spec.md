# Feature Specification: Simplify CLI Interface

> **STATUS: COMPLETE** | Merged: 2026-02-19 | Branch: `009-simplify-cli-ui`

**Feature Branch**: `009-simplify-cli-ui`
**Created**: 2026-02-19
**Status**: Draft
**Input**: User description: "Restructure the dbtoon CLI to separate connection management from query execution, replace exec-read/exec-write with a unified query command, and add profile management commands." (Issue #14)

## Clarifications

### Session 2026-02-19

- Q: How does `profile edit` handle field removal? → A: Two mechanisms: (1) `--set key=` (empty value) removes the field, (2) `--unset key` flag exists for explicit removal. Both `profile create --set` and `profile edit --set` support the same syntax.
- Q: Should env-var indirection use separate `_env` fields (e.g., `host`/`host_env`) or inline `$VAR` syntax? → A: Use `$VAR` syntax within the same field (e.g., `host = "$DATABRICKS_HOST"`). A value starting with `$` is resolved as an env var reference; `$$` escapes a literal dollar sign. This eliminates dual-field complexity entirely. On the CLI, `--set host='$DATABRICKS_HOST'` requires shell quoting to prevent shell expansion; unquoted `--set host=$DATABRICKS_HOST` passes the resolved literal value, which is also valid.
- Q: How are valid profile fields per backend defined? → A: Derived from existing Rust connection structs. The implementation plan will document the canonical field list per backend. No new spec-level enumeration needed.
- Q: Should removed commands/flags produce migration hints or be simply unrecognized? → A: Clean break — removed commands are simply unrecognized (clap default). No migration messages needed; this is pre-release software with a single user.
- Q: Are `--database` and `--catalog` true aliases or backend-dependent flags? → A: True aliases for a single internal field (canonically `catalog`). Both set the same value regardless of backend. SQL Server "database" and Databricks "catalog" are the same concept in this tool's abstraction.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - First-Time Setup with Config Initialization (Priority: P1)

A new user installs dbtoon and runs `dbtoon init` to create their configuration file. The system generates a config file at `~/.config/dbtoon/config.toml` with sensible defaults and example profiles. If Databricks standard environment variables are detected, the Databricks profile is auto-populated with `$VAR` references and uncommented. The user receives clear guidance on what to do next.

**Why this priority**: Without a config file, no other commands work. This is the entry point for all users.

**Independent Test**: Can be fully tested by running `dbtoon init` in a clean environment and verifying the generated config file contents and stdout output.

**Acceptance Scenarios**:

1. **Given** no config file exists, **When** the user runs `dbtoon init`, **Then** a config file is created at `~/.config/dbtoon/config.toml` with a `[defaults]` section and commented-out example profiles, and stdout shows the file path and next-step instructions.
2. **Given** Databricks standard env vars (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`) are set, **When** the user runs `dbtoon init`, **Then** the Databricks profile is uncommented with `$VAR` references (e.g., `host = "$DATABRICKS_HOST"`), and stdout lists which required fields are still missing.
3. **Given** no Databricks env vars are set, **When** the user runs `dbtoon init`, **Then** both example profiles remain commented out, and stdout explains how to create profiles.
4. **Given** a config file already exists, **When** the user runs `dbtoon init`, **Then** the system warns that a config file already exists and does not overwrite it.

---

### User Story 2 - Execute a Query Using a Profile (Priority: P1)

A user runs a SQL query against a configured database profile using the unified `query` command. The user specifies which profile to use with `-P`, and can optionally override row limits, timeouts, database/catalog, and schema at the command level.

**Why this priority**: Query execution is the core value of dbtoon. This replaces the previous `exec-read` and `exec-write` commands.

**Independent Test**: Can be tested by creating a profile and running `dbtoon query -P <profile> "SELECT 1"` to verify query execution, output, and flag handling.

**Acceptance Scenarios**:

1. **Given** a valid profile "dev" exists, **When** the user runs `dbtoon query -P dev "SELECT 1"`, **Then** the query executes using the profile's connection settings and results are displayed.
2. **Given** a valid profile exists, **When** the user runs `dbtoon query -P dev -f query.sql`, **Then** the SQL is read from the file and executed.
3. **Given** both positional SQL and `-f` are provided, **When** the user runs the command, **Then** the system reports an error about the conflict.
4. **Given** a profile with `row_limit = 500`, **When** the user runs `dbtoon query -P dev --no-limit "SELECT *"`, **Then** all rows are returned without limit.
5. **Given** a profile exists, **When** the user runs `dbtoon query -P dev -d otherdb "SELECT 1"`, **Then** the query executes against the overridden database/catalog.
6. **Given** no `-P` flag is provided, **When** the user runs `dbtoon query "SELECT 1"`, **Then** the system reports an error that `-P` is required.
7. **Given** a write query (e.g., `INSERT`), **When** the user runs without `--allow-write`, **Then** the system blocks the query with a safety message.
8. **Given** a write query, **When** the user runs with `--allow-write`, **Then** the query executes normally.

---

### User Story 3 - Profile Management (Priority: P2)

A user manages connection profiles through subcommands: creating, editing, viewing, listing, testing, deleting, and renaming profiles. This replaces passing connection details as inline flags.

**Why this priority**: Profile management is how users configure connections. Without it, the `query` command has no profiles to use, but profiles can also be created by editing the config file directly.

**Independent Test**: Can be tested by running `dbtoon profile create`, `profile show`, `profile list`, etc., and verifying the config file is updated correctly.

**Acceptance Scenarios**:

1. **Given** a config file exists, **When** the user runs `dbtoon profile create mydb --backend sqlserver`, **Then** a new `[profiles.mydb]` section is added to the config with `$VAR` defaults for SQL Server.
2. **Given** a config file exists, **When** the user runs `dbtoon profile create mydb --backend databricks --set host=ws.databricks.net`, **Then** the profile is created with the literal `host` value overriding the default `$VAR` reference.
3. **Given** a profile "mydb" exists, **When** the user runs `dbtoon profile edit mydb --set database=newdb`, **Then** the profile's `database` field is updated.
4. **Given** a profile "mydb" exists with `token = "$MY_TOKEN"` and the env var is set, **When** the user runs `dbtoon profile show mydb`, **Then** the display shows the env var name, the masked resolved value, and no warnings.
5. **Given** a profile "mydb" exists with `token = "$MY_TOKEN"` and the env var is NOT set, **When** the user runs `dbtoon profile show mydb`, **Then** a warning indicates the env var is unset.
6. **Given** multiple profiles exist, **When** the user runs `dbtoon profile list`, **Then** all profile names are listed.
7. **Given** a profile "mydb" exists, **When** the user runs `dbtoon profile test mydb`, **Then** the system attempts a connection and reports success or the specific failure.
8. **Given** a profile "mydb" exists, **When** the user runs `dbtoon profile delete mydb`, **Then** the profile is removed from the config.
9. **Given** a profile "old" exists, **When** the user runs `dbtoon profile rename old new`, **Then** the profile is renamed to "new" in the config.
10. **Given** a profile "mydb" has `host = "wrong.host"`, **When** the user runs `dbtoon profile edit mydb --set 'host=$DATABRICKS_HOST'`, **Then** the `host` field is updated to the `$VAR` reference.
11. **Given** a profile "mydb" has `database = "olddb"`, **When** the user runs `dbtoon profile edit mydb --unset database`, **Then** the `database` field is removed from the profile.
12. **Given** a profile "mydb" has `server = "old"`, **When** the user runs `dbtoon profile edit mydb --set server=`, **Then** the `server` field is removed from the profile.

---

### User Story 4 - Warehouse Listing via Profile (Priority: P3)

A user lists available Databricks warehouses using a profile for connection details instead of inline flags.

**Why this priority**: Existing functionality that needs to be migrated to use profiles. Lower priority because the core behavior is unchanged.

**Independent Test**: Can be tested by running `dbtoon warehouse list -P <databricks-profile>` and verifying warehouse list is returned.

**Acceptance Scenarios**:

1. **Given** a valid Databricks profile "dbx" exists, **When** the user runs `dbtoon warehouse list -P dbx`, **Then** the available warehouses are listed.
2. **Given** `--host` or `--token` flags are passed to `warehouse list`, **When** the command runs, **Then** clap rejects them as unrecognized flags (clean break, no migration message).

---

### User Story 5 - Config File Requirement Enforcement (Priority: P1)

When no config file exists, all commands that require one (query, profile *, warehouse list) display a clear error directing the user to run `dbtoon init`.

**Why this priority**: Prevents confusing errors when the user hasn't set up yet.

**Independent Test**: Can be tested by deleting or renaming the config file and running any command that requires it.

**Acceptance Scenarios**:

1. **Given** no config file exists at the default or specified path, **When** the user runs `dbtoon query -P dev "SELECT 1"`, **Then** the system displays an error message directing the user to run `dbtoon init`.
2. **Given** no config file exists, **When** the user runs `dbtoon profile list`, **Then** the same helpful error is shown.

---

### User Story 6 - Config Resolution Hierarchy (Priority: P2)

Configuration values are resolved in a defined priority order: CLI flags override TOML profile values, which override TOML defaults, which override Databricks standard env vars. A `$VAR` reference to an unset variable is an error, not a fallthrough.

**Why this priority**: Correct resolution order is essential for predictable behavior but relies on the config and profile infrastructure being in place first.

**Independent Test**: Can be tested by setting values at different levels of the hierarchy and verifying the correct value is used for query execution.

**Acceptance Scenarios**:

1. **Given** a profile sets `row_limit = 500` and the user passes `--limit 10`, **When** the query runs, **Then** the limit is 10 (CLI flag wins).
2. **Given** the `[defaults]` section sets `timeout = 60` and the profile does not set `timeout`, **When** the query runs, **Then** the timeout is 60 (defaults apply).
3. **Given** a profile sets `host = "$DATABRICKS_HOST"` and the env var is not set, **When** the user tries to use the profile, **Then** an error is raised indicating the env var is unset.
4. **Given** no profile or default sets `catalog`, and `DATABRICKS_CATALOG` is set, **When** a Databricks query runs, **Then** the env var value is used as the lowest-priority fallback.

---

### User Story 7 - Removal of Legacy Commands and Env Vars (Priority: P2)

The `exec-read` and `exec-write` subcommands, all connection-identity flags on query commands, all `DBTOON_*` environment variables, and the macOS `~/Library/` config path are removed.

**Why this priority**: Cleanup is necessary to avoid confusion, but depends on the new commands being in place.

**Independent Test**: Can be tested by verifying that removed commands, flags, and env vars produce appropriate errors or are simply not recognized.

**Acceptance Scenarios**:

1. **Given** the new CLI is installed, **When** the user runs `dbtoon exec-read "SELECT 1"`, **Then** clap rejects it as an unrecognized subcommand.
2. **Given** the new CLI is installed, **When** the user passes `--server` to `dbtoon query`, **Then** clap rejects it as an unrecognized flag.
3. **Given** `DBTOON_SERVER` env var is set, **When** any dbtoon command runs, **Then** the env var is ignored (no effect on behavior).

---

### User Story 8 - Updated Documentation (Priority: P3)

The README and CLI help text are updated to reflect the new command structure, profile-based workflow, and removal of legacy features.

**Why this priority**: Documentation is important but does not block functionality.

**Independent Test**: Can be tested by reviewing the README content and running `dbtoon --help` and `dbtoon query --help` to verify output matches the new structure.

**Acceptance Scenarios**:

1. **Given** the new CLI, **When** the user runs `dbtoon --help`, **Then** the output lists `init`, `query`, `profile`, `warehouse`, and `update` as commands (no `exec-read` or `exec-write`).
2. **Given** the new CLI, **When** the user runs `dbtoon query --help`, **Then** the output shows `-P` as required, shows `-d`/`--database`/`--catalog`/`-s`/`--schema` overrides, and does not show connection-identity flags.
3. **Given** the README, **When** a new user reads it, **Then** it shows `dbtoon init` as the first step, uses `query -P <profile>` in all examples, and documents only Databricks standard env vars.

---

### Edge Cases

- What happens when a user runs `dbtoon init` and the `~/.config/dbtoon/` directory does not exist? The system creates the directory tree.
- What happens when a user tries to create a profile with a name that already exists? The system rejects it with an error.
- What happens when `--database` and `--catalog` are both passed to `query`? They are true aliases for a single internal field (`catalog`). Clap enforces mutual exclusivity via a conflict group — passing both is a clap-level error.
- What happens when `dbtoon init` is run but the config directory is not writable? The system reports a clear file-system error.
- What happens when `profile edit` tries to set a field that is not valid for the profile's backend? The system rejects it with an error listing valid fields. Valid fields are derived from the existing Rust connection structs per backend.
- What happens when `profile test` is run on a profile with missing required fields? The system reports which fields are missing before attempting connection. The connectivity check establishes a connection only (no query executed).
- What happens when a profile value starts with `$` but the env var doesn't exist? The system errors with a message naming the unset variable. There is no fallthrough to a literal interpretation.
- What happens when a user needs a literal value starting with `$`? They use `$$` as an escape (e.g., `host = "$$pecial"` resolves to `$pecial`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `dbtoon init` command that creates a config file at `~/.config/dbtoon/config.toml` with a `[defaults]` section and example profiles.
- **FR-002**: System MUST detect Databricks standard env vars during `init` and auto-populate the Databricks profile with `$VAR` references (e.g., `host = "$DATABRICKS_HOST"`) when any are found.
- **FR-003**: System MUST provide a unified `dbtoon query` command that replaces `exec-read` and `exec-write`, requiring `-P <PROFILE>` for connection selection.
- **FR-004**: System MUST support query-level overrides via `-d`/`--database`/`--catalog` (true aliases for a single `catalog` field, mutually exclusive) and `-s`/`--schema` flags that do not modify the profile.
- **FR-005**: System MUST support `--allow-write` to bypass read-only safety validation on the `query` command.
- **FR-006**: System MUST support `--no-limit` to disable any configured row limit.
- **FR-007**: System MUST support `-l`/`--limit` and `-t`/`--timeout` to override profile/default values.
- **FR-008**: System MUST provide `profile create` with `--backend` and optional `--set key=value` flags, generating `$VAR` defaults appropriate to the backend when no `--set` flags are provided.
- **FR-009**: System MUST provide `profile edit` to update, add, or remove fields on an existing profile. Fields can be removed via `--set key=` (empty value) or `--unset key`. Both `profile create --set` and `profile edit --set` support the same syntax.
- **FR-010**: System MUST provide `profile show` that displays resolved config values with credential masking and unset-env-var warnings. For `$VAR` references, show the env var name, the resolved value (masked by default), and a warning if unset.
- **FR-011**: System MUST provide `profile list` to list all configured profiles.
- **FR-012**: System MUST provide `profile test` that validates required fields are present, then attempts to establish a backend connection (no query executed) and reports success or the specific connection failure.
- **FR-013**: System MUST provide `profile delete` to remove a profile from config.
- **FR-014**: System MUST provide `profile rename` to rename a profile in config.
- **FR-015**: System MUST update `warehouse list` to use `-P <PROFILE>` for connection details.
- **FR-016**: System MUST resolve configuration in priority order: CLI flags > TOML profile > TOML defaults > Databricks standard env vars.
- **FR-017**: System MUST resolve profile string values starting with `$` as environment variable references. If the referenced env var is unset, the system MUST error (no fallthrough). A leading `$$` escapes to a literal `$`.
- **FR-018**: System MUST remove `exec-read` and `exec-write` subcommands.
- **FR-019**: System MUST remove all connection-identity flags from `query` and `warehouse list`.
- **FR-020**: System MUST remove all `DBTOON_*` environment variables.
- **FR-021**: System MUST use `~/.config/dbtoon/` as the config path on all platforms including macOS.
- **FR-022**: System MUST display a helpful error directing the user to `dbtoon init` when a required config file is missing.
- **FR-023**: System MUST support `-c`/`--config`, `-v`/`--verbose`, and `--show-secrets` as global flags on all commands.
- **FR-024**: System MUST update the README to reflect the new CLI structure.
- **FR-025**: System MUST update all `--help` output to reflect the new command structure.

### Key Entities

- **Config File**: TOML file at `~/.config/dbtoon/config.toml` containing defaults and profiles. Created by `init`, modified by `profile` subcommands.
- **Profile**: A named connection configuration within the config file. Contains backend type and connection fields. Field values may be literals or `$VAR` env-var references resolved at use time.
- **Defaults**: Global settings (`row_limit`, `timeout`, `verbose`, `allow_write`) that apply when not overridden by profile or CLI flags.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can go from install to first query in under 3 commands (`init`, optionally `profile create`, `query`).
- **SC-002**: All existing query functionality (read, write, output formats, row limits, timeouts) is accessible through the single `query` command.
- **SC-003**: Users can manage all connection details without editing the config file manually (via `profile` subcommands).
- **SC-004**: Users can switch between database connections by changing only the `-P` flag, without re-specifying connection details.
- **SC-005**: The config resolution hierarchy (CLI > profile > defaults > env vars) produces predictable, documented results in all cases.
- **SC-006**: All removed commands and flags are cleanly removed (no migration shims; clap rejects them as unrecognized).
- **SC-007**: Zero `DBTOON_*` environment variables are recognized by the system; only Databricks standard vars are used as lowest-priority fallbacks.

## Assumptions

- The existing query execution logic, output formatting, and write-query detection remain unchanged in behavior — only how connection details reach them changes.
- Profile names are case-sensitive and must be valid TOML key names.
- The `dbtoon init` command will not overwrite an existing config file (safe re-runs).
- The `--set` syntax for `profile create`/`edit` uses simple `key=value` pairs with no nested structures.
- Credential masking behavior (via `secrecy` crate) is retained as-is; `--show-secrets` globally disables it.
- The `$VAR` resolution applies only to string-typed profile fields. Numeric and boolean fields (e.g., `windows_auth = true`) are always literals.

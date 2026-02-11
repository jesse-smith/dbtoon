# Feature Specification: Standard Databricks Environment Variable Fallback

**Feature Branch**: `002-std-env-vars`
**Created**: 2026-02-11
**Status**: Draft
**Input**: User description: "Address GitHub issue #2 — use standard Databricks environment variables as fallback when dbtoon-specific vars are not set, and add tests for env var vs TOML default resolution."
**Related Issue**: [#2 — Use Standard Databricks Environment Variables](https://github.com/jesse-smith/dbtoon/issues/2)

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Standard Databricks Env Vars as Fallback (Priority: P1)

A user who already has standard Databricks environment variables set (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`) wants to use dbtoon without redefining those values under dbtoon-specific names. When dbtoon-specific environment variables (`DBTOON_DATABRICKS_HOST`, `DBTOON_DATABRICKS_TOKEN`, `DBTOON_WAREHOUSE_ID`, `DBTOON_CATALOG`, `DBTOON_SCHEMA`) are not set, the system should fall back to the standard Databricks environment variables.

**Why this priority**: This is the core ask of the feature. Most Databricks users already have standard env vars configured; requiring dbtoon-specific duplicates is unnecessary friction and the primary pain point described in issue #2.

**Independent Test**: Can be fully tested by setting only standard Databricks env vars, running a Databricks query, and verifying the connection resolves correctly without any dbtoon-specific env vars.

**Acceptance Scenarios**:

1. **Given** only standard Databricks env vars are set (no dbtoon-specific vars, no TOML profile), **When** the user runs a Databricks query, **Then** the system uses the standard env var values for host, token, warehouse ID, catalog, and schema.
2. **Given** both dbtoon-specific and standard Databricks env vars are set, **When** the user runs a Databricks query, **Then** the system uses the dbtoon-specific values (they take precedence).
3. **Given** a CLI flag is provided along with standard Databricks env vars, **When** the user runs a Databricks query, **Then** the CLI flag value takes precedence over the env var.

---

### User Story 2 — TOML Profile Values Override Standard Env Vars (Priority: P2)

A user has a TOML config profile with Databricks connection details and also has standard Databricks env vars set in their environment. The TOML profile values should take precedence over standard environment variables, preserving the existing config hierarchy.

**Why this priority**: Users who have invested in configuring TOML profiles expect those settings to be authoritative for named profiles. This story ensures the fallback doesn't break existing workflows.

**Independent Test**: Can be tested by setting standard Databricks env vars, configuring a TOML profile with different values, selecting that profile, and verifying the TOML values are used.

**Acceptance Scenarios**:

1. **Given** a TOML profile specifies `host = "toml-host"` and `DATABRICKS_HOST=env-host` is set, **When** the user runs a query selecting that profile, **Then** the system uses "toml-host".
2. **Given** a TOML profile specifies `host` but not `catalog`, and `DATABRICKS_CATALOG=env-catalog` is set, **When** the user runs a query selecting that profile, **Then** the system uses "env-catalog" for catalog (fallback applies for fields not in profile).

---

### User Story 3 — Test Coverage for Env Var vs TOML Resolution (Priority: P2)

A developer working on dbtoon wants confidence that the priority ladder for configuration values (CLI > dbtoon env var > TOML profile > standard env var > TOML defaults) is correct and won't regress. Automated tests should cover the key combinations.

**Why this priority**: The existing test suite does not thoroughly cover the interaction between env vars and TOML defaults. Adding tests prevents regressions as the config system evolves.

**Independent Test**: Can be tested by running the automated test suite and verifying all new config resolution tests pass.

**Acceptance Scenarios**:

1. **Given** the test suite is run, **When** tests exercise all tiers of the priority ladder for Databricks fields, **Then** each tier resolves correctly and higher-priority sources override lower ones.
2. **Given** the test suite is run, **When** tests exercise combinations where some tiers are absent, **Then** the system correctly falls through to the next available tier.

---

### Edge Cases

- What happens when a standard env var is set to an empty string? The system should treat empty strings the same as unset (skip to next tier).
- What happens when `DATABRICKS_TOKEN` is set but `DBTOON_DATABRICKS_TOKEN` is also set to an empty string? The dbtoon-specific var being empty should be treated as unset, falling through to the standard var.
- What happens when only some standard env vars are set (e.g., `DATABRICKS_HOST` is set but `DATABRICKS_TOKEN` is not)? The system should use the standard var for host and continue resolving token through other tiers independently per field.
- What happens when a `.env` file contains standard Databricks env vars? They should be loaded by dotenvy and participate in the fallback chain just as shell-exported vars do.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST fall back to standard Databricks environment variables (`DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA`) when the corresponding dbtoon-specific environment variables are not set.
- **FR-002**: System MUST preserve the existing priority ladder: CLI flag > dbtoon-specific env var > TOML profile field > standard Databricks env var > TOML defaults section.
- **FR-003**: System MUST resolve each Databricks configuration field independently through the priority ladder (i.e., host may come from one tier while token comes from another).
- **FR-004**: System MUST treat empty-string environment variables the same as unset for the purpose of fallback resolution.
- **FR-005**: System MUST NOT change the behavior of SQL Server configuration or any non-Databricks settings.
- **FR-006**: System MUST include automated tests covering each tier of the Databricks configuration priority ladder, including combinations where intermediate tiers are absent.
- **FR-007**: System MUST include automated tests verifying that empty-string env vars are treated as unset.

### Key Entities

- **Configuration Field**: A single named setting (e.g., host, token) that can be sourced from multiple tiers. Key attributes: field name, resolved value, source tier.
- **Priority Ladder**: The ordered list of sources consulted for a configuration field: CLI flag, dbtoon-specific env var, TOML profile field, standard Databricks env var, TOML defaults section.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users with standard Databricks env vars set can connect to Databricks without defining any dbtoon-specific env vars, with zero additional setup steps.
- **SC-002**: All existing configuration tests continue to pass with no modifications (backward compatibility).
- **SC-003**: New tests cover at least 5 distinct priority-ladder scenarios for Databricks fields (e.g., CLI-only, env-only, TOML-only, mixed, empty-string fallthrough).
- **SC-004**: No existing CLI flags, env var names, or TOML config fields are removed or renamed (non-breaking change).

## Assumptions

- The standard Databricks environment variable names are: `DATABRICKS_HOST`, `DATABRICKS_TOKEN`, `DATABRICKS_SQL_WAREHOUSE_ID`, `DATABRICKS_CATALOG`, `DATABRICKS_SCHEMA` (as listed in issue #2).
- The `token_env` / `password_env` indirection in TOML profiles is a separate mechanism and is not affected by this change. Standard env vars serve as a final fallback, not a replacement for the `token_env` feature.
- The `.env` file loaded by dotenvy participates naturally — no special handling is needed beyond what dotenvy already provides.

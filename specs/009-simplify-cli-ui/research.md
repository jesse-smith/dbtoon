# Research: Simplify CLI Interface

**Feature Branch**: `009-simplify-cli-ui` | **Date**: 2026-02-19

## R1: TOML File Editing (Preserving Comments)

**Decision**: Add `toml_edit = "0.25"` for config write operations alongside existing `toml = "0.8"` for reads.

**Rationale**: `toml_edit` is the only Rust crate that preserves comments, formatting, and ordering during edits. Both crates are from the same `toml-rs` monorepo and are designed to coexist. The existing `toml` crate loses all comments on serialization.

**Alternatives Considered**:
- **toml only**: Cannot preserve comments — config file would lose user formatting on every `profile` write.
- **toml_edit only**: Would require rewriting all existing deserialization code that uses serde. Unnecessary change.
- **Template-based generation**: Would be fragile and can't handle arbitrary user edits to the config file.

**API Pattern**:
```rust
use toml_edit::{DocumentMut, value, Item, Table};

let content = fs::read_to_string(path)?;
let mut doc = content.parse::<DocumentMut>()?;

// Add a profile
let mut profile = Table::new();
profile["backend"] = value("databricks");
profile["host"] = value("$DATABRICKS_HOST");
doc["profiles"]["mydb"] = Item::Table(profile);

// Remove a profile
doc["profiles"].as_table_mut().unwrap().remove("mydb");

// Write back (comments preserved)
fs::write(path, doc.to_string())?;
```

## R2: Environment Variable Reference Resolution (`$VAR` Syntax)

**Decision**: Simple `resolve_env_var(s: &str) -> Result<String, DbtoonError>` function. No wrapper types, no external crates.

**Rationale**: The `$VAR` / `$$` rules are simple enough (~15 lines) that a dedicated crate would be overkill. A plain function matches the existing codebase patterns (`env_non_empty()`, `resolve_secret()`). Resolution at use-time means TOML structs stay as simple `Option<String>` fields.

**Alternatives Considered**:
- **`shellexpand` crate**: Does full shell expansion (`~`, `$HOME`, etc.) — far more than needed, risk of unexpected behavior.
- **Wrapper type (e.g., `EnvString`)**: Requires implementing `Deserialize`, `Debug`, `Clone`, etc. Unnecessary complexity for a value that's just a string until resolution.
- **Parse-time resolution**: Would prevent showing the original `$VAR` reference in `profile show`. Use-time resolution is spec-required.

**Implementation**:
```rust
fn resolve_env_var(value: &str) -> Result<String, DbtoonError> {
    if let Some(rest) = value.strip_prefix("$$") {
        Ok(format!("${}", rest))  // Escape
    } else if let Some(var_name) = value.strip_prefix('$') {
        std::env::var(var_name).map_err(|_| DbtoonError::Config {
            message: format!("environment variable '{}' is not set", var_name),
        })
    } else {
        Ok(value.to_string())  // Literal
    }
}
```

**Companion functions**: `resolve_profile_string()` → `Option<String>`, `resolve_profile_secret()` → `Option<SecretString>`.

## R3: Config Path (`~/.config/dbtoon/` on All Platforms)

**Decision**: Replace `directories::ProjectDirs` with `std::env::var("HOME")` + hardcoded `.config/dbtoon/config.toml`.

**Rationale**: The spec requires `~/.config/dbtoon/` on ALL platforms including macOS. The `directories` crate returns `~/Library/Application Support/dbtoon/` on macOS and cannot be overridden. The requirement is a single fixed path — no XDG_CONFIG_HOME override needed.

**Alternatives Considered**:
- **`directories` crate**: Cannot produce XDG paths on macOS. Would need a macOS-specific override, defeating the purpose.
- **`xdg` crate**: Respects `XDG_CONFIG_HOME` and defaults to `~/.config`, but adds an unnecessary dependency for what is a single `PathBuf::join`.
- **`dirs` crate**: Same limitation as `directories` on macOS.

**Implementation**:
```rust
fn default_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/dbtoon/config.toml"))
}
```

**Side effect**: Remove `directories` from `Cargo.toml` dependencies.

## R4: Profile Field Validation

**Decision**: Derive valid fields from existing Rust connection structs. No new enum or registry needed.

**Rationale**: The `DatabricksBackend` and `SqlServerBackend` structs already define the canonical fields. Validation can be a `match` on backend type returning `&[&str]` of valid field names.

**Canonical fields per backend**:

| Backend | Fields |
|---------|--------|
| **databricks** | `host`, `token`, `warehouse_id`, `catalog`, `schema` |
| **sqlserver** | `server`, `database`, `username`, `password`, `windows_auth`, `trust_server_certificate` |

**Common fields** (all backends): `backend` (required, set at creation)

## R5: `dbtoon init` Template Generation

**Decision**: Embed the default config template as a `const &str` in the binary. Use string interpolation for conditional uncommenting of Databricks profile when env vars are detected.

**Rationale**: The template is small (~30 lines), rarely changes, and needs conditional logic (detect env vars → uncomment profile). Embedding avoids filesystem dependencies.

**Default template structure**:
```toml
[defaults]
row_limit = 500
timeout = 60

# Example profiles — uncomment and configure:
#
# [profiles.sqlserver_example]
# backend = "sqlserver"
# server = "localhost"
# database = "mydb"
# username = "sa"
# password = "$SA_PASSWORD"
#
# [profiles.databricks_example]
# backend = "databricks"
# host = "$DATABRICKS_HOST"
# token = "$DATABRICKS_TOKEN"
# warehouse_id = "$DATABRICKS_SQL_WAREHOUSE_ID"
# catalog = "$DATABRICKS_CATALOG"
# schema = "$DATABRICKS_SCHEMA"
```

When Databricks env vars are detected during `init`, the Databricks profile section is uncommented.

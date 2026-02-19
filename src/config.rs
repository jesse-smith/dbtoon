use crate::cli::{QueryArgs, WarehouseListArgs};
use crate::error::DbtoonError;
use secrecy::SecretString;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level application configuration.
#[derive(Debug)]
pub struct AppConfig {
    pub backend: BackendConfig,
    pub allow_write: bool,
    pub default_row_limit: Option<usize>,
    pub query_timeout_secs: u64,
    pub verbose: bool,
    pub show_secrets: bool,
    pub output_file: Option<PathBuf>,
}

/// Configured database connection target.
#[derive(Debug)]
pub enum BackendConfig {
    SqlServer {
        server: String,
        database: Option<String>,
        auth: SqlServerAuth,
        trust_server_certificate: bool,
    },
    Databricks {
        host: String,
        token: SecretString,
        warehouse_id: String,
        catalog: Option<String>,
        schema: Option<String>,
    },
}

/// Authentication method for SQL Server.
#[derive(Debug)]
pub enum SqlServerAuth {
    WindowsIntegrated,
    SqlLogin {
        username: String,
        password: SecretString,
    },
}

// --- TOML config file structs ---

#[derive(Debug, Deserialize, Default)]
pub struct TomlConfig {
    #[serde(default)]
    pub defaults: TomlDefaults,
    #[serde(default)]
    pub profiles: HashMap<String, TomlProfile>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TomlDefaults {
    pub row_limit: Option<usize>,
    pub timeout: Option<u64>,
    pub verbose: Option<bool>,
    pub allow_write: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct TomlProfile {
    pub backend: Option<String>,
    pub server: Option<String>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub windows_auth: Option<bool>,
    pub trust_server_certificate: Option<bool>,
    pub host: Option<String>,
    pub token: Option<String>,
    pub warehouse_id: Option<String>,
    pub catalog: Option<String>,
    pub schema: Option<String>,
}

/// Filter `Some("")` to `None`. Passes through `None` and non-empty values.
pub fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|v| !v.is_empty())
}

/// Read an env var, returning `None` if unset or empty.
pub fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Return the default config file path: `$HOME/.config/dbtoon/config.toml`.
pub fn default_config_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config/dbtoon/config.toml"))
}

/// Resolve a `$VAR` reference to its env var value.
///
/// - `$VARNAME` → env var value (error if unset)
/// - `$$...` → literal `$...` (escape)
/// - anything else → literal passthrough
pub fn resolve_env_var(value: &str) -> Result<String, DbtoonError> {
    if let Some(rest) = value.strip_prefix("$$") {
        Ok(format!("${}", rest))
    } else if let Some(var_name) = value.strip_prefix('$') {
        std::env::var(var_name).map_err(|_| DbtoonError::Config {
            message: format!("environment variable '{}' is not set", var_name),
        })
    } else {
        Ok(value.to_string())
    }
}

/// Resolve an optional string profile field via `$VAR` resolution.
pub fn resolve_profile_string(value: Option<&str>) -> Result<Option<String>, DbtoonError> {
    match value {
        Some(v) => resolve_env_var(v).map(Some),
        None => Ok(None),
    }
}

/// Resolve an optional secret profile field via `$VAR` resolution.
pub fn resolve_profile_secret(value: Option<&str>) -> Result<Option<SecretString>, DbtoonError> {
    match value {
        Some(v) => resolve_env_var(v).map(|s| Some(SecretString::from(s))),
        None => Ok(None),
    }
}

/// Config path resolution result — distinguishes explicit vs auto-resolved paths.
struct ResolvedConfigPath {
    path: PathBuf,
}

/// Resolve the config file path: --config flag > platform default.
fn resolve_config_path(cli_config: Option<&PathBuf>) -> Option<ResolvedConfigPath> {
    if let Some(path) = cli_config {
        return Some(ResolvedConfigPath { path: path.clone() });
    }
    default_config_path().map(|path| ResolvedConfigPath { path })
}

/// Load and parse the TOML config file (if it exists).
pub fn load_toml_config(resolved: Option<&PathBuf>, explicit: bool) -> Result<TomlConfig, DbtoonError> {
    let path = match resolved {
        Some(p) => p,
        None => return Ok(TomlConfig::default()),
    };

    if !path.exists() {
        if explicit {
            return Err(DbtoonError::Config {
                message: format!("config file not found: {}", path.display()),
            });
        }
        return Ok(TomlConfig::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| DbtoonError::Config {
        message: format!("cannot read config file {}: {}", path.display(), e),
    })?;

    toml::from_str(&content).map_err(|e| DbtoonError::Config {
        message: format!("invalid config file {}: {}", path.display(), e),
    })
}

/// Load and parse the TOML config file, erroring if missing (for commands that require it).
pub fn load_toml_config_required(config_path: Option<&PathBuf>) -> Result<(TomlConfig, PathBuf), DbtoonError> {
    let resolved = resolve_config_path(config_path);
    match resolved {
        Some(r) => {
            if !r.path.exists() {
                return Err(DbtoonError::Config {
                    message: format!(
                        "config file not found: {}\n\nRun `dbtoon init` to create one.",
                        r.path.display()
                    ),
                });
            }
            let content = std::fs::read_to_string(&r.path).map_err(|e| DbtoonError::Config {
                message: format!("cannot read config file {}: {}", r.path.display(), e),
            })?;
            let config: TomlConfig = toml::from_str(&content).map_err(|e| DbtoonError::Config {
                message: format!("invalid config file {}: {}", r.path.display(), e),
            })?;
            Ok((config, r.path))
        }
        None => Err(DbtoonError::Config {
            message: "cannot determine config file location (HOME not set)\n\nRun `dbtoon init` to create one.".to_string(),
        }),
    }
}

/// Load a named profile from config, with `$VAR` resolution applied to string fields.
pub fn load_profile(
    toml_config: &TomlConfig,
    profile_name: &str,
) -> Result<TomlProfile, DbtoonError> {
    toml_config.profiles.get(profile_name).cloned().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", profile_name),
    })
}

/// Build a BackendConfig from a resolved profile, applying CLI overrides and Databricks env fallbacks.
pub fn build_backend_config(
    profile: &TomlProfile,
    cli_database: Option<&str>,
    cli_schema: Option<&str>,
) -> Result<BackendConfig, DbtoonError> {
    let backend_str = profile.backend.as_deref().ok_or_else(|| DbtoonError::Config {
        message: "profile has no 'backend' field".to_string(),
    })?;

    match backend_str {
        "sqlserver" => build_sqlserver_config(profile, cli_database),
        "databricks" => build_databricks_config(profile, cli_database, cli_schema),
        other => Err(DbtoonError::Config {
            message: format!("unknown backend type: '{}' (expected 'sqlserver' or 'databricks')", other),
        }),
    }
}

fn build_sqlserver_config(
    profile: &TomlProfile,
    cli_database: Option<&str>,
) -> Result<BackendConfig, DbtoonError> {
    let server = resolve_profile_string(profile.server.as_deref())?
        .ok_or_else(|| DbtoonError::Config {
            message: "no 'server' specified for sqlserver backend".to_string(),
        })?;

    let database = if let Some(db) = cli_database {
        Some(db.to_string())
    } else {
        resolve_profile_string(profile.database.as_deref())?
    };

    let windows_auth = profile.windows_auth.unwrap_or(false);

    let auth = if windows_auth {
        SqlServerAuth::WindowsIntegrated
    } else {
        let username = resolve_profile_string(profile.username.as_deref())?
            .ok_or_else(|| DbtoonError::Config {
                message: "no 'username' specified for SQL Server SQL Auth".to_string(),
            })?;

        let password = resolve_profile_secret(profile.password.as_deref())?
            .ok_or_else(|| DbtoonError::Config {
                message: "no 'password' specified for SQL Server SQL Auth".to_string(),
            })?;

        SqlServerAuth::SqlLogin { username, password }
    };

    let trust_server_certificate = profile.trust_server_certificate.unwrap_or(false);

    Ok(BackendConfig::SqlServer {
        server,
        database,
        auth,
        trust_server_certificate,
    })
}

fn build_databricks_config(
    profile: &TomlProfile,
    cli_database: Option<&str>,
    cli_schema: Option<&str>,
) -> Result<BackendConfig, DbtoonError> {
    // Databricks standard env vars as lowest-priority fallback
    let std_host = env_non_empty("DATABRICKS_HOST");
    let std_token = env_non_empty("DATABRICKS_TOKEN");
    let std_warehouse = env_non_empty("DATABRICKS_SQL_WAREHOUSE_ID");
    let std_catalog = env_non_empty("DATABRICKS_CATALOG");
    let std_schema = env_non_empty("DATABRICKS_SCHEMA");

    let host = resolve_profile_string(profile.host.as_deref())?
        .or(std_host)
        .ok_or_else(|| DbtoonError::Config {
            message: "no 'host' specified for databricks backend".to_string(),
        })?;

    let token = resolve_profile_secret(profile.token.as_deref())?
        .or_else(|| std_token.map(SecretString::from))
        .ok_or_else(|| DbtoonError::Config {
            message: "no 'token' specified for databricks backend".to_string(),
        })?;

    let warehouse_id = resolve_profile_string(profile.warehouse_id.as_deref())?
        .or(std_warehouse)
        .ok_or_else(|| DbtoonError::Config {
            message: "no 'warehouse_id' specified for databricks backend".to_string(),
        })?;

    let catalog = if let Some(db) = cli_database {
        Some(db.to_string())
    } else {
        resolve_profile_string(profile.catalog.as_deref())?
            .or(std_catalog)
    };

    let schema = if let Some(s) = cli_schema {
        Some(s.to_string())
    } else {
        resolve_profile_string(profile.schema.as_deref())?
            .or(std_schema)
    };

    Ok(BackendConfig::Databricks {
        host,
        token,
        warehouse_id,
        catalog,
        schema,
    })
}

/// Build AppConfig from query args.
pub fn load_from_query_args(
    args: &QueryArgs,
    toml_config: &TomlConfig,
    verbose: bool,
    show_secrets: bool,
) -> Result<AppConfig, DbtoonError> {
    let profile = load_profile(toml_config, &args.profile)?;

    // --database and --catalog are aliases for a single concept
    let cli_database = args.database.as_deref().or(args.catalog.as_deref());
    let cli_schema = args.schema.as_deref();

    let backend = build_backend_config(&profile, cli_database, cli_schema)?;

    // allow_write: CLI flag > defaults > false
    let allow_write = args.allow_write || toml_config.defaults.allow_write.unwrap_or(false);

    // row_limit: --no-limit > --limit > defaults > 500
    let default_row_limit = if args.no_limit {
        None
    } else {
        Some(args.limit.unwrap_or_else(|| toml_config.defaults.row_limit.unwrap_or(500)))
    };

    // timeout: --timeout > defaults > 60
    let query_timeout_secs = args.timeout
        .unwrap_or_else(|| toml_config.defaults.timeout.unwrap_or(60));

    // verbose: CLI flag OR defaults
    let verbose = verbose || toml_config.defaults.verbose.unwrap_or(false);

    Ok(AppConfig {
        backend,
        allow_write,
        default_row_limit,
        query_timeout_secs,
        verbose,
        show_secrets,
        output_file: args.output.clone(),
    })
}

/// Build AppConfig for warehouse list.
pub fn load_from_warehouse_list_args(
    args: &WarehouseListArgs,
    toml_config: &TomlConfig,
    verbose: bool,
    show_secrets: bool,
) -> Result<AppConfig, DbtoonError> {
    let profile = load_profile(toml_config, &args.profile)?;
    let backend = build_backend_config(&profile, None, None)?;

    let verbose = verbose || toml_config.defaults.verbose.unwrap_or(false);

    Ok(AppConfig {
        backend,
        allow_write: false,
        default_row_limit: Some(toml_config.defaults.row_limit.unwrap_or(500)),
        query_timeout_secs: toml_config.defaults.timeout.unwrap_or(60),
        verbose,
        show_secrets,
        output_file: None,
    })
}

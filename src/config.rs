use crate::cli::{ExecArgs, ListWarehousesArgs};
use crate::error::DbtoonError;
use directories::ProjectDirs;
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
struct TomlConfig {
    #[serde(default)]
    defaults: TomlDefaults,
    #[serde(default)]
    profiles: HashMap<String, TomlProfile>,
}

#[derive(Debug, Deserialize, Default)]
struct TomlDefaults {
    row_limit: Option<usize>,
    timeout: Option<u64>,
    verbose: Option<bool>,
    allow_write: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct TomlProfile {
    backend: Option<String>,
    server: Option<String>,
    database: Option<String>,
    username: Option<String>,
    password: Option<String>,
    password_env: Option<String>,
    windows_auth: Option<bool>,
    trust_server_certificate: Option<bool>,
    host: Option<String>,
    token: Option<String>,
    token_env: Option<String>,
    warehouse_id: Option<String>,
    catalog: Option<String>,
    schema: Option<String>,
}

/// Config path resolution result — distinguishes explicit vs auto-resolved paths.
struct ResolvedConfigPath {
    path: PathBuf,
    /// true if user explicitly specified via --config or DBTOON_CONFIG
    explicit: bool,
}

/// Resolve the config file path: --config flag > env var > platform default.
fn resolve_config_path(cli_config: Option<&PathBuf>) -> Option<ResolvedConfigPath> {
    if let Some(path) = cli_config {
        return Some(ResolvedConfigPath { path: path.clone(), explicit: true });
    }
    if let Ok(path) = std::env::var("DBTOON_CONFIG") {
        return Some(ResolvedConfigPath { path: PathBuf::from(path), explicit: true });
    }
    ProjectDirs::from("", "", "dbtoon")
        .map(|dirs| ResolvedConfigPath {
            path: dirs.config_dir().join("config.toml"),
            explicit: false,
        })
}

/// Load and parse the TOML config file (if it exists).
fn load_toml_config(resolved: Option<&ResolvedConfigPath>) -> Result<TomlConfig, DbtoonError> {
    let resolved = match resolved {
        Some(r) => r,
        None => return Ok(TomlConfig::default()),
    };

    if !resolved.path.exists() {
        if resolved.explicit {
            return Err(DbtoonError::Config {
                message: format!("config file not found: {}", resolved.path.display()),
            });
        }
        // Auto-resolved path doesn't exist — that's fine
        return Ok(TomlConfig::default());
    }

    let content = std::fs::read_to_string(&resolved.path).map_err(|e| DbtoonError::Config {
        message: format!("cannot read config file {}: {}", resolved.path.display(), e),
    })?;

    toml::from_str(&content).map_err(|e| DbtoonError::Config {
        message: format!("invalid config file {}: {}", resolved.path.display(), e),
    })
}

/// Resolve a password from direct value, env indirection, or env var.
fn resolve_secret(
    direct: Option<&str>,
    env_key: Option<&str>,
    fallback_env: &str,
) -> Option<SecretString> {
    // Direct value first
    if let Some(val) = direct
        && !val.is_empty() {
            return Some(SecretString::from(val.to_string()));
        }
    // Env indirection (e.g., password_env = "MY_SECRET")
    if let Some(key) = env_key
        && let Ok(val) = std::env::var(key)
            && !val.is_empty() {
                return Some(SecretString::from(val));
            }
    // Fallback env var (e.g., DBTOON_PASSWORD)
    if let Ok(val) = std::env::var(fallback_env)
        && !val.is_empty() {
            return Some(SecretString::from(val));
        }
    None
}

/// Build AppConfig from exec-read/exec-write CLI args.
pub fn load_from_exec_args(
    args: &ExecArgs,
    verbose: bool,
    show_secrets: bool,
    config_path: Option<&PathBuf>,
) -> Result<AppConfig, DbtoonError> {
    let resolved_path = resolve_config_path(config_path);
    let toml_config = load_toml_config(resolved_path.as_ref())?;

    // Load profile if specified
    let profile = args.profile.as_ref().map(|name| {
        toml_config.profiles.get(name).cloned().ok_or_else(|| DbtoonError::Config {
            message: format!("profile '{}' not found in config file", name),
        })
    }).transpose()?;

    let profile = profile.unwrap_or_default();

    // Resolve backend type: CLI > env > profile > error
    let backend_str = args
        .backend
        .as_deref()
        .or(profile.backend.as_deref())
        .ok_or_else(|| DbtoonError::Config {
            message: "no backend specified — use --backend or configure a profile".to_string(),
        })?;

    let backend = match backend_str {
        "sqlserver" => {
            let server = args
                .server
                .as_deref()
                .or(profile.server.as_deref())
                .ok_or_else(|| DbtoonError::Config {
                    message: "no server specified for sqlserver backend".to_string(),
                })?
                .to_string();

            let database = args
                .database
                .as_deref()
                .or(profile.database.as_deref())
                .map(|s| s.to_string());

            let windows_auth =
                args.windows_auth || profile.windows_auth.unwrap_or(false);

            let auth = if windows_auth {
                SqlServerAuth::WindowsIntegrated
            } else {
                let username = args
                    .username
                    .as_deref()
                    .or(profile.username.as_deref())
                    .ok_or_else(|| DbtoonError::Config {
                        message: "no username specified for SQL Server SQL Auth".to_string(),
                    })?
                    .to_string();

                let password = resolve_secret(
                    args.password.as_deref(),
                    profile.password_env.as_deref(),
                    "DBTOON_PASSWORD",
                )
                .or_else(|| {
                    profile
                        .password
                        .as_ref()
                        .map(|p| SecretString::from(p.clone()))
                })
                .ok_or_else(|| DbtoonError::Config {
                    message: "no password specified for SQL Server SQL Auth".to_string(),
                })?;

                SqlServerAuth::SqlLogin { username, password }
            };

            let trust_server_certificate = args.trust_server_certificate
                || profile.trust_server_certificate.unwrap_or(false);

            BackendConfig::SqlServer {
                server,
                database,
                auth,
                trust_server_certificate,
            }
        }
        "databricks" => {
            let host = args
                .host
                .as_deref()
                .or(profile.host.as_deref())
                .ok_or_else(|| DbtoonError::Config {
                    message: "no host specified for databricks backend".to_string(),
                })?
                .to_string();

            let token = resolve_secret(
                args.token.as_deref(),
                profile.token_env.as_deref(),
                "DBTOON_DATABRICKS_TOKEN",
            )
            .or_else(|| {
                profile
                    .token
                    .as_ref()
                    .map(|t| SecretString::from(t.clone()))
            })
            .ok_or_else(|| DbtoonError::Config {
                message: "no token specified for databricks backend".to_string(),
            })?;

            let warehouse_id = args
                .warehouse
                .as_deref()
                .or(profile.warehouse_id.as_deref())
                .ok_or_else(|| DbtoonError::Config {
                    message: "no warehouse ID specified for databricks backend".to_string(),
                })?
                .to_string();

            let catalog = args
                .catalog
                .as_deref()
                .or(profile.catalog.as_deref())
                .map(|s| s.to_string());

            let schema = args
                .schema
                .as_deref()
                .or(profile.schema.as_deref())
                .map(|s| s.to_string());

            BackendConfig::Databricks {
                host,
                token,
                warehouse_id,
                catalog,
                schema,
            }
        }
        other => {
            return Err(DbtoonError::Config {
                message: format!("unknown backend type: '{}' (expected 'sqlserver' or 'databricks')", other),
            });
        }
    };

    // Resolve allow_write: env > config defaults
    let allow_write = std::env::var("DBTOON_ALLOW_WRITE")
        .map(|v| v == "true")
        .unwrap_or(toml_config.defaults.allow_write.unwrap_or(false));

    // row_limit: --no-limit > CLI/ENV > TOML > 500
    let default_row_limit = if args.no_limit {
        None
    } else {
        Some(args.limit.unwrap_or_else(|| toml_config.defaults.row_limit.unwrap_or(500)))
    };

    // timeout: CLI/ENV > TOML > 60
    let query_timeout_secs = args.timeout
        .unwrap_or_else(|| toml_config.defaults.timeout.unwrap_or(60));

    // verbose: CLI/ENV OR TOML default
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

/// Build AppConfig for list-warehouses subcommand.
pub fn load_from_list_warehouses_args(
    args: &ListWarehousesArgs,
    verbose: bool,
    show_secrets: bool,
    config_path: Option<&PathBuf>,
) -> Result<AppConfig, DbtoonError> {
    let resolved_path = resolve_config_path(config_path);
    let toml_config = load_toml_config(resolved_path.as_ref())?;

    let profile = args.profile.as_ref().map(|name| {
        toml_config.profiles.get(name).cloned().ok_or_else(|| DbtoonError::Config {
            message: format!("profile '{}' not found in config file", name),
        })
    }).transpose()?;

    let profile = profile.unwrap_or_default();

    let host = args
        .host
        .as_deref()
        .or(profile.host.as_deref())
        .ok_or_else(|| DbtoonError::Config {
            message: "no host specified for list-warehouses".to_string(),
        })?
        .to_string();

    let token = resolve_secret(
        args.token.as_deref(),
        profile.token_env.as_deref(),
        "DBTOON_DATABRICKS_TOKEN",
    )
    .or_else(|| {
        profile
            .token
            .as_ref()
            .map(|t| SecretString::from(t.clone()))
    })
    .ok_or_else(|| DbtoonError::Config {
        message: "no token specified for list-warehouses".to_string(),
    })?;

    let warehouse_id = profile.warehouse_id.clone().unwrap_or_default();

    let backend = BackendConfig::Databricks {
        host,
        token,
        warehouse_id,
        catalog: profile.catalog.clone(),
        schema: profile.schema.clone(),
    };

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

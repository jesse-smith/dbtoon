use dbtoon::cli::{ExecArgs, ListWarehousesArgs};
use dbtoon::config::{
    env_non_empty, load_from_exec_args, load_from_list_warehouses_args, non_empty, BackendConfig,
    SqlServerAuth,
};
use secrecy::ExposeSecret;
use std::path::PathBuf;
use std::sync::Mutex;

// --- Env var test infrastructure (T001) ---

/// Static mutex to serialize tests that touch process env vars.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// RAII guard that sets env vars on creation and removes them on Drop.
/// Holds the ENV_MUTEX lock for its lifetime.
struct EnvGuard {
    keys: Vec<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// Create a guard that sets the given env vars and holds the mutex.
    fn new(vars: &[(&str, &str)]) -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        for (key, val) in vars {
            // SAFETY: env var access is serialized by ENV_MUTEX
            unsafe { std::env::set_var(key, val); }
        }
        EnvGuard {
            keys: vars.iter().map(|(k, _)| k.to_string()).collect(),
            _lock: lock,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            // SAFETY: env var access is serialized by ENV_MUTEX
            unsafe { std::env::remove_var(key); }
        }
    }
}

fn make_exec_args(overrides: impl FnOnce(&mut ExecArgs)) -> ExecArgs {
    let mut args = ExecArgs {
        sql: Some("SELECT 1".to_string()),
        sql_file: None,
        backend: None,
        server: None,
        database: None,
        username: None,
        password: None,
        windows_auth: false,
        trust_server_certificate: false,
        host: None,
        token: None,
        warehouse: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        profile: None,
    };
    overrides(&mut args);
    args
}

#[test]
fn test_sqlserver_windows_auth_config() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.database = Some("testdb".to_string());
        a.windows_auth = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::SqlServer { server, database, auth, .. } => {
            assert_eq!(server, "localhost");
            assert_eq!(database.as_deref(), Some("testdb"));
            assert!(matches!(auth, SqlServerAuth::WindowsIntegrated));
        }
        _ => panic!("Expected SqlServer backend"),
    }
}

#[test]
fn test_sqlserver_sql_auth_config() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.username = Some("sa".to_string());
        a.password = Some("secret".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::SqlServer { auth, .. } => {
            assert!(matches!(auth, SqlServerAuth::SqlLogin { .. }));
        }
        _ => panic!("Expected SqlServer backend"),
    }
}

#[test]
fn test_missing_backend_errors() {
    let args = make_exec_args(|_| {});
    let result = load_from_exec_args(&args, false, false, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no backend specified"), "Got: {}", err);
}

#[test]
fn test_default_row_limit() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.default_row_limit, Some(500));
}

#[test]
fn test_no_limit_flag() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
        a.no_limit = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.default_row_limit, None);
}

#[test]
fn test_default_timeout() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.query_timeout_secs, 60);
}

#[test]
fn test_default_allow_write_is_false() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert!(!config.allow_write);
}

#[test]
fn test_config_file_not_found_errors() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
    });

    let bad_path = PathBuf::from("/nonexistent/config.toml");
    let result = load_from_exec_args(&args, false, false, Some(&bad_path));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("config file not found"), "Got: {}", err);
}

#[test]
fn test_explicit_limit_overrides_default() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
        a.limit = Some(100);
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.default_row_limit, Some(100));
}

#[test]
fn test_explicit_timeout_overrides_default() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
        a.timeout = Some(120);
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.query_timeout_secs, 120);
}

#[test]
fn test_no_limit_overrides_explicit_limit() {
    let args = make_exec_args(|a| {
        a.backend = Some("sqlserver".to_string());
        a.server = Some("localhost".to_string());
        a.windows_auth = true;
        a.limit = Some(100);
        a.no_limit = true;
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    assert_eq!(config.default_row_limit, None);
}

// --- T002: Unit tests for non_empty ---

#[test]
fn test_non_empty_none() {
    assert_eq!(non_empty(None), None);
}

#[test]
fn test_non_empty_empty_string() {
    assert_eq!(non_empty(Some("")), None);
}

#[test]
fn test_non_empty_value() {
    assert_eq!(non_empty(Some("value")), Some("value"));
}

// --- T003: Unit tests for env_non_empty ---

#[test]
fn test_env_non_empty_unset() {
    let _guard = EnvGuard::new(&[]);
    // Ensure the var is not set
    unsafe { std::env::remove_var("TEST_ENV_NON_EMPTY_UNSET"); }
    assert_eq!(env_non_empty("TEST_ENV_NON_EMPTY_UNSET"), None);
}

#[test]
fn test_env_non_empty_empty() {
    let _guard = EnvGuard::new(&[("TEST_ENV_NON_EMPTY_EMPTY", "")]);
    assert_eq!(env_non_empty("TEST_ENV_NON_EMPTY_EMPTY"), None);
}

#[test]
fn test_env_non_empty_value() {
    let _guard = EnvGuard::new(&[("TEST_ENV_NON_EMPTY_VAL", "value")]);
    assert_eq!(env_non_empty("TEST_ENV_NON_EMPTY_VAL"), Some("value".to_string()));
}

// --- T006: Standard Databricks env var fallback (US1) ---

#[test]
fn test_databricks_std_env_fallback() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse-id"),
        ("DATABRICKS_CATALOG", "std-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, catalog, schema } => {
            assert_eq!(host, "https://std-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-std-token");
            assert_eq!(warehouse_id, "std-warehouse-id");
            assert_eq!(catalog.as_deref(), Some("std-catalog"));
            assert_eq!(schema.as_deref(), Some("std-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T007: dbtoon-specific env overrides standard env (US1) ---

#[test]
fn test_dbtoon_env_overrides_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
        ("DATABRICKS_CATALOG", "std-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    // Simulate clap having resolved dbtoon-specific env vars into args
    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.host = Some("https://dbtoon-host.azuredatabricks.net".to_string());
        a.token = Some("dapi-dbtoon-token".to_string());
        a.warehouse = Some("dbtoon-warehouse".to_string());
        a.catalog = Some("dbtoon-catalog".to_string());
        a.schema = Some("dbtoon-schema".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, catalog, schema } => {
            assert_eq!(host, "https://dbtoon-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-dbtoon-token");
            assert_eq!(warehouse_id, "dbtoon-warehouse");
            assert_eq!(catalog.as_deref(), Some("dbtoon-catalog"));
            assert_eq!(schema.as_deref(), Some("dbtoon-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

fn make_list_warehouses_args(overrides: impl FnOnce(&mut ListWarehousesArgs)) -> ListWarehousesArgs {
    let mut args = ListWarehousesArgs {
        host: None,
        token: None,
        profile: None,
    };
    overrides(&mut args);
    args
}

/// Write a TOML config to a temp file and return its path.
fn write_temp_toml(content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("dbtoon-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("config-{}.toml", std::process::id()));
    std::fs::write(&path, content).unwrap();
    path
}

// --- T010: TOML profile overrides standard env vars (US2) ---

#[test]
fn test_toml_profile_overrides_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
        ("DATABRICKS_CATALOG", "std-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let toml_content = r#"
[profiles.test]
backend = "databricks"
host = "https://toml-host.azuredatabricks.net"
token = "dapi-toml-token"
warehouse_id = "toml-warehouse"
catalog = "toml-catalog"
schema = "toml-schema"
"#;
    let config_path = write_temp_toml(toml_content);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.profile = Some("test".to_string());
    });

    let config = load_from_exec_args(&args, false, false, Some(&config_path)).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, catalog, schema } => {
            assert_eq!(host, "https://toml-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-toml-token");
            assert_eq!(warehouse_id, "toml-warehouse");
            assert_eq!(catalog.as_deref(), Some("toml-catalog"));
            assert_eq!(schema.as_deref(), Some("toml-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }

    std::fs::remove_file(&config_path).ok();
}

// --- T011: Partial TOML profile falls through to standard env (US2) ---

#[test]
fn test_toml_partial_profile_falls_through_to_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
        ("DATABRICKS_CATALOG", "env-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let toml_content = r#"
[profiles.partial]
backend = "databricks"
host = "https://toml-host.azuredatabricks.net"
token = "dapi-toml-token"
warehouse_id = "toml-warehouse"
"#;
    let config_path = write_temp_toml(toml_content);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.profile = Some("partial".to_string());
    });

    let config = load_from_exec_args(&args, false, false, Some(&config_path)).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, catalog, .. } => {
            assert_eq!(host, "https://toml-host.azuredatabricks.net");
            assert_eq!(catalog.as_deref(), Some("env-catalog"));
        }
        _ => panic!("Expected Databricks backend"),
    }

    std::fs::remove_file(&config_path).ok();
}

// --- T012: list-warehouses standard env var fallback (US2) ---

#[test]
fn test_list_warehouses_std_env_fallback() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
        ("DATABRICKS_CATALOG", "std-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let args = make_list_warehouses_args(|_| {});

    let config = load_from_list_warehouses_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, catalog, schema } => {
            assert_eq!(host, "https://std-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-std-token");
            assert_eq!(warehouse_id, "std-warehouse");
            assert_eq!(catalog.as_deref(), Some("std-catalog"));
            assert_eq!(schema.as_deref(), Some("std-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T015: CLI flag overrides all tiers (US3) ---

#[test]
fn test_cli_flag_overrides_all_tiers() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
    ]);

    let toml_content = r#"
[profiles.test]
backend = "databricks"
host = "https://toml-host.azuredatabricks.net"
token = "dapi-toml-token"
warehouse_id = "toml-warehouse"
"#;
    let config_path = write_temp_toml(toml_content);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.profile = Some("test".to_string());
        a.host = Some("https://cli-host.azuredatabricks.net".to_string());
        a.token = Some("dapi-cli-token".to_string());
        a.warehouse = Some("cli-warehouse".to_string());
    });

    let config = load_from_exec_args(&args, false, false, Some(&config_path)).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, .. } => {
            assert_eq!(host, "https://cli-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-cli-token");
            assert_eq!(warehouse_id, "cli-warehouse");
        }
        _ => panic!("Expected Databricks backend"),
    }

    std::fs::remove_file(&config_path).ok();
}

// --- T016: Empty dbtoon env falls through to standard env (US3) ---

#[test]
fn test_empty_dbtoon_env_falls_through_to_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
    ]);

    // Simulate clap resolving empty DBTOON_DATABRICKS_HOST to Some("")
    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.host = Some(String::new());
        a.token = Some(String::new());
        a.warehouse = Some(String::new());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, .. } => {
            assert_eq!(host, "https://std-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-std-token");
            assert_eq!(warehouse_id, "std-warehouse");
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T017: Empty standard env treated as unset (US3) ---

#[test]
fn test_empty_std_env_treated_as_unset() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", ""),
    ]);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
    });

    let result = load_from_exec_args(&args, false, false, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("no host specified"), "Got: {}", err);
}

#[test]
fn test_empty_std_env_catalog_treated_as_none() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
        ("DATABRICKS_CATALOG", ""),
    ]);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { catalog, .. } => {
            assert_eq!(catalog.as_deref(), None);
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T018: Independent field resolution (US3) ---

#[test]
fn test_independent_field_resolution() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_TOKEN", "std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
    ]);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
        a.host = Some("https://cli-host.azuredatabricks.net".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, token, warehouse_id, .. } => {
            assert_eq!(host, "https://cli-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "std-token");
            assert_eq!(warehouse_id, "std-warehouse");
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T019: Standard env token fallback (US3) ---

#[test]
fn test_std_env_token_fallback() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { token, .. } => {
            assert_eq!(token.expose_secret(), "std-token");
        }
        _ => panic!("Expected Databricks backend"),
    }
}

// --- T020: Dotenv standard vars participate (US3) ---

#[test]
fn test_dotenv_std_vars_participate() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let dir = std::env::temp_dir().join("dbtoon-dotenv-test");
    std::fs::create_dir_all(&dir).unwrap();
    let env_path = dir.join(".env");
    std::fs::write(&env_path, "DATABRICKS_HOST=https://dotenv-host.azuredatabricks.net\n").unwrap();

    dotenvy::from_path(&env_path).ok();

    let args = make_exec_args(|a| {
        a.backend = Some("databricks".to_string());
    });

    let config = load_from_exec_args(&args, false, false, None).unwrap();
    match &config.backend {
        BackendConfig::Databricks { host, .. } => {
            assert_eq!(host, "https://dotenv-host.azuredatabricks.net");
        }
        _ => panic!("Expected Databricks backend"),
    }

    // Cleanup
    unsafe { std::env::remove_var("DATABRICKS_HOST"); }
    std::fs::remove_file(&env_path).ok();
    std::fs::remove_dir(&dir).ok();
}

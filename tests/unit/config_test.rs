use dbtoon::config::{
    self, default_config_path, env_non_empty, load_toml_config_required,
    non_empty, resolve_env_var, resolve_profile_string, resolve_profile_secret,
    BackendConfig, SqlServerAuth, TomlConfig, TomlProfile,
};
use secrecy::ExposeSecret;
use std::path::PathBuf;
use std::sync::Mutex;

// --- Env var test infrastructure ---

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
            unsafe { std::env::remove_var(key); }
        }
    }
}

/// Helper to build a TomlConfig with a single profile for testing.
fn make_toml_config(profile_name: &str, profile: TomlProfile) -> TomlConfig {
    let mut profiles = std::collections::HashMap::new();
    profiles.insert(profile_name.to_string(), profile);
    TomlConfig {
        defaults: Default::default(),
        profiles,
    }
}

// =====================================================================
// T002: Unit tests for non_empty
// =====================================================================

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

// =====================================================================
// T003: Unit tests for env_non_empty
// =====================================================================

#[test]
fn test_env_non_empty_unset() {
    let _guard = EnvGuard::new(&[]);
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

// =====================================================================
// T004: Unit tests for default_config_path()
// =====================================================================

#[test]
fn test_default_config_path_with_home() {
    let _guard = EnvGuard::new(&[("HOME", "/Users/testuser")]);
    let path = default_config_path();
    assert_eq!(
        path,
        Some(PathBuf::from("/Users/testuser/.config/dbtoon/config.toml"))
    );
}

#[test]
fn test_default_config_path_no_home() {
    let _guard = EnvGuard::new(&[]);
    unsafe { std::env::remove_var("HOME"); }
    let path = default_config_path();
    assert_eq!(path, None);
}

// =====================================================================
// T005: Unit tests for resolve_env_var()
// =====================================================================

#[test]
fn test_resolve_env_var_literal_passthrough() {
    let result = resolve_env_var("plain_value").unwrap();
    assert_eq!(result, "plain_value");
}

#[test]
fn test_resolve_env_var_dollar_reference() {
    let _guard = EnvGuard::new(&[("TEST_RESOLVE_VAR", "resolved_value")]);
    let result = resolve_env_var("$TEST_RESOLVE_VAR").unwrap();
    assert_eq!(result, "resolved_value");
}

#[test]
fn test_resolve_env_var_dollar_dollar_escape() {
    let result = resolve_env_var("$$pecial").unwrap();
    assert_eq!(result, "$pecial");
}

#[test]
fn test_resolve_env_var_unset_var_error() {
    let _guard = EnvGuard::new(&[]);
    unsafe { std::env::remove_var("NONEXISTENT_TEST_VAR_009"); }
    let result = resolve_env_var("$NONEXISTENT_TEST_VAR_009");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("NONEXISTENT_TEST_VAR_009"), "Got: {}", err);
    assert!(err.contains("not set"), "Got: {}", err);
}

#[test]
fn test_resolve_profile_string_none() {
    let result = resolve_profile_string(None).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_resolve_profile_string_literal() {
    let result = resolve_profile_string(Some("literal")).unwrap();
    assert_eq!(result, Some("literal".to_string()));
}

#[test]
fn test_resolve_profile_string_var() {
    let _guard = EnvGuard::new(&[("TEST_PROFILE_STR", "resolved")]);
    let result = resolve_profile_string(Some("$TEST_PROFILE_STR")).unwrap();
    assert_eq!(result, Some("resolved".to_string()));
}

#[test]
fn test_resolve_profile_secret_var() {
    let _guard = EnvGuard::new(&[("TEST_PROFILE_SECRET", "secret_val")]);
    let result = resolve_profile_secret(Some("$TEST_PROFILE_SECRET")).unwrap();
    assert_eq!(result.unwrap().expose_secret(), "secret_val");
}

// =====================================================================
// T006: CLI parsing tests — covered in cli_test.rs
// =====================================================================

// =====================================================================
// T007: Config-missing error directs user to `dbtoon init`
// =====================================================================

#[test]
fn test_config_missing_error_mentions_init() {
    let _guard = EnvGuard::new(&[("HOME", "/tmp/dbtoon-test-nonexistent")]);
    let result = load_toml_config_required(None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("dbtoon init"), "Error should mention 'dbtoon init', got: {}", err);
}

#[test]
fn test_config_explicit_not_found_errors() {
    let bad_path = PathBuf::from("/nonexistent/config.toml");
    let result = load_toml_config_required(Some(&bad_path));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("config file not found"), "Got: {}", err);
    assert!(err.contains("dbtoon init"), "Got: {}", err);
}

// =====================================================================
// Profile loading and backend config tests
// =====================================================================

#[test]
fn test_sqlserver_windows_auth_config() {
    let profile = TomlProfile {
        backend: Some("sqlserver".to_string()),
        server: Some("localhost".to_string()),
        database: Some("testdb".to_string()),
        windows_auth: Some(true),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
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
    let profile = TomlProfile {
        backend: Some("sqlserver".to_string()),
        server: Some("localhost".to_string()),
        username: Some("sa".to_string()),
        password: Some("secret".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
        BackendConfig::SqlServer { auth, .. } => {
            assert!(matches!(auth, SqlServerAuth::SqlLogin { .. }));
        }
        _ => panic!("Expected SqlServer backend"),
    }
}

#[test]
fn test_missing_backend_errors() {
    let profile = TomlProfile::default();
    let result = config::build_backend_config(&profile, None, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("backend"), "Got: {}", err);
}

#[test]
fn test_databricks_std_env_fallback() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse-id"),
        ("DATABRICKS_CATALOG", "std-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
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

#[test]
fn test_toml_profile_overrides_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        host: Some("https://toml-host.azuredatabricks.net".to_string()),
        token: Some("dapi-toml-token".to_string()),
        warehouse_id: Some("toml-warehouse".to_string()),
        catalog: Some("toml-catalog".to_string()),
        schema: Some("toml-schema".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
        BackendConfig::Databricks { host, token, warehouse_id, catalog, schema } => {
            assert_eq!(host, "https://toml-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-toml-token");
            assert_eq!(warehouse_id, "toml-warehouse");
            assert_eq!(catalog.as_deref(), Some("toml-catalog"));
            assert_eq!(schema.as_deref(), Some("toml-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_toml_partial_profile_falls_through_to_std_env() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://std-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-std-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "std-warehouse"),
        ("DATABRICKS_CATALOG", "env-catalog"),
        ("DATABRICKS_SCHEMA", "std-schema"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        host: Some("https://toml-host.azuredatabricks.net".to_string()),
        token: Some("dapi-toml-token".to_string()),
        warehouse_id: Some("toml-warehouse".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
        BackendConfig::Databricks { host, catalog, .. } => {
            assert_eq!(host, "https://toml-host.azuredatabricks.net");
            assert_eq!(catalog.as_deref(), Some("env-catalog"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_cli_database_override() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
        ("DATABRICKS_CATALOG", "env-catalog"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, Some("cli-catalog"), None).unwrap();
    match &backend {
        BackendConfig::Databricks { catalog, .. } => {
            assert_eq!(catalog.as_deref(), Some("cli-catalog"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_cli_schema_override() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        schema: Some("profile-schema".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, Some("cli-schema")).unwrap();
    match &backend {
        BackendConfig::Databricks { schema, .. } => {
            assert_eq!(schema.as_deref(), Some("cli-schema"));
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_dollar_var_resolution_in_profile() {
    let _guard = EnvGuard::new(&[
        ("MY_HOST", "https://resolved-host.azuredatabricks.net"),
        ("MY_TOKEN", "dapi-resolved-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        host: Some("$MY_HOST".to_string()),
        token: Some("$MY_TOKEN".to_string()),
        ..Default::default()
    };
    let backend = config::build_backend_config(&profile, None, None).unwrap();
    match &backend {
        BackendConfig::Databricks { host, token, .. } => {
            assert_eq!(host, "https://resolved-host.azuredatabricks.net");
            assert_eq!(token.expose_secret(), "dapi-resolved-token");
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_dollar_var_unset_error_in_profile() {
    let _guard = EnvGuard::new(&[]);
    unsafe { std::env::remove_var("UNSET_VAR_FOR_TEST_009"); }

    let profile = TomlProfile {
        backend: Some("databricks".to_string()),
        host: Some("$UNSET_VAR_FOR_TEST_009".to_string()),
        ..Default::default()
    };
    let result = config::build_backend_config(&profile, None, None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("UNSET_VAR_FOR_TEST_009"), "Got: {}", err);
}

// =====================================================================
// Query args config loading
// =====================================================================

#[test]
fn test_load_from_query_args_basic() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert_eq!(app_config.default_row_limit, Some(500));
    assert_eq!(app_config.query_timeout_secs, 60);
    assert!(!app_config.allow_write);
}

#[test]
fn test_load_from_query_args_no_limit() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: true,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert_eq!(app_config.default_row_limit, None);
}

#[test]
fn test_load_from_query_args_explicit_limit() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: Some(100),
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert_eq!(app_config.default_row_limit, Some(100));
}

#[test]
fn test_load_from_query_args_allow_write() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: true,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert!(app_config.allow_write);
}

// =====================================================================
// T017: Profile loading and config resolution (CLI > profile > defaults > Databricks env)
// =====================================================================

#[test]
fn test_toml_defaults_apply_when_profile_missing_field() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let mut toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });
    toml_config.defaults.row_limit = Some(200);
    toml_config.defaults.timeout = Some(90);

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert_eq!(app_config.default_row_limit, Some(200), "should use defaults.row_limit");
    assert_eq!(app_config.query_timeout_secs, 90, "should use defaults.timeout");
}

#[test]
fn test_cli_limit_overrides_defaults() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let mut toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });
    toml_config.defaults.row_limit = Some(200);

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: Some(50),
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert_eq!(app_config.default_row_limit, Some(50), "CLI limit should override defaults");
}

// =====================================================================
// T019: --allow-write flag gating write queries
// =====================================================================

#[test]
fn test_allow_write_false_by_default() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("INSERT INTO t VALUES(1)".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert!(!app_config.allow_write, "allow_write should be false by default");
}

#[test]
fn test_allow_write_from_defaults() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
    ]);

    let mut toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });
    toml_config.defaults.allow_write = Some(true);

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    assert!(app_config.allow_write, "defaults.allow_write should propagate");
}

// =====================================================================
// T038-T039: Full resolution hierarchy integration tests
// =====================================================================

#[test]
fn test_full_resolution_hierarchy_cli_wins() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://env-host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-env-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "env-wh"),
        ("DATABRICKS_CATALOG", "env-catalog"),
    ]);

    // Profile sets catalog, but CLI override should win
    let mut toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        catalog: Some("profile-catalog".to_string()),
        ..Default::default()
    });
    toml_config.defaults.row_limit = Some(200);

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: Some("cli-catalog".to_string()),
        catalog: None,
        schema: None,
        limit: Some(10),
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    match &app_config.backend {
        BackendConfig::Databricks { catalog, .. } => {
            assert_eq!(catalog.as_deref(), Some("cli-catalog"), "CLI should win over profile");
        }
        _ => panic!("Expected Databricks backend"),
    }
    assert_eq!(app_config.default_row_limit, Some(10), "CLI limit should win over defaults");
}

#[test]
fn test_databricks_env_fallback_lowest_priority() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://host.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
        ("DATABRICKS_CATALOG", "env-catalog-fallback"),
    ]);

    // Profile does NOT set catalog — env var should be used as fallback
    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let app_config = config::load_from_query_args(&args, &toml_config, false, false).unwrap();
    match &app_config.backend {
        BackendConfig::Databricks { catalog, .. } => {
            assert_eq!(catalog.as_deref(), Some("env-catalog-fallback"), "Databricks env var should be used as fallback");
        }
        _ => panic!("Expected Databricks backend"),
    }
}

#[test]
fn test_dollar_var_unset_is_error_not_fallthrough() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_TOKEN", "dapi-token"),
        ("DATABRICKS_SQL_WAREHOUSE_ID", "wh-id"),
        ("DATABRICKS_HOST", "fallback-host"),
    ]);
    unsafe { std::env::remove_var("UNSET_HOST_FOR_RESOLUTION_TEST"); }

    // Profile sets host as $VAR reference to unset var — should error, NOT fall through to DATABRICKS_HOST
    let toml_config = make_toml_config("dev", TomlProfile {
        backend: Some("databricks".to_string()),
        host: Some("$UNSET_HOST_FOR_RESOLUTION_TEST".to_string()),
        ..Default::default()
    });

    let args = dbtoon::cli::QueryArgs {
        sql: Some("SELECT 1".to_string()),
        file: None,
        profile: "dev".to_string(),
        database: None,
        catalog: None,
        schema: None,
        limit: None,
        no_limit: false,
        timeout: None,
        output: None,
        allow_write: false,
    };

    let result = config::load_from_query_args(&args, &toml_config, false, false);
    assert!(result.is_err(), "$VAR to unset var should error, not fallthrough");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("UNSET_HOST_FOR_RESOLUTION_TEST"), "Got: {}", err);
}

// =====================================================================
// T036: Config-missing error for all config-dependent commands
// =====================================================================

#[test]
fn test_config_missing_error_with_explicit_nonexistent_path() {
    let bad_path = PathBuf::from("/tmp/dbtoon-test-nonexistent-cfg/config.toml");
    let result = load_toml_config_required(Some(&bad_path));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("dbtoon init"), "should mention dbtoon init, got: {}", err);
}

#[test]
fn test_profile_not_found_errors() {
    let toml_config = TomlConfig::default();
    let result = config::load_profile(&toml_config, "nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Got: {}", err);
}

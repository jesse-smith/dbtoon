use dbtoon::cli::ExecArgs;
use dbtoon::config::{load_from_exec_args, BackendConfig, SqlServerAuth};
use std::path::PathBuf;

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
        limit: 500,
        no_limit: false,
        timeout: 60,
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

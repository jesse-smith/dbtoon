use clap::Parser;
use dbtoon::cli::Cli;

/// Parse a CLI command from args (simulating shell invocation).
fn parse_cli(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(args)
}

// =====================================================================
// T006: CLI parsing tests for new command structure
// =====================================================================

// --- Init ---

#[test]
fn test_cli_init_command() {
    let cli = parse_cli(&["dbtoon", "init"]).unwrap();
    assert!(matches!(cli.command, dbtoon::cli::Command::Init));
}

// --- Query ---

#[test]
fn test_cli_query_requires_profile() {
    let result = parse_cli(&["dbtoon", "query", "SELECT 1"]);
    assert!(result.is_err(), "query without -P should fail");
}

#[test]
fn test_cli_query_with_profile_and_sql() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.profile, "dev");
            assert_eq!(args.sql.as_deref(), Some("SELECT 1"));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_file() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-f", "query.sql"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.profile, "dev");
            assert!(args.file.is_some());
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_sql_conflicts_with_file() {
    let result = parse_cli(&["dbtoon", "query", "-P", "dev", "SELECT 1", "-f", "query.sql"]);
    assert!(result.is_err(), "SQL and --file should conflict");
}

#[test]
fn test_cli_query_database_and_catalog_conflict() {
    let result = parse_cli(&[
        "dbtoon", "query", "-P", "dev", "-d", "mydb", "--catalog", "mycat", "SELECT 1",
    ]);
    assert!(result.is_err(), "--database and --catalog should conflict");
}

#[test]
fn test_cli_query_with_database_short() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-d", "mydb", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.database.as_deref(), Some("mydb"));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_catalog() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "--catalog", "mycat", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.catalog.as_deref(), Some("mycat"));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_schema() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-s", "myschema", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.schema.as_deref(), Some("myschema"));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_limit() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-l", "100", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.limit, Some(100));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_no_limit() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "--no-limit", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert!(args.no_limit);
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_timeout() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-t", "120", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert_eq!(args.timeout, Some(120));
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_output() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "-o", "out.csv", "SELECT 1"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert!(args.output.is_some());
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_query_with_allow_write() {
    let cli = parse_cli(&["dbtoon", "query", "-P", "dev", "--allow-write", "INSERT INTO t VALUES(1)"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Query(args) => {
            assert!(args.allow_write);
        }
        _ => panic!("Expected Query command"),
    }
}

// --- Profile ---

#[test]
fn test_cli_profile_create() {
    let cli = parse_cli(&["dbtoon", "profile", "create", "mydb", "--backend", "sqlserver"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Create(args)) => {
            assert_eq!(args.name, "mydb");
            assert_eq!(args.backend, "sqlserver");
        }
        _ => panic!("Expected Profile Create command"),
    }
}

#[test]
fn test_cli_profile_create_with_set() {
    let cli = parse_cli(&[
        "dbtoon", "profile", "create", "mydb", "--backend", "sqlserver",
        "--set", "server=localhost", "--set", "database=testdb",
    ]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Create(args)) => {
            assert_eq!(args.set_fields.len(), 2);
            assert_eq!(args.set_fields[0], "server=localhost");
        }
        _ => panic!("Expected Profile Create command"),
    }
}

#[test]
fn test_cli_profile_edit() {
    let cli = parse_cli(&["dbtoon", "profile", "edit", "mydb", "--set", "database=newdb"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Edit(args)) => {
            assert_eq!(args.name, "mydb");
            assert_eq!(args.set_fields[0], "database=newdb");
        }
        _ => panic!("Expected Profile Edit command"),
    }
}

#[test]
fn test_cli_profile_edit_unset() {
    let cli = parse_cli(&["dbtoon", "profile", "edit", "mydb", "--unset", "database"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Edit(args)) => {
            assert_eq!(args.unset_fields[0], "database");
        }
        _ => panic!("Expected Profile Edit command"),
    }
}

#[test]
fn test_cli_profile_show() {
    let cli = parse_cli(&["dbtoon", "profile", "show", "mydb"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Show(args)) => {
            assert_eq!(args.name, "mydb");
        }
        _ => panic!("Expected Profile Show command"),
    }
}

#[test]
fn test_cli_profile_list() {
    let cli = parse_cli(&["dbtoon", "profile", "list"]).unwrap();
    assert!(matches!(
        cli.command,
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::List)
    ));
}

#[test]
fn test_cli_profile_test() {
    let cli = parse_cli(&["dbtoon", "profile", "test", "mydb"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Test(args)) => {
            assert_eq!(args.name, "mydb");
        }
        _ => panic!("Expected Profile Test command"),
    }
}

#[test]
fn test_cli_profile_delete() {
    let cli = parse_cli(&["dbtoon", "profile", "delete", "mydb"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Delete(args)) => {
            assert_eq!(args.name, "mydb");
        }
        _ => panic!("Expected Profile Delete command"),
    }
}

#[test]
fn test_cli_profile_rename() {
    let cli = parse_cli(&["dbtoon", "profile", "rename", "old", "new"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Profile(dbtoon::cli::ProfileCommand::Rename(args)) => {
            assert_eq!(args.old, "old");
            assert_eq!(args.new, "new");
        }
        _ => panic!("Expected Profile Rename command"),
    }
}

// --- Warehouse ---

#[test]
fn test_cli_warehouse_list_requires_profile() {
    let result = parse_cli(&["dbtoon", "warehouse", "list"]);
    assert!(result.is_err(), "warehouse list without -P should fail");
}

#[test]
fn test_cli_warehouse_list_with_profile() {
    let cli = parse_cli(&["dbtoon", "warehouse", "list", "-P", "dbx"]).unwrap();
    match &cli.command {
        dbtoon::cli::Command::Warehouse(args) => {
            match &args.command {
                dbtoon::cli::WarehouseCommand::List(list_args) => {
                    assert_eq!(list_args.profile, "dbx");
                }
            }
        }
        _ => panic!("Expected Warehouse command"),
    }
}

// --- Global flags ---

#[test]
fn test_cli_global_config_flag() {
    let cli = parse_cli(&["dbtoon", "-c", "/custom/config.toml", "init"]).unwrap();
    assert_eq!(
        cli.config,
        Some(std::path::PathBuf::from("/custom/config.toml"))
    );
}

#[test]
fn test_cli_global_verbose_flag() {
    let cli = parse_cli(&["dbtoon", "-v", "init"]).unwrap();
    assert!(cli.verbose);
}

#[test]
fn test_cli_global_show_secrets_flag() {
    let cli = parse_cli(&["dbtoon", "--show-secrets", "init"]).unwrap();
    assert!(cli.show_secrets);
}

// =====================================================================
// T042: Legacy commands and flags are rejected
// =====================================================================

#[test]
fn test_exec_read_rejected() {
    let result = parse_cli(&["dbtoon", "exec-read", "SELECT 1"]);
    assert!(result.is_err(), "exec-read should be unrecognized");
}

#[test]
fn test_exec_write_rejected() {
    let result = parse_cli(&["dbtoon", "exec-write", "SELECT 1"]);
    assert!(result.is_err(), "exec-write should be unrecognized");
}

#[test]
fn test_list_warehouses_rejected() {
    let result = parse_cli(&["dbtoon", "list-warehouses"]);
    assert!(result.is_err(), "list-warehouses should be unrecognized");
}

#[test]
fn test_query_server_flag_rejected() {
    let result = parse_cli(&["dbtoon", "query", "-P", "dev", "--server", "localhost", "SELECT 1"]);
    assert!(result.is_err(), "--server should be unrecognized on query");
}

#[test]
fn test_query_host_flag_rejected() {
    let result = parse_cli(&["dbtoon", "query", "-P", "dev", "--host", "ws.databricks.net", "SELECT 1"]);
    assert!(result.is_err(), "--host should be unrecognized on query");
}

#[test]
fn test_query_token_flag_rejected() {
    let result = parse_cli(&["dbtoon", "query", "-P", "dev", "--token", "dapi-123", "SELECT 1"]);
    assert!(result.is_err(), "--token should be unrecognized on query");
}

#[test]
fn test_query_backend_flag_rejected() {
    let result = parse_cli(&["dbtoon", "query", "-P", "dev", "--backend", "sqlserver", "SELECT 1"]);
    assert!(result.is_err(), "--backend should be unrecognized on query");
}

#[test]
fn test_warehouse_list_host_flag_rejected() {
    let result = parse_cli(&["dbtoon", "warehouse", "list", "-P", "dbx", "--host", "ws.databricks.net"]);
    assert!(result.is_err(), "--host should be unrecognized on warehouse list");
}

#[test]
fn test_warehouse_list_token_flag_rejected() {
    let result = parse_cli(&["dbtoon", "warehouse", "list", "-P", "dbx", "--token", "dapi-123"]);
    assert!(result.is_err(), "--token should be unrecognized on warehouse list");
}

// --- Update ---

#[test]
fn test_cli_update_command() {
    let cli = parse_cli(&["dbtoon", "update"]).unwrap();
    assert!(matches!(cli.command, dbtoon::cli::Command::Update));
}

use dbtoon::validation::{validate, BackendDialect, ValidationResult};

fn assert_safe(sql: &str, dialect: BackendDialect) {
    match validate(sql, dialect) {
        ValidationResult::Safe => {}
        ValidationResult::Denied { reasons } => {
            panic!(
                "Expected Safe for '{}', got Denied: {:?}",
                sql,
                reasons.iter().map(|r| &r.detail).collect::<Vec<_>>()
            );
        }
    }
}

fn assert_denied(sql: &str, dialect: BackendDialect, expected_kind: &str) {
    match validate(sql, dialect) {
        ValidationResult::Safe => {
            panic!("Expected Denied for '{}', got Safe", sql);
        }
        ValidationResult::Denied { reasons } => {
            assert!(!reasons.is_empty(), "Expected denial reasons for '{}'", sql);
            // Verify the denial kind matches
            let kind_str = format!("{:?}", reasons[0].kind);
            assert!(
                kind_str.contains(expected_kind),
                "Expected kind '{}' for '{}', got '{}'",
                expected_kind,
                sql,
                kind_str
            );
        }
    }
}

// --- SELECT (allowed) ---

#[test]
fn test_select_allowed() {
    assert_safe("SELECT 1", BackendDialect::SqlServer);
}

#[test]
fn test_select_from_table_allowed() {
    assert_safe("SELECT * FROM users", BackendDialect::SqlServer);
}

#[test]
fn test_select_with_join_allowed() {
    assert_safe(
        "SELECT u.name, o.id FROM users u JOIN orders o ON u.id = o.user_id",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_select_with_subquery_allowed() {
    assert_safe(
        "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_select_with_cte_allowed() {
    assert_safe(
        "WITH cte AS (SELECT id FROM users) SELECT * FROM cte",
        BackendDialect::SqlServer,
    );
}

// --- EXPLAIN / DESCRIBE / SHOW / USE (allowed) ---

#[test]
fn test_explain_allowed() {
    assert_safe("EXPLAIN SELECT 1", BackendDialect::SqlServer);
}

#[test]
fn test_show_tables_allowed() {
    assert_safe("SHOW TABLES", BackendDialect::Databricks);
}

#[test]
fn test_use_allowed() {
    assert_safe("USE mydb", BackendDialect::SqlServer);
}

// --- INSERT (denied) ---

#[test]
fn test_insert_denied() {
    assert_denied(
        "INSERT INTO users (name) VALUES ('alice')",
        BackendDialect::SqlServer,
        "Dml",
    );
}

// --- UPDATE (denied) ---

#[test]
fn test_update_denied() {
    assert_denied(
        "UPDATE users SET name = 'bob' WHERE id = 1",
        BackendDialect::SqlServer,
        "Dml",
    );
}

// --- DELETE (denied) ---

#[test]
fn test_delete_denied() {
    assert_denied(
        "DELETE FROM users WHERE id = 1",
        BackendDialect::SqlServer,
        "Dml",
    );
}

// --- DROP (denied) ---

#[test]
fn test_drop_table_denied() {
    assert_denied(
        "DROP TABLE users",
        BackendDialect::SqlServer,
        "Ddl",
    );
}

// --- SELECT INTO (denied) ---

#[test]
fn test_select_into_denied() {
    assert_denied(
        "SELECT * INTO new_table FROM users",
        BackendDialect::SqlServer,
        "SelectInto",
    );
}

// --- CTE-wrapped writes (denied) ---

#[test]
fn test_cte_wrapped_insert_denied() {
    assert_denied(
        "WITH cte AS (SELECT 1 AS id) INSERT INTO users (id) SELECT id FROM cte",
        BackendDialect::SqlServer,
        "CteWrappedWrite",
    );
}

#[test]
fn test_cte_wrapped_delete_denied() {
    assert_denied(
        "WITH cte AS (SELECT 1 AS id) DELETE FROM users WHERE id IN (SELECT id FROM cte)",
        BackendDialect::SqlServer,
        "CteWrappedWrite",
    );
}

// --- EXEC / EXECUTE (denied — non-allowlisted procedures) ---

#[test]
fn test_exec_denied() {
    assert_denied(
        "EXEC my_custom_proc",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

#[test]
fn test_execute_denied() {
    assert_denied(
        "EXECUTE some_write_proc",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

// --- Parse failure (denied) ---

#[test]
fn test_unparseable_sql_denied() {
    assert_denied(
        "THIS IS NOT VALID SQL !!!",
        BackendDialect::SqlServer,
        "ParseFailure",
    );
}

// --- Multi-statement batch ---

#[test]
fn test_multi_statement_all_select_allowed() {
    assert_safe(
        "SELECT 1; SELECT 2; SELECT 3",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_multi_statement_with_insert_denied() {
    assert_denied(
        "SELECT 1; INSERT INTO users (name) VALUES ('alice')",
        BackendDialect::SqlServer,
        "Dml",
    );
}

// --- US1: Legitimate read-only patterns (Phase 2) ---

#[test]
fn test_set_nocount_with_select_safe() {
    assert_safe(
        "SET NOCOUNT ON; SELECT * FROM users",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_begin_tran_select_commit_safe() {
    // Note: sqlparser doesn't support T-SQL `BEGIN TRAN` abbreviation,
    // only `BEGIN TRANSACTION`. Using full form to test the intent.
    assert_safe(
        "BEGIN TRANSACTION; SELECT * FROM orders; COMMIT",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_declare_variable_with_select_safe() {
    assert_safe(
        "DECLARE @id INT = 1; SELECT * FROM users WHERE id = @id",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_standalone_set_safe() {
    assert_safe(
        "SET NOCOUNT ON",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_begin_transaction_select_commit_safe() {
    assert_safe(
        "BEGIN TRANSACTION; SELECT 1; COMMIT",
        BackendDialect::SqlServer,
    );
}

#[test]
fn test_begin_transaction_drop_table_denied() {
    assert_denied(
        "BEGIN TRANSACTION; DROP TABLE users; COMMIT",
        BackendDialect::SqlServer,
        "Ddl",
    );
}

// --- US2: Write operations denied with category-specific reasons (Phase 4) ---

#[test]
fn test_grant_denied_dcl() {
    assert_denied(
        "GRANT SELECT ON users TO public_role",
        BackendDialect::SqlServer,
        "Dcl",
    );
}

#[test]
fn test_revoke_denied_dcl() {
    assert_denied(
        "REVOKE SELECT ON users FROM public_role",
        BackendDialect::SqlServer,
        "Dcl",
    );
}

#[test]
fn test_create_index_denied_ddl() {
    assert_denied(
        "CREATE INDEX idx ON users (name)",
        BackendDialect::SqlServer,
        "Ddl",
    );
}

#[test]
fn test_truncate_denied_ddl() {
    assert_denied(
        "TRUNCATE TABLE users",
        BackendDialect::SqlServer,
        "Ddl",
    );
}

#[test]
fn test_merge_denied_dml() {
    assert_denied(
        "MERGE INTO target USING source ON target.id = source.id WHEN MATCHED THEN UPDATE SET name = source.name",
        BackendDialect::SqlServer,
        "Dml",
    );
}

#[test]
fn test_select_into_regression() {
    assert_denied(
        "SELECT * INTO new_table FROM users",
        BackendDialect::SqlServer,
        "SelectInto",
    );
}

#[test]
fn test_cte_wrapped_insert_regression() {
    assert_denied(
        "WITH cte AS (SELECT 1) INSERT INTO users SELECT * FROM cte",
        BackendDialect::SqlServer,
        "CteWrappedWrite",
    );
}

#[test]
fn test_backup_database_denied() {
    // BACKUP DATABASE is T-SQL specific. sqlparser may or may not parse it.
    // Either way, it should be denied (ParseFailure if unparseable, or category-specific if parsed).
    let result = validate("BACKUP DATABASE mydb TO DISK = '/path'", BackendDialect::SqlServer);
    match result {
        ValidationResult::Safe => panic!("Expected BACKUP DATABASE to be denied, got Safe"),
        ValidationResult::Denied { reasons } => {
            // Document which path: ParseFailure (sqlparser doesn't know BACKUP) or Operational
            let kind_str = format!("{:?}", reasons[0].kind);
            assert!(
                kind_str.contains("ParseFailure") || kind_str.contains("Operational"),
                "Expected ParseFailure or Operational for BACKUP DATABASE, got '{}'",
                kind_str
            );
        }
    }
}

#[test]
fn test_dbcc_checkdb_denied() {
    // DBCC is T-SQL specific. sqlparser may or may not parse it.
    // Either way, it should be denied (ParseFailure if unparseable, or category-specific if parsed).
    let result = validate("DBCC CHECKDB", BackendDialect::SqlServer);
    match result {
        ValidationResult::Safe => panic!("Expected DBCC CHECKDB to be denied, got Safe"),
        ValidationResult::Denied { reasons } => {
            let kind_str = format!("{:?}", reasons[0].kind);
            assert!(
                kind_str.contains("ParseFailure") || kind_str.contains("Operational"),
                "Expected ParseFailure or Operational for DBCC CHECKDB, got '{}'",
                kind_str
            );
        }
    }
}

// --- US3: Schema exploration via safe system procedures (Phase 5) ---

#[test]
fn test_exec_sp_help_safe_on_sqlserver() {
    assert_safe("EXEC sp_help 'users'", BackendDialect::SqlServer);
}

#[test]
fn test_exec_sp_columns_safe_on_sqlserver() {
    assert_safe("EXEC sp_columns 'orders'", BackendDialect::SqlServer);
}

#[test]
fn test_exec_sp_help_uppercase_safe() {
    assert_safe("EXEC SP_HELP 'users'", BackendDialect::SqlServer);
}

#[test]
fn test_exec_three_part_name_safe() {
    assert_safe("EXEC master.dbo.sp_help 'users'", BackendDialect::SqlServer);
}

#[test]
fn test_exec_two_part_name_safe() {
    assert_safe("EXEC dbo.sp_help 'users'", BackendDialect::SqlServer);
}

#[test]
fn test_exec_sp_executesql_denied() {
    assert_denied(
        "EXEC sp_executesql N'SELECT 1'",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

#[test]
fn test_exec_non_allowlisted_denied() {
    assert_denied(
        "EXEC my_custom_proc",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

#[test]
fn test_exec_sp_help_denied_on_databricks() {
    assert_denied(
        "EXEC sp_help 'users'",
        BackendDialect::Databricks,
        "StoredProcedure",
    );
}

#[test]
fn test_exec_sp_help_evil_denied() {
    assert_denied(
        "EXEC sp_help_evil",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

// --- Databricks dialect ---

#[test]
fn test_databricks_select_allowed() {
    assert_safe("SELECT * FROM main.default.my_table", BackendDialect::Databricks);
}

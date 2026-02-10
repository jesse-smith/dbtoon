use dbtoon::validation::{validate, BackendDialect, DenialKind, ValidationResult};

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
        "WriteStatement",
    );
}

// --- UPDATE (denied) ---

#[test]
fn test_update_denied() {
    assert_denied(
        "UPDATE users SET name = 'bob' WHERE id = 1",
        BackendDialect::SqlServer,
        "WriteStatement",
    );
}

// --- DELETE (denied) ---

#[test]
fn test_delete_denied() {
    assert_denied(
        "DELETE FROM users WHERE id = 1",
        BackendDialect::SqlServer,
        "WriteStatement",
    );
}

// --- DROP (denied) ---

#[test]
fn test_drop_table_denied() {
    assert_denied(
        "DROP TABLE users",
        BackendDialect::SqlServer,
        "WriteStatement",
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

// --- EXEC / EXECUTE (denied) ---

#[test]
fn test_exec_denied() {
    assert_denied(
        "EXEC sp_help",
        BackendDialect::SqlServer,
        "StoredProcedure",
    );
}

#[test]
fn test_execute_denied() {
    assert_denied(
        "EXECUTE sp_who",
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
        "WriteStatement",
    );
}

// --- Databricks dialect ---

#[test]
fn test_databricks_select_allowed() {
    assert_safe("SELECT * FROM main.default.my_table", BackendDialect::Databricks);
}

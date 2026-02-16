use dbtoon::backend::sqlserver::SqlServerBackend;
use dbtoon::backend::{Backend, CellValue, QueryResult};
use dbtoon::config::SqlServerAuth;
use secrecy::SecretString;
use std::env;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read env vars and construct a SqlServerBackend. Returns None when the
/// required `TEST_SQLSERVER_HOST` env var is absent, allowing tests to skip
/// gracefully on local dev machines.
fn try_backend() -> Option<SqlServerBackend> {
    let host = env::var("TEST_SQLSERVER_HOST").ok()?;
    let port = env::var("TEST_SQLSERVER_PORT").unwrap_or_else(|_| "1433".into());
    let user = env::var("TEST_SQLSERVER_USER").unwrap_or_else(|_| "sa".into());
    let password = env::var("TEST_SQLSERVER_PASSWORD").unwrap_or_else(|_| String::new());

    let server = format!("{},{}", host, port);
    let auth = SqlServerAuth::SqlLogin {
        username: user,
        password: SecretString::from(password),
    };

    Some(SqlServerBackend::new(server, None, auth, true))
}

/// Early-return skip macro: if no SQL Server env is configured, print a
/// diagnostic and return Ok(()) so the test shows as "passed (skipped)".
macro_rules! require_sqlserver {
    () => {
        match try_backend() {
            Some(b) => b,
            None => {
                println!("SKIP: TEST_SQLSERVER_HOST not set — skipping integration test");
                return;
            }
        }
    };
}

/// Default timeout for integration test queries (seconds).
const DEFAULT_TIMEOUT: u64 = 30;

/// Execute a query with no row limit.
async fn exec(backend: &SqlServerBackend, sql: &str) -> Result<QueryResult, dbtoon::error::DbtoonError> {
    backend.execute(sql, None, DEFAULT_TIMEOUT).await
}

/// Execute a query with a row limit.
#[allow(dead_code)]
async fn exec_limited(
    backend: &SqlServerBackend,
    sql: &str,
    limit: usize,
) -> Result<QueryResult, dbtoon::error::DbtoonError> {
    backend.execute(sql, Some(limit), DEFAULT_TIMEOUT).await
}

/// Extract the text of the first cell from the first row.
/// Panics if the result is empty or the cell is null.
fn first_cell_text(result: &QueryResult) -> &str {
    match &result.rows[0][0] {
        CellValue::Text(s) => s.as_str(),
        CellValue::Null => panic!("expected Text, got Null"),
    }
}

// ---------------------------------------------------------------------------
// Connection & Auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn select_1_basic_query() {
    let backend = require_sqlserver!();
    let result = exec(&backend, "SELECT 1 AS x").await.unwrap();
    assert_eq!(result.columns.len(), 1);
    assert_eq!(result.columns[0].name, "x");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(first_cell_text(&result), "1");
    assert!(!result.truncated);
}

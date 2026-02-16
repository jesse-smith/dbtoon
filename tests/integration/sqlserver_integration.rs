use dbtoon::backend::sqlserver::SqlServerBackend;
use dbtoon::backend::{Backend, CellValue, QueryResult};
use dbtoon::config::SqlServerAuth;
use dbtoon::error::DbtoonError;
use secrecy::SecretString;
use std::env;
use std::fs;

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

/// Construct a backend with explicit credentials (for bad-auth tests).
fn backend_with_creds(user: &str, password: &str) -> Option<SqlServerBackend> {
    let host = env::var("TEST_SQLSERVER_HOST").ok()?;
    let port = env::var("TEST_SQLSERVER_PORT").unwrap_or_else(|_| "1433".into());

    let server = format!("{},{}", host, port);
    let auth = SqlServerAuth::SqlLogin {
        username: user.to_string(),
        password: SecretString::from(password.to_string()),
    };

    Some(SqlServerBackend::new(server, None, auth, true))
}

/// Early-return skip macro: if no SQL Server env is configured, print a
/// diagnostic and return so the test shows as "passed (skipped)".
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

/// Variant that returns an Option-based backend for custom credential tests.
macro_rules! require_sqlserver_env {
    () => {
        if env::var("TEST_SQLSERVER_HOST").is_err() {
            println!("SKIP: TEST_SQLSERVER_HOST not set — skipping integration test");
            return;
        }
    };
}

/// Default timeout for integration test queries (seconds).
const DEFAULT_TIMEOUT: u64 = 30;

/// Execute a query with no row limit.
async fn exec(
    backend: &SqlServerBackend,
    sql: &str,
) -> Result<QueryResult, DbtoonError> {
    backend.execute(sql, None, DEFAULT_TIMEOUT).await
}

/// Execute a query with a row limit.
async fn exec_limited(
    backend: &SqlServerBackend,
    sql: &str,
    limit: usize,
) -> Result<QueryResult, DbtoonError> {
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

/// Extract the cell value at (row, col).
fn cell_at(result: &QueryResult, row: usize, col: usize) -> &CellValue {
    &result.rows[row][col]
}

/// Extract the text at (row, col), panicking on Null.
fn text_at(result: &QueryResult, row: usize, col: usize) -> &str {
    match cell_at(result, row, col) {
        CellValue::Text(s) => s.as_str(),
        CellValue::Null => panic!("expected Text at ({}, {}), got Null", row, col),
    }
}

// ===========================================================================
// Connection & Auth (4 tests)
// ===========================================================================

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

#[tokio::test]
async fn trust_server_certificate_implicit() {
    // The container uses a self-signed cert. If trust_cert weren't working,
    // the connection would fail with a TLS error.
    let backend = require_sqlserver!();
    let result = exec(&backend, "SELECT @@VERSION AS v").await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert!(first_cell_text(&result).contains("Microsoft SQL Server"));
}

#[tokio::test]
async fn bad_credentials_returns_auth_error() {
    require_sqlserver_env!();
    let backend = backend_with_creds("not_a_real_user", "wrong_password").unwrap();
    let err = exec(&backend, "SELECT 1").await.unwrap_err();
    assert!(
        matches!(err, DbtoonError::Auth { .. }),
        "expected Auth error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn non_routable_host_returns_connection_error() {
    // 192.0.2.1 is TEST-NET-1 (RFC 5737) — guaranteed non-routable.
    let auth = SqlServerAuth::SqlLogin {
        username: "sa".to_string(),
        password: SecretString::from("whatever".to_string()),
    };
    let backend = SqlServerBackend::new("192.0.2.1,1433".to_string(), None, auth, true);
    let err = backend.execute("SELECT 1", None, 5).await.unwrap_err();
    assert!(
        matches!(err, DbtoonError::Connection { .. } | DbtoonError::Timeout { .. }),
        "expected Connection or Timeout error, got: {:?}",
        err
    );
}

// ===========================================================================
// Basic Queries (3 tests)
// ===========================================================================

#[tokio::test]
async fn multi_row_query() {
    let backend = require_sqlserver!();
    let result = exec(
        &backend,
        "SELECT v FROM (VALUES (1),(2),(3)) AS t(v)",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 3);
    assert_eq!(text_at(&result, 0, 0), "1");
    assert_eq!(text_at(&result, 1, 0), "2");
    assert_eq!(text_at(&result, 2, 0), "3");
}

#[tokio::test]
async fn multi_column_with_null() {
    let backend = require_sqlserver!();
    let result = exec(
        &backend,
        "SELECT 1 AS a, NULL AS b, N'hello' AS c",
    )
    .await
    .unwrap();
    assert_eq!(result.columns.len(), 3);
    assert_eq!(text_at(&result, 0, 0), "1");
    assert_eq!(*cell_at(&result, 0, 1), CellValue::Null);
    assert_eq!(text_at(&result, 0, 2), "hello");
}

#[tokio::test]
async fn empty_result_set() {
    let backend = require_sqlserver!();
    let result = exec(&backend, "SELECT 1 AS x WHERE 1=0").await.unwrap();
    assert_eq!(result.columns.len(), 1);
    assert_eq!(result.rows.len(), 0);
    assert!(!result.truncated);
}

// ===========================================================================
// Row Limiting (3 tests)
// ===========================================================================

#[tokio::test]
async fn limit_less_than_row_count() {
    let backend = require_sqlserver!();
    let result = exec_limited(
        &backend,
        "SELECT v FROM (VALUES (1),(2),(3),(4),(5)) AS t(v)",
        3,
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 3);
    assert!(result.truncated);
}

#[tokio::test]
async fn limit_greater_than_row_count() {
    let backend = require_sqlserver!();
    let result = exec_limited(
        &backend,
        "SELECT v FROM (VALUES (1),(2),(3)) AS t(v)",
        10,
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 3);
    assert!(!result.truncated);
}

#[tokio::test]
async fn limit_exact_boundary() {
    let backend = require_sqlserver!();
    let result = exec_limited(
        &backend,
        "SELECT v FROM (VALUES (1),(2),(3)) AS t(v)",
        3,
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 3);
    assert!(!result.truncated);
}

// ===========================================================================
// DMV Column Type Metadata (20 tests)
// ===========================================================================

/// Helper: run a CAST query and assert the DMV-reported type_name.
async fn assert_dmv_type(backend: &SqlServerBackend, cast_expr: &str, expected_type: &str) {
    let sql = format!("SELECT CAST({}) AS col", cast_expr);
    let result = exec(backend, &sql).await.unwrap();
    assert_eq!(
        result.columns[0].type_name, expected_type,
        "DMV type mismatch for CAST({}): got '{}', expected '{}'",
        cast_expr, result.columns[0].type_name, expected_type
    );
}

#[tokio::test]
async fn dmv_type_int() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1 AS INT", "INT").await;
}

#[tokio::test]
async fn dmv_type_bigint() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1 AS BIGINT", "BIGINT").await;
}

#[tokio::test]
async fn dmv_type_smallint() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1 AS SMALLINT", "SMALLINT").await;
}

#[tokio::test]
async fn dmv_type_tinyint() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1 AS TINYINT", "TINYINT").await;
}

#[tokio::test]
async fn dmv_type_bit() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1 AS BIT", "BIT").await;
}

#[tokio::test]
async fn dmv_type_decimal() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1.00 AS DECIMAL(10,2)", "DECIMAL(10,2)").await;
}

#[tokio::test]
async fn dmv_type_numeric() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1.0000 AS NUMERIC(18,4)", "NUMERIC(18,4)").await;
}

#[tokio::test]
async fn dmv_type_float() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1.0 AS FLOAT", "FLOAT").await;
}

#[tokio::test]
async fn dmv_type_real() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1.0 AS REAL", "REAL").await;
}

#[tokio::test]
async fn dmv_type_nvarchar_100() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "N'test' AS NVARCHAR(100)", "NVARCHAR(100)").await;
}

#[tokio::test]
async fn dmv_type_varchar_50() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "'test' AS VARCHAR(50)", "VARCHAR(50)").await;
}

#[tokio::test]
async fn dmv_type_nvarchar_max() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "N'test' AS NVARCHAR(MAX)", "NVARCHAR(MAX)").await;
}

#[tokio::test]
async fn dmv_type_datetime2() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "'2025-01-01' AS DATETIME2(3)", "DATETIME2(3)").await;
}

#[tokio::test]
async fn dmv_type_date() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "'2025-01-01' AS DATE", "DATE").await;
}

#[tokio::test]
async fn dmv_type_time() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "'12:30:00' AS TIME(7)", "TIME(7)").await;
}

#[tokio::test]
async fn dmv_type_uniqueidentifier() {
    let b = require_sqlserver!();
    assert_dmv_type(
        &b,
        "'A0EEBC99-9C0B-4EF8-BB6D-6BB9BD380A11' AS UNIQUEIDENTIFIER",
        "UNIQUEIDENTIFIER",
    )
    .await;
}

#[tokio::test]
async fn dmv_type_varbinary() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "0xDEAD AS VARBINARY(10)", "VARBINARY(10)").await;
}

#[tokio::test]
async fn dmv_type_xml() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "N'<root/>' AS XML", "XML").await;
}

#[tokio::test]
async fn dmv_type_money() {
    let b = require_sqlserver!();
    assert_dmv_type(&b, "1.00 AS MONEY", "MONEY").await;
}

#[tokio::test]
async fn dmv_type_datetimeoffset() {
    let b = require_sqlserver!();
    assert_dmv_type(
        &b,
        "'2025-01-01 12:00:00 +05:30' AS DATETIMEOFFSET(7)",
        "DATETIMEOFFSET(7)",
    )
    .await;
}

// ===========================================================================
// Value Rendering (16 tests)
// ===========================================================================

#[tokio::test]
async fn value_int() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(42 AS INT) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "42");
}

#[tokio::test]
async fn value_bigint() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(9999999999 AS BIGINT) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "9999999999");
}

#[tokio::test]
async fn value_bit_true() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(1 AS BIT) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "1");
}

#[tokio::test]
async fn value_bit_false() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(0 AS BIT) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "0");
}

#[tokio::test]
async fn value_decimal_trailing_zeros() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(1.50 AS DECIMAL(10,2)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "1.50");
}

#[tokio::test]
async fn value_nvarchar() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(N'hello world' AS NVARCHAR(100)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "hello world");
}

#[tokio::test]
async fn value_date() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST('2025-06-15' AS DATE) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "2025-06-15");
}

#[tokio::test]
async fn value_datetime2() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST('2025-06-15 14:30:00.123' AS DATETIME2(3)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "2025-06-15 14:30:00.123");
}

#[tokio::test]
async fn value_uniqueidentifier() {
    let b = require_sqlserver!();
    let r = exec(
        &b,
        "SELECT CAST('a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11' AS UNIQUEIDENTIFIER) AS v",
    )
    .await
    .unwrap();
    // GUIDs are lowercase hyphenated
    assert_eq!(first_cell_text(&r), "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11");
}

#[tokio::test]
async fn value_varbinary_hex() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(0xDEADBEEF AS VARBINARY(10)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "0xDEADBEEF");
}

#[tokio::test]
async fn value_null() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(NULL AS INT) AS v").await.unwrap();
    assert_eq!(*cell_at(&r, 0, 0), CellValue::Null);
}

#[tokio::test]
async fn value_money() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(19.99 AS MONEY) AS v").await.unwrap();
    // Money is transferred as Numeric by tiberius; check it has decimal places
    let text = first_cell_text(&r);
    assert!(
        text.contains("19.99"),
        "expected money value containing '19.99', got '{}'",
        text
    );
}

#[tokio::test]
async fn value_float() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(1.23456 AS FLOAT) AS v").await.unwrap();
    let val: f64 = first_cell_text(&r).parse().unwrap();
    assert!((val - 1.23456).abs() < 0.001);
}

#[tokio::test]
async fn value_xml() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(N'<root>data</root>' AS XML) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "<root>data</root>");
}

#[tokio::test]
async fn value_datetimeoffset() {
    let b = require_sqlserver!();
    // Use UTC offset so the time component is unambiguous (no UTC conversion shift)
    let r = exec(
        &b,
        "SELECT CAST('2025-06-15 14:30:00.1234567 +00:00' AS DATETIMEOFFSET(7)) AS v",
    )
    .await
    .unwrap();
    let text = first_cell_text(&r);
    assert!(
        text.starts_with("2025-06-15 14:30:00.1234567"),
        "expected datetimeoffset starting with '2025-06-15 14:30:00.1234567', got '{}'",
        text
    );
    assert!(
        text.contains("+00:00"),
        "expected offset '+00:00' in '{}'",
        text
    );
}

#[tokio::test]
async fn value_time() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST('14:30:00.1234567' AS TIME(7)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "14:30:00.1234567");
}

#[tokio::test]
async fn value_empty_string() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT CAST(N'' AS NVARCHAR(10)) AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "");
}

// ===========================================================================
// Error Cases (3 tests)
// ===========================================================================

#[tokio::test]
async fn syntax_error_returns_query_error() {
    let b = require_sqlserver!();
    let err = exec(&b, "SELECTTTT 1").await.unwrap_err();
    assert!(
        matches!(err, DbtoonError::Query { .. }),
        "expected Query error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn timeout_via_waitfor_delay() {
    let b = require_sqlserver!();
    // WAITFOR DELAY 10 seconds with 2-second timeout
    let err = b
        .execute("WAITFOR DELAY '00:00:10'", None, 2)
        .await
        .unwrap_err();
    assert!(
        matches!(err, DbtoonError::Timeout { .. }),
        "expected Timeout error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn nonexistent_table_returns_query_error() {
    let b = require_sqlserver!();
    let err = exec(&b, "SELECT * FROM dbo.nonexistent_table_xyz_12345").await.unwrap_err();
    assert!(
        matches!(err, DbtoonError::Query { .. }),
        "expected Query error, got: {:?}",
        err
    );
}

// ===========================================================================
// Unicode & Edge Cases (3 tests)
// ===========================================================================

#[tokio::test]
async fn unicode_cjk_characters() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT N'\u{4F60}\u{597D}\u{4E16}\u{754C}' AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "\u{4F60}\u{597D}\u{4E16}\u{754C}");
}

#[tokio::test]
async fn empty_string_value() {
    let b = require_sqlserver!();
    let r = exec(&b, "SELECT N'' AS v").await.unwrap();
    assert_eq!(first_cell_text(&r), "");
}

#[tokio::test]
async fn twenty_column_query() {
    let b = require_sqlserver!();
    let cols: Vec<String> = (1..=20).map(|i| format!("{} AS c{}", i, i)).collect();
    let sql = format!("SELECT {}", cols.join(", "));
    let r = exec(&b, &sql).await.unwrap();
    assert_eq!(r.columns.len(), 20);
    for i in 0..20 {
        assert_eq!(r.columns[i].name, format!("c{}", i + 1));
        assert_eq!(text_at(&r, 0, i), format!("{}", i + 1));
    }
}

// ===========================================================================
// Memory Benchmark (1 test, gated on TEST_SQLSERVER_BENCH=1)
// ===========================================================================

/// Read VmRSS from /proc/self/status (Linux only). Returns None on other platforms.
fn read_rss_kb() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb_str = rest.trim().strip_suffix("kB")?.trim();
            return kb_str.parse().ok();
        }
    }
    None
}

#[tokio::test]
async fn memory_benchmark_100k_rows() {
    if env::var("TEST_SQLSERVER_BENCH").as_deref() != Ok("1") {
        println!("SKIP: TEST_SQLSERVER_BENCH=1 not set — skipping memory benchmark");
        return;
    }
    let b = require_sqlserver!();

    // CTE generating 100k rows: cross join of two 317-row sequences
    // 317 * 317 = 100489 rows > 100k
    let sql = "\
        WITH nums AS ( \
            SELECT TOP 317 ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) AS n \
            FROM sys.all_columns \
        ) \
        SELECT a.n AS id, \
               CAST(a.n AS NVARCHAR(20)) AS label, \
               CAST(a.n * 1.5 AS DECIMAL(10,2)) AS amount \
        FROM nums a CROSS JOIN nums b";

    let rss_before = read_rss_kb();

    let result = b.execute(sql, None, 120).await.unwrap();

    let rss_after = read_rss_kb();

    println!("benchmark: {} rows returned", result.rows.len());
    assert!(
        result.rows.len() >= 100_000,
        "expected >=100k rows, got {}",
        result.rows.len()
    );

    if let (Some(before), Some(after)) = (rss_before, rss_after) {
        let delta_mb = (after.saturating_sub(before)) as f64 / 1024.0;
        println!(
            "benchmark: RSS before={} kB, after={} kB, delta={:.1} MB",
            before, after, delta_mb
        );
        if delta_mb > 200.0 {
            println!(
                "WARNING: RSS delta {:.1} MB exceeds 200 MB soft threshold",
                delta_mb
            );
        }
    } else {
        println!("benchmark: /proc/self/status not available (non-Linux); RSS measurement skipped");
    }
}

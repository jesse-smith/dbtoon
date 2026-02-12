use dbtoon::backend::sqlserver::normalize_odbc_type;
use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format::{to_toon, to_toon_kv};
use odbc_api::DataType;
use std::num::NonZeroUsize;

#[test]
fn test_3_column_2_row_result() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
            ColumnMeta { name: "name".to_string(), type_name: "VARCHAR".to_string() },
            ColumnMeta { name: "email".to_string(), type_name: "VARCHAR".to_string() },
        ],
        rows: vec![
            vec![
                CellValue::Text("1".to_string()),
                CellValue::Text("Alice".to_string()),
                CellValue::Text("alice@co.com".to_string()),
            ],
            vec![
                CellValue::Text("2".to_string()),
                CellValue::Text("Bob".to_string()),
                CellValue::Text("bob@co.com".to_string()),
            ],
        ],
        total_rows: None,
        truncated: false,
    };

    let toon = to_toon(&result).unwrap();
    // Should contain tabular format markers
    assert!(toon.contains("[2]"), "Should contain row count [2], got: {}", toon);
    assert!(toon.contains("id"), "Should contain column 'id'");
    assert!(toon.contains("name"), "Should contain column 'name'");
    assert!(toon.contains("email"), "Should contain column 'email'");
    assert!(toon.contains("Alice"), "Should contain value 'Alice'");
    assert!(toon.contains("Bob"), "Should contain value 'Bob'");
}

#[test]
fn test_zero_row_result() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "col1".to_string(), type_name: "INT".to_string() },
            ColumnMeta { name: "col2".to_string(), type_name: "VARCHAR".to_string() },
        ],
        rows: vec![],
        total_rows: None,
        truncated: false,
    };

    let toon = to_toon(&result).unwrap();
    assert!(toon.contains("[0]"), "Should contain [0] for zero rows, got: {}", toon);
    assert!(toon.contains("col1"), "Should contain column 'col1'");
    assert!(toon.contains("col2"), "Should contain column 'col2'");
}

#[test]
fn test_null_cell_value() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "val".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![
            vec![CellValue::Null],
        ],
        total_rows: None,
        truncated: false,
    };

    let toon = to_toon(&result).unwrap();
    assert!(toon.contains("null"), "Should contain 'null' for NULL value, got: {}", toon);
}

#[test]
fn test_single_column_single_row() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "count".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![
            vec![CellValue::Text("42".to_string())],
        ],
        total_rows: None,
        truncated: false,
    };

    let toon = to_toon(&result).unwrap();
    assert!(toon.contains("[1]"), "Should contain [1], got: {}", toon);
    assert!(toon.contains("42"), "Should contain value '42'");
}

#[test]
fn test_toon_kv() {
    let kv = to_toon_kv(&[("truncated", "true"), ("message", "Showing 500 rows.")]);
    assert_eq!(kv, "truncated: true\nmessage: Showing 500 rows.");
}

// --- normalize_odbc_type tests ---

#[test]
fn test_normalize_simple_types() {
    assert_eq!(normalize_odbc_type(&DataType::Integer), "INT");
    assert_eq!(normalize_odbc_type(&DataType::SmallInt), "SMALLINT");
    assert_eq!(normalize_odbc_type(&DataType::BigInt), "BIGINT");
    assert_eq!(normalize_odbc_type(&DataType::TinyInt), "TINYINT");
    assert_eq!(normalize_odbc_type(&DataType::Real), "REAL");
    assert_eq!(normalize_odbc_type(&DataType::Double), "FLOAT");
    assert_eq!(normalize_odbc_type(&DataType::Date), "DATE");
    assert_eq!(normalize_odbc_type(&DataType::Bit), "BIT");
}

#[test]
fn test_normalize_types_with_length() {
    let n10 = NonZeroUsize::new(10).unwrap();
    let n16 = NonZeroUsize::new(16).unwrap();
    let n100 = NonZeroUsize::new(100).unwrap();
    let n255 = NonZeroUsize::new(255).unwrap();

    assert_eq!(normalize_odbc_type(&DataType::Char { length: Some(n10) }), "CHAR(10)");
    assert_eq!(normalize_odbc_type(&DataType::Char { length: None }), "CHAR");
    assert_eq!(normalize_odbc_type(&DataType::WChar { length: Some(n10) }), "NCHAR(10)");
    assert_eq!(normalize_odbc_type(&DataType::WChar { length: None }), "NCHAR");
    assert_eq!(normalize_odbc_type(&DataType::Varchar { length: Some(n255) }), "VARCHAR(255)");
    assert_eq!(normalize_odbc_type(&DataType::WVarchar { length: Some(n100) }), "NVARCHAR(100)");
    assert_eq!(normalize_odbc_type(&DataType::Binary { length: Some(n16) }), "BINARY(16)");
    assert_eq!(normalize_odbc_type(&DataType::Binary { length: None }), "BINARY");
    assert_eq!(normalize_odbc_type(&DataType::Varbinary { length: Some(n100) }), "VARBINARY(100)");
}

#[test]
fn test_normalize_max_types() {
    let n1000 = NonZeroUsize::new(1000).unwrap();

    assert_eq!(normalize_odbc_type(&DataType::Varchar { length: None }), "VARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::WVarchar { length: None }), "NVARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::Varbinary { length: None }), "VARBINARY(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::LongVarchar { length: Some(n1000) }), "VARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::LongVarchar { length: None }), "VARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::WLongVarchar { length: Some(n1000) }), "NVARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::WLongVarchar { length: None }), "NVARCHAR(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::LongVarbinary { length: Some(n1000) }), "VARBINARY(MAX)");
    assert_eq!(normalize_odbc_type(&DataType::LongVarbinary { length: None }), "VARBINARY(MAX)");
}

#[test]
fn test_normalize_precision_scale_types() {
    assert_eq!(normalize_odbc_type(&DataType::Decimal { precision: 10, scale: 2 }), "DECIMAL(10,2)");
    assert_eq!(normalize_odbc_type(&DataType::Numeric { precision: 18, scale: 0 }), "NUMERIC(18,0)");
}

#[test]
fn test_normalize_precision_types() {
    assert_eq!(normalize_odbc_type(&DataType::Float { precision: 53 }), "FLOAT(53)");
    assert_eq!(normalize_odbc_type(&DataType::Time { precision: 7 }), "TIME(7)");
    assert_eq!(normalize_odbc_type(&DataType::Timestamp { precision: 3 }), "DATETIME2(3)");
}

#[test]
fn test_normalize_unknown_fallback() {
    assert_eq!(normalize_odbc_type(&DataType::Unknown), "UNKNOWN");
    assert_eq!(
        normalize_odbc_type(&DataType::Other {
            data_type: odbc_api::sys::SqlDataType(-9999),
            column_size: None,
            decimal_digits: 0,
        }),
        "UNKNOWN"
    );
}

use dbtoon::backend::sqlserver::normalize_odbc_type;
use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format::{to_toon, to_toon_kv};
use odbc_api::DataType;
use std::num::NonZeroUsize;

/// Helper: encode to TOON and decode back to serde_json::Value (no type coercion)
fn round_trip(result: &QueryResult) -> serde_json::Value {
    let toon = to_toon(result).unwrap();
    toon_format::decode_no_coerce(&toon).unwrap()
}

#[test]
fn test_3_column_2_row_result() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
            ColumnMeta { name: "name".to_string(), type_name: "VARCHAR(255)".to_string() },
            ColumnMeta { name: "email".to_string(), type_name: "VARCHAR(255)".to_string() },
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

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");

    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");
    assert_eq!(types.len(), 3);
    assert_eq!(types[0], "INT");
    assert_eq!(types[1], "VARCHAR(255)");
    assert_eq!(types[2], "VARCHAR(255)");

    let rows = obj.get("rows").expect("should have 'rows' key")
        .as_array().expect("rows should be an array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "1");
    assert_eq!(rows[0]["name"], "Alice");
    assert_eq!(rows[0]["email"], "alice@co.com");
    assert_eq!(rows[1]["id"], "2");
    assert_eq!(rows[1]["name"], "Bob");
    assert_eq!(rows[1]["email"], "bob@co.com");
}

#[test]
fn test_zero_row_result() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "col1".to_string(), type_name: "INT".to_string() },
            ColumnMeta { name: "col2".to_string(), type_name: "VARCHAR(100)".to_string() },
        ],
        rows: vec![],
        total_rows: None,
        truncated: false,
    };

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");

    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");
    assert_eq!(types.len(), 2);
    assert_eq!(types[0], "INT");
    assert_eq!(types[1], "VARCHAR(100)");

    let rows = obj.get("rows").expect("should have 'rows' key")
        .as_array().expect("rows should be an array");
    assert!(rows.is_empty(), "rows should be empty for zero-row result");
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

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");

    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0], "INT");

    let rows = obj.get("rows").expect("should have 'rows' key")
        .as_array().expect("rows should be an array");
    assert_eq!(rows.len(), 1);
    assert!(rows[0]["val"].is_null(), "NULL cell should decode as null");
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

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");

    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");
    assert_eq!(types.len(), 1);
    assert_eq!(types[0], "INT");

    let rows = obj.get("rows").expect("should have 'rows' key")
        .as_array().expect("rows should be an array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["count"], "42");
}

#[test]
fn test_toon_kv() {
    let kv = to_toon_kv(&[("truncated", "true"), ("message", "Showing 500 rows.")]);
    assert_eq!(kv, "truncated: true\nmessage: Showing 500 rows.");
}

// --- US2: End-to-end normalization verification ---

#[test]
fn test_sqlserver_normalized_types_in_output() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
            ColumnMeta { name: "name".to_string(), type_name: "VARCHAR(255)".to_string() },
            ColumnMeta { name: "balance".to_string(), type_name: "DECIMAL(18,2)".to_string() },
            ColumnMeta { name: "created".to_string(), type_name: "DATETIME2(7)".to_string() },
            ColumnMeta { name: "notes".to_string(), type_name: "NVARCHAR(MAX)".to_string() },
        ],
        rows: vec![
            vec![
                CellValue::Text("1".to_string()),
                CellValue::Text("Alice".to_string()),
                CellValue::Text("1234.56".to_string()),
                CellValue::Text("2026-01-01".to_string()),
                CellValue::Text("test".to_string()),
            ],
        ],
        total_rows: None,
        truncated: false,
    };

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");
    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");

    assert_eq!(types[0], "INT");
    assert_eq!(types[1], "VARCHAR(255)");
    assert_eq!(types[2], "DECIMAL(18,2)");
    assert_eq!(types[3], "DATETIME2(7)");
    assert_eq!(types[4], "NVARCHAR(MAX)");
}

#[test]
fn test_databricks_passthrough_types_in_output() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "BIGINT".to_string() },
            ColumnMeta { name: "name".to_string(), type_name: "STRING".to_string() },
            ColumnMeta { name: "price".to_string(), type_name: "DECIMAL(10,2)".to_string() },
            ColumnMeta { name: "active".to_string(), type_name: "BOOLEAN".to_string() },
            ColumnMeta { name: "tags".to_string(), type_name: "ARRAY<STRING>".to_string() },
        ],
        rows: vec![
            vec![
                CellValue::Text("100".to_string()),
                CellValue::Text("Widget".to_string()),
                CellValue::Text("9.99".to_string()),
                CellValue::Text("true".to_string()),
                CellValue::Text("[\"a\",\"b\"]".to_string()),
            ],
        ],
        total_rows: None,
        truncated: false,
    };

    let decoded = round_trip(&result);
    let obj = decoded.as_object().expect("output should be a root object");
    let types = obj.get("types").expect("should have 'types' key")
        .as_array().expect("types should be an array");

    // FR-006: Databricks types pass through unchanged
    assert_eq!(types[0], "BIGINT");
    assert_eq!(types[1], "STRING");
    assert_eq!(types[2], "DECIMAL(10,2)");
    assert_eq!(types[3], "BOOLEAN");
    assert_eq!(types[4], "ARRAY<STRING>");
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

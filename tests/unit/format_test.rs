use dbtoon::backend::sqlserver::normalize_odbc_type;
use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format::to_toon;
use odbc_api::DataType;
use std::num::NonZeroUsize;

/// Helper: encode to TOON and decode back to serde_json::Value (no type coercion)
fn round_trip(result: &QueryResult) -> serde_json::Value {
    let toon = to_toon(result, false, None).unwrap();
    toon_format::decode_no_coerce(&toon).unwrap()
}

/// Helper: encode with truncation args and decode back
fn round_trip_with_truncation(
    result: &QueryResult,
    truncated: bool,
    message: Option<&str>,
) -> serde_json::Value {
    let toon = to_toon(result, truncated, message).unwrap();
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

// --- T002: Truncated TOON output contains truncated + message keys ---

#[test]
fn test_truncated_toon_has_truncated_true_and_message() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![
            vec![CellValue::Text("1".to_string())],
            vec![CellValue::Text("2".to_string())],
        ],
        total_rows: None,
        truncated: true,
    };

    let message = "Showing 2 rows. Use --no-limit to return all rows.";
    let decoded = round_trip_with_truncation(&result, true, Some(message));
    let obj = decoded.as_object().expect("output should be a root object");

    // truncated key must be present and true
    let truncated_val = obj.get("truncated").expect("should have 'truncated' key");
    assert_eq!(truncated_val, &serde_json::Value::Bool(true));

    // message key must be present with the expected text
    let message_val = obj.get("message").expect("should have 'message' key");
    assert_eq!(message_val, message);

    // types and rows must still be present
    assert!(obj.contains_key("types"));
    assert!(obj.contains_key("rows"));
}

#[test]
fn test_truncated_toon_zero_rows_edge_case() {
    // Edge case per spec: truncated=true but zero rows (e.g., server returned 0 rows with truncation flag)
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![],
        total_rows: None,
        truncated: true,
    };

    let message = "Showing 0 rows. Use --no-limit to return all rows.";
    let decoded = round_trip_with_truncation(&result, true, Some(message));
    let obj = decoded.as_object().expect("output should be a root object");

    let truncated_val = obj.get("truncated").expect("should have 'truncated' key");
    assert_eq!(truncated_val, &serde_json::Value::Bool(true));

    let message_val = obj.get("message").expect("should have 'message' key");
    assert_eq!(message_val, message);
}

// --- T003: Non-truncated TOON output contains truncated=false and NO message key ---

#[test]
fn test_non_truncated_toon_has_truncated_false_and_no_message() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![
            vec![CellValue::Text("1".to_string())],
        ],
        total_rows: None,
        truncated: false,
    };

    let decoded = round_trip_with_truncation(&result, false, None);
    let obj = decoded.as_object().expect("output should be a root object");

    // truncated key must be present and false
    let truncated_val = obj.get("truncated").expect("should have 'truncated' key");
    assert_eq!(truncated_val, &serde_json::Value::Bool(false));

    // message key must NOT be present
    assert!(!obj.contains_key("message"), "non-truncated output should not have 'message' key");

    // types and rows must still be present
    assert!(obj.contains_key("types"));
    assert!(obj.contains_key("rows"));
}

// --- T015: Round-trip test: encode with truncation, write to temp .toon file, read back ---

#[test]
fn test_toon_file_round_trip_with_truncation() {
    let result = QueryResult {
        columns: vec![
            ColumnMeta { name: "id".to_string(), type_name: "INT".to_string() },
        ],
        rows: vec![
            vec![CellValue::Text("1".to_string())],
            vec![CellValue::Text("2".to_string())],
        ],
        total_rows: None,
        truncated: true,
    };

    let message = "Showing 2 rows. Use --no-limit to return all rows.";
    let toon = to_toon(&result, true, Some(message)).unwrap();

    // Write to temp file
    let dir = std::env::temp_dir().join("dbtoon_test_toon");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("round_trip_trunc.toon");
    std::fs::write(&path, &toon).unwrap();

    // Read back and decode
    let content = std::fs::read_to_string(&path).unwrap();
    let decoded: serde_json::Value = toon_format::decode_no_coerce(&content).unwrap();
    let obj = decoded.as_object().expect("should be a root object");

    assert_eq!(obj.get("truncated").unwrap(), &serde_json::Value::Bool(true));
    assert_eq!(obj.get("message").unwrap(), message);
    assert!(obj.contains_key("types"));
    assert!(obj.contains_key("rows"));

    let _ = std::fs::remove_file(&path);
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

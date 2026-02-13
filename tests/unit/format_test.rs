use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format::to_toon;

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


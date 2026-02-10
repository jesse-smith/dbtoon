use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format::{to_toon, to_toon_kv};

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

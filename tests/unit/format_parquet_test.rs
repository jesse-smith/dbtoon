use std::collections::HashMap;
use std::fs;

use arrow::array::RecordBatchReader;
use arrow::datatypes::DataType;
use parquet::arrow::arrow_reader::ParquetRecordBatchReader;
use parquet::file::reader::{FileReader as ParquetFileReader, SerializedFileReader};

use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format_parquet::write_parquet;

fn make_column(name: &str, type_name: &str) -> ColumnMeta {
    ColumnMeta {
        name: name.to_string(),
        type_name: type_name.to_string(),
    }
}

fn make_result(columns: Vec<ColumnMeta>, rows: Vec<Vec<CellValue>>) -> QueryResult {
    let total_rows = Some(rows.len());
    QueryResult {
        columns,
        rows,
        total_rows,
        truncated: false,
    }
}

fn temp_parquet_path(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("dbtoon_test_parquet");
    fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{test_name}.parquet"))
}

#[test]
fn write_and_read_back_parquet_with_typed_columns() {
    let result = make_result(
        vec![
            make_column("id", "INT"),
            make_column("name", "VARCHAR(100)"),
            make_column("salary", "FLOAT"),
        ],
        vec![
            vec![
                CellValue::Text("1".into()),
                CellValue::Text("Alice".into()),
                CellValue::Text("50000.5".into()),
            ],
            vec![
                CellValue::Text("2".into()),
                CellValue::Text("Bob".into()),
                CellValue::Text("60000.75".into()),
            ],
        ],
    );

    let path = temp_parquet_path("typed_columns");
    write_parquet(&result, &path, false, None).unwrap();

    // Read back
    let file = fs::File::open(&path).unwrap();
    let reader = ParquetRecordBatchReader::try_new(file, 1024).unwrap();
    let schema = reader.schema();

    // Verify column names and types
    assert_eq!(schema.field(0).name(), "id");
    assert_eq!(schema.field(0).data_type(), &DataType::Int32);
    assert_eq!(schema.field(1).name(), "name");
    assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
    assert_eq!(schema.field(2).name(), "salary");
    assert_eq!(schema.field(2).data_type(), &DataType::Float64);

    // Verify values
    let batches: Vec<_> = reader.into_iter().collect::<Result<_, _>>().unwrap();
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 2);

    let batch = &batches[0];
    let id_col = batch
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::Int32Array>()
        .unwrap();
    assert_eq!(id_col.value(0), 1);
    assert_eq!(id_col.value(1), 2);

    let name_col = batch
        .column(1)
        .as_any()
        .downcast_ref::<arrow::array::StringArray>()
        .unwrap();
    assert_eq!(name_col.value(0), "Alice");
    assert_eq!(name_col.value(1), "Bob");

    // Cleanup
    let _ = fs::remove_file(&path);
}

#[test]
fn null_values_are_native_parquet_nulls() {
    let result = make_result(
        vec![
            make_column("id", "INT"),
            make_column("name", "VARCHAR(50)"),
        ],
        vec![
            vec![CellValue::Text("1".into()), CellValue::Null],
            vec![CellValue::Null, CellValue::Text("Bob".into())],
        ],
    );

    let path = temp_parquet_path("null_values");
    write_parquet(&result, &path, false, None).unwrap();

    let file = fs::File::open(&path).unwrap();
    let reader = ParquetRecordBatchReader::try_new(file, 1024).unwrap();
    let batches: Vec<_> = reader.into_iter().collect::<Result<_, _>>().unwrap();
    let batch = &batches[0];

    // id column: row 0 = 1 (valid), row 1 = null
    let id_col = batch.column(0);
    assert!(!id_col.is_null(0));
    assert!(id_col.is_null(1));

    // name column: row 0 = null, row 1 = "Bob"
    let name_col = batch.column(1);
    assert!(name_col.is_null(0));
    assert!(!name_col.is_null(1));

    let _ = fs::remove_file(&path);
}

#[test]
fn empty_result_produces_valid_parquet_with_schema() {
    let result = make_result(
        vec![
            make_column("id", "INT"),
            make_column("name", "VARCHAR(50)"),
        ],
        vec![],
    );

    let path = temp_parquet_path("empty_result");
    write_parquet(&result, &path, false, None).unwrap();

    let file = fs::File::open(&path).unwrap();
    let reader = ParquetRecordBatchReader::try_new(file, 1024).unwrap();
    let schema = reader.schema();

    assert_eq!(schema.fields().len(), 2);
    assert_eq!(schema.field(0).name(), "id");
    assert_eq!(schema.field(1).name(), "name");

    let batches: Vec<_> = reader.into_iter().collect::<Result<_, _>>().unwrap();
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 0);

    let _ = fs::remove_file(&path);
}

#[test]
fn string_fallback_columns_stored_as_utf8() {
    // Column declared as INT but has unparseable values → should fall back to Utf8
    let result = make_result(
        vec![make_column("weird", "INT")],
        vec![
            vec![CellValue::Text("42".into())],
            vec![CellValue::Text("not_a_number".into())],
        ],
    );

    let path = temp_parquet_path("fallback_utf8");
    write_parquet(&result, &path, false, None).unwrap();

    let file = fs::File::open(&path).unwrap();
    let reader = ParquetRecordBatchReader::try_new(file, 1024).unwrap();
    let schema = reader.schema();

    // Column should have fallen back to Utf8
    assert_eq!(schema.field(0).data_type(), &DataType::Utf8);

    let batches: Vec<_> = reader.into_iter().collect::<Result<_, _>>().unwrap();
    let batch = &batches[0];
    let col = batch
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::StringArray>()
        .unwrap();
    assert_eq!(col.value(0), "42");
    assert_eq!(col.value(1), "not_a_number");

    let _ = fs::remove_file(&path);
}

// --- T006: Truncation metadata in Parquet files ---

fn read_parquet_kv_metadata(path: &std::path::Path) -> HashMap<String, String> {
    let file = fs::File::open(path).unwrap();
    let reader = SerializedFileReader::new(file).unwrap();
    let file_meta = reader.metadata().file_metadata();
    let mut map = HashMap::new();
    if let Some(kv_meta) = file_meta.key_value_metadata() {
        for kv in kv_meta {
            if let Some(ref v) = kv.value {
                map.insert(kv.key.clone(), v.clone());
            }
        }
    }
    map
}

#[test]
fn truncated_parquet_has_dbtoon_metadata() {
    let result = make_result(
        vec![make_column("id", "INT")],
        vec![vec![CellValue::Text("1".into())]],
    );

    let path = temp_parquet_path("truncated_meta");
    let message = "Showing 1 rows. Use --no-limit to return all rows.";
    write_parquet(&result, &path, true, Some(message)).unwrap();

    let meta = read_parquet_kv_metadata(&path);
    assert_eq!(meta.get("dbtoon:truncated").map(String::as_str), Some("true"));
    assert_eq!(meta.get("dbtoon:message").map(String::as_str), Some(message));

    let _ = fs::remove_file(&path);
}

#[test]
fn non_truncated_parquet_has_no_dbtoon_metadata() {
    let result = make_result(
        vec![make_column("id", "INT")],
        vec![vec![CellValue::Text("1".into())]],
    );

    let path = temp_parquet_path("non_truncated_meta");
    write_parquet(&result, &path, false, None).unwrap();

    let meta = read_parquet_kv_metadata(&path);
    assert!(!meta.contains_key("dbtoon:truncated"), "non-truncated should not have dbtoon:truncated");
    assert!(!meta.contains_key("dbtoon:message"), "non-truncated should not have dbtoon:message");

    let _ = fs::remove_file(&path);
}

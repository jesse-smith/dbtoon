use arrow::datatypes::DataType;
use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format_columnar::{build_record_batch, sql_type_to_arrow};

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

// --- sql_type_to_arrow tests ---

#[test]
fn int_maps_to_int32() {
    assert_eq!(sql_type_to_arrow("INT"), DataType::Int32);
}

#[test]
fn integer_maps_to_int32() {
    assert_eq!(sql_type_to_arrow("INTEGER"), DataType::Int32);
}

#[test]
fn smallint_maps_to_int16() {
    assert_eq!(sql_type_to_arrow("SMALLINT"), DataType::Int16);
}

#[test]
fn bigint_maps_to_int64() {
    assert_eq!(sql_type_to_arrow("BIGINT"), DataType::Int64);
}

#[test]
fn tinyint_maps_to_uint8() {
    assert_eq!(sql_type_to_arrow("TINYINT"), DataType::UInt8);
}

#[test]
fn bit_maps_to_boolean() {
    assert_eq!(sql_type_to_arrow("BIT"), DataType::Boolean);
}

#[test]
fn boolean_maps_to_boolean() {
    assert_eq!(sql_type_to_arrow("BOOLEAN"), DataType::Boolean);
}

#[test]
fn real_maps_to_float32() {
    assert_eq!(sql_type_to_arrow("REAL"), DataType::Float32);
}

#[test]
fn float_maps_to_float64() {
    assert_eq!(sql_type_to_arrow("FLOAT"), DataType::Float64);
}

#[test]
fn float_with_precision_maps_to_float64() {
    assert_eq!(sql_type_to_arrow("FLOAT(53)"), DataType::Float64);
}

#[test]
fn varchar_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("VARCHAR(255)"), DataType::Utf8);
}

#[test]
fn nvarchar_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("NVARCHAR(100)"), DataType::Utf8);
}

#[test]
fn char_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("CHAR(10)"), DataType::Utf8);
}

#[test]
fn nchar_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("NCHAR(10)"), DataType::Utf8);
}

#[test]
fn string_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("STRING"), DataType::Utf8);
}

#[test]
fn varchar_max_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("VARCHAR(MAX)"), DataType::Utf8);
}

#[test]
fn nvarchar_max_maps_to_utf8() {
    assert_eq!(sql_type_to_arrow("NVARCHAR(MAX)"), DataType::Utf8);
}

#[test]
fn decimal_maps_to_decimal128() {
    assert_eq!(
        sql_type_to_arrow("DECIMAL(10,2)"),
        DataType::Decimal128(10, 2)
    );
}

#[test]
fn numeric_maps_to_decimal128() {
    assert_eq!(
        sql_type_to_arrow("NUMERIC(18,4)"),
        DataType::Decimal128(18, 4)
    );
}

#[test]
fn date_maps_to_date32() {
    assert_eq!(sql_type_to_arrow("DATE"), DataType::Date32);
}

#[test]
fn datetime2_maps_to_timestamp_microsecond() {
    assert_eq!(
        sql_type_to_arrow("DATETIME2(7)"),
        DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None)
    );
}

#[test]
fn timestamp_maps_to_timestamp_microsecond() {
    assert_eq!(
        sql_type_to_arrow("TIMESTAMP"),
        DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None)
    );
}

#[test]
fn time_maps_to_time64_microsecond() {
    assert_eq!(
        sql_type_to_arrow("TIME(7)"),
        DataType::Time64(arrow::datatypes::TimeUnit::Microsecond)
    );
}

#[test]
fn binary_maps_to_binary() {
    assert_eq!(sql_type_to_arrow("BINARY(16)"), DataType::Binary);
}

#[test]
fn varbinary_maps_to_binary() {
    assert_eq!(sql_type_to_arrow("VARBINARY(256)"), DataType::Binary);
}

#[test]
fn varbinary_max_maps_to_binary() {
    assert_eq!(sql_type_to_arrow("VARBINARY(MAX)"), DataType::Binary);
}

#[test]
fn unknown_type_falls_back_to_utf8() {
    assert_eq!(sql_type_to_arrow("GEOMETRY"), DataType::Utf8);
}

#[test]
fn exotic_type_falls_back_to_utf8() {
    assert_eq!(sql_type_to_arrow("XML"), DataType::Utf8);
}

#[test]
fn case_insensitive_mapping() {
    assert_eq!(sql_type_to_arrow("int"), DataType::Int32);
    assert_eq!(sql_type_to_arrow("bigint"), DataType::Int64);
    assert_eq!(sql_type_to_arrow("varchar(50)"), DataType::Utf8);
}

#[test]
fn whitespace_trimming() {
    assert_eq!(sql_type_to_arrow("  INT  "), DataType::Int32);
}

// --- build_record_batch tests ---

#[test]
fn build_batch_with_mixed_types() {
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

    let (_schema, batch) = build_record_batch(&result).unwrap();
    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.num_columns(), 3);

    // Check column types
    assert_eq!(batch.schema().field(0).data_type(), &DataType::Int32);
    assert_eq!(batch.schema().field(1).data_type(), &DataType::Utf8);
    assert_eq!(batch.schema().field(2).data_type(), &DataType::Float64);
}

#[test]
fn build_batch_null_values_produce_nulls() {
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

    let (_schema, batch) = build_record_batch(&result).unwrap();
    assert_eq!(batch.num_rows(), 2);

    // Check null flags
    let id_col = batch.column(0);
    assert!(!id_col.is_null(0)); // "1" → valid
    assert!(id_col.is_null(1)); // Null → null

    let name_col = batch.column(1);
    assert!(name_col.is_null(0)); // Null → null
    assert!(!name_col.is_null(1)); // "Bob" → valid
}

#[test]
fn build_batch_column_fallback_on_parse_failure() {
    // A column declared as INT but containing unparseable values should fall back to Utf8
    let result = make_result(
        vec![make_column("weird_int", "INT")],
        vec![
            vec![CellValue::Text("42".into())],
            vec![CellValue::Text("not_a_number".into())],
            vec![CellValue::Text("99".into())],
        ],
    );

    let (_schema, batch) = build_record_batch(&result).unwrap();
    // The entire column should have fallen back to Utf8
    assert_eq!(batch.schema().field(0).data_type(), &DataType::Utf8);
    assert_eq!(batch.num_rows(), 3);

    // Values should be preserved as strings
    let col = batch
        .column(0)
        .as_any()
        .downcast_ref::<arrow::array::StringArray>()
        .expect("column should be StringArray after fallback");
    assert_eq!(col.value(0), "42");
    assert_eq!(col.value(1), "not_a_number");
    assert_eq!(col.value(2), "99");
}

#[test]
fn build_batch_empty_result_set() {
    let result = make_result(
        vec![
            make_column("id", "INT"),
            make_column("name", "VARCHAR(50)"),
        ],
        vec![], // no rows
    );

    let (schema, batch) = build_record_batch(&result).unwrap();
    assert_eq!(batch.num_rows(), 0);
    assert_eq!(schema.fields().len(), 2);
    assert_eq!(schema.field(0).data_type(), &DataType::Int32);
    assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
}

use dbtoon::backend::{CellValue, ColumnMeta, QueryResult};
use dbtoon::format_csv::write_csv_to_writer;

fn make_column(name: &str) -> ColumnMeta {
    ColumnMeta {
        name: name.to_string(),
        type_name: "VARCHAR".to_string(),
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

#[test]
fn basic_output_header_and_data_rows() {
    let result = make_result(
        vec![make_column("id"), make_column("name"), make_column("city")],
        vec![
            vec![
                CellValue::Text("1".into()),
                CellValue::Text("Alice".into()),
                CellValue::Text("Portland".into()),
            ],
            vec![
                CellValue::Text("2".into()),
                CellValue::Text("Bob".into()),
                CellValue::Text("Seattle".into()),
            ],
        ],
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert_eq!(
        output,
        "id,name,city\r\n1,Alice,Portland\r\n2,Bob,Seattle\r\n"
    );
}

#[test]
fn null_values_produce_empty_fields() {
    let result = make_result(
        vec![make_column("a"), make_column("b"), make_column("c")],
        vec![
            vec![
                CellValue::Text("x".into()),
                CellValue::Null,
                CellValue::Text("z".into()),
            ],
            vec![CellValue::Null, CellValue::Null, CellValue::Null],
        ],
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert_eq!(output, "a,b,c\r\nx,,z\r\n,,\r\n");
}

#[test]
fn rfc4180_escaping_commas_quotes_newlines() {
    let result = make_result(
        vec![make_column("val")],
        vec![
            // Value containing a comma
            vec![CellValue::Text("hello, world".into())],
            // Value containing a double quote
            vec![CellValue::Text("say \"hi\"".into())],
            // Value containing a newline
            vec![CellValue::Text("line1\nline2".into())],
            // Value containing all three
            vec![CellValue::Text("a,b\"c\nd".into())],
        ],
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    // Per RFC 4180: fields with commas/quotes/newlines are enclosed in double quotes,
    // and embedded double quotes are escaped by doubling them.
    let expected = "val\r\n\
                    \"hello, world\"\r\n\
                    \"say \"\"hi\"\"\"\r\n\
                    \"line1\nline2\"\r\n\
                    \"a,b\"\"c\nd\"\r\n";
    assert_eq!(output, expected);
}

#[test]
fn crlf_line_terminators() {
    let result = make_result(
        vec![make_column("x")],
        vec![vec![CellValue::Text("1".into())]],
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    // Every record (including header) ends with \r\n
    let lines: Vec<&str> = output.split("\r\n").collect();
    // "x", "1", "" (trailing split)
    assert_eq!(lines, vec!["x", "1", ""]);
    // No bare \n outside of escaped fields
    let without_crlf = output.replace("\r\n", "");
    assert!(!without_crlf.contains('\n'), "found bare LF outside CRLF");
}

#[test]
fn empty_result_set_produces_header_only() {
    let result = make_result(
        vec![make_column("col1"), make_column("col2")],
        vec![], // no rows
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    assert_eq!(output, "col1,col2\r\n");
}

#[test]
fn column_names_with_special_characters_are_escaped() {
    let result = make_result(
        vec![
            make_column("normal"),
            make_column("has, comma"),
            make_column("has \"quote\""),
            make_column("has\nnewline"),
        ],
        vec![vec![
            CellValue::Text("a".into()),
            CellValue::Text("b".into()),
            CellValue::Text("c".into()),
            CellValue::Text("d".into()),
        ]],
    );

    let mut buf = Vec::new();
    write_csv_to_writer(&result, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();

    let first_line = output.split("\r\n").next().unwrap();
    assert_eq!(
        first_line,
        "normal,\"has, comma\",\"has \"\"quote\"\"\",\"has\nnewline\""
    );
}

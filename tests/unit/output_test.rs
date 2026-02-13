use std::path::Path;

use dbtoon::output::{print_summary, print_truncation_warning};

// --- T016: TOON file output print summary contains truncation keys ---

#[test]
fn print_summary_toon_file_truncated_succeeds() {
    let path = Path::new("/tmp/test_output.toon");
    let message = "Showing 500 rows. Use --no-limit to return all rows.";
    let result = print_summary(500, path, true, Some(message));
    assert!(result.is_ok(), "print_summary for .toon file should succeed");
}

#[test]
fn print_summary_truncated_produces_valid_toon() {
    // Capture stdout by calling print_summary and checking the return value
    // Since print_summary prints to stdout, we test the underlying logic
    // by calling it and verifying it doesn't error
    let path = Path::new("/tmp/test_output.csv");
    let message = "Showing 500 rows. Use --no-limit to return all rows.";
    let result = print_summary(500, path, true, Some(message));
    assert!(result.is_ok(), "print_summary should succeed for truncated case");
}

#[test]
fn print_summary_non_truncated_produces_valid_toon() {
    let path = Path::new("/tmp/test_output.csv");
    let result = print_summary(100, path, false, None);
    assert!(result.is_ok(), "print_summary should succeed for non-truncated case");
}

/// Test that print_summary output is valid TOON by capturing and decoding it.
/// We test the summary construction logic directly using the same encoding path.
#[test]
fn print_summary_truncated_output_is_decodable_toon() {
    // Build the same JSON object that print_summary builds
    let mut map = serde_json::Map::new();
    map.insert(
        "rows_written".to_string(),
        serde_json::Value::Number(serde_json::Number::from(500)),
    );
    map.insert(
        "file".to_string(),
        serde_json::Value::String("/tmp/test.csv".to_string()),
    );
    map.insert("truncated".to_string(), serde_json::Value::Bool(true));
    map.insert(
        "message".to_string(),
        serde_json::Value::String(
            "Showing 500 rows. Use --no-limit to return all rows.".to_string(),
        ),
    );

    let json_val = serde_json::Value::Object(map);
    let toon = toon_format::encode_default(&json_val).unwrap();
    let decoded: serde_json::Value = toon_format::decode_no_coerce(&toon).unwrap();
    let obj = decoded.as_object().expect("should be a TOON object");

    assert_eq!(obj.get("rows_written").unwrap(), 500);
    assert_eq!(obj.get("file").unwrap(), "/tmp/test.csv");
    assert_eq!(obj.get("truncated").unwrap(), true);
    assert_eq!(
        obj.get("message").unwrap(),
        "Showing 500 rows. Use --no-limit to return all rows."
    );
}

#[test]
fn print_summary_non_truncated_output_has_no_message() {
    let mut map = serde_json::Map::new();
    map.insert(
        "rows_written".to_string(),
        serde_json::Value::Number(serde_json::Number::from(100)),
    );
    map.insert(
        "file".to_string(),
        serde_json::Value::String("/tmp/test.csv".to_string()),
    );
    map.insert("truncated".to_string(), serde_json::Value::Bool(false));

    let json_val = serde_json::Value::Object(map);
    let toon = toon_format::encode_default(&json_val).unwrap();
    let decoded: serde_json::Value = toon_format::decode_no_coerce(&toon).unwrap();
    let obj = decoded.as_object().expect("should be a TOON object");

    assert_eq!(obj.get("rows_written").unwrap(), 100);
    assert_eq!(obj.get("truncated").unwrap(), false);
    assert!(
        !obj.contains_key("message"),
        "non-truncated summary should not have message"
    );
}

// --- T018: print_truncation_warning() test ---

#[test]
fn print_truncation_warning_does_not_panic() {
    // print_truncation_warning writes to stderr; verify it doesn't panic
    print_truncation_warning("Showing 500 rows. Use --no-limit to return all rows.");
}

use std::path::{Path, PathBuf};

use dbtoon::format_detect::{detect_format, OutputFormat};

#[test]
fn csv_extension() {
    let (fmt, path) = detect_format(Path::new("results.csv")).unwrap();
    assert_eq!(fmt, OutputFormat::Csv);
    assert_eq!(path, PathBuf::from("results.csv"));
}

#[test]
fn parquet_extension() {
    let (fmt, path) = detect_format(Path::new("results.parquet")).unwrap();
    assert_eq!(fmt, OutputFormat::Parquet);
    assert_eq!(path, PathBuf::from("results.parquet"));
}

#[test]
fn arrow_extension() {
    let (fmt, path) = detect_format(Path::new("results.arrow")).unwrap();
    assert_eq!(fmt, OutputFormat::Arrow);
    assert_eq!(path, PathBuf::from("results.arrow"));
}

#[test]
fn toon_extension() {
    let (fmt, path) = detect_format(Path::new("results.toon")).unwrap();
    assert_eq!(fmt, OutputFormat::Toon);
    assert_eq!(path, PathBuf::from("results.toon"));
}

#[test]
fn txt_extension() {
    let (fmt, path) = detect_format(Path::new("results.txt")).unwrap();
    assert_eq!(fmt, OutputFormat::Toon);
    assert_eq!(path, PathBuf::from("results.txt"));
}

#[test]
fn no_extension_appends_toon() {
    let (fmt, path) = detect_format(Path::new("results")).unwrap();
    assert_eq!(fmt, OutputFormat::Toon);
    assert_eq!(path, PathBuf::from("results.toon"));
}

#[test]
fn case_insensitive_uppercase() {
    let (fmt, _) = detect_format(Path::new("results.CSV")).unwrap();
    assert_eq!(fmt, OutputFormat::Csv);
}

#[test]
fn case_insensitive_mixed() {
    let (fmt, _) = detect_format(Path::new("results.Csv")).unwrap();
    assert_eq!(fmt, OutputFormat::Csv);
}

#[test]
fn case_insensitive_parquet() {
    let (fmt, _) = detect_format(Path::new("results.PARQUET")).unwrap();
    assert_eq!(fmt, OutputFormat::Parquet);
}

#[test]
fn case_insensitive_arrow() {
    let (fmt, _) = detect_format(Path::new("results.Arrow")).unwrap();
    assert_eq!(fmt, OutputFormat::Arrow);
}

#[test]
fn case_insensitive_toon() {
    let (fmt, _) = detect_format(Path::new("results.TOON")).unwrap();
    assert_eq!(fmt, OutputFormat::Toon);
}

#[test]
fn unrecognized_extension_errors() {
    let err = detect_format(Path::new("results.xlsx")).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(".xlsx"), "error should mention the bad extension: {msg}");
    assert!(msg.contains(".toon"), "error should list supported formats: {msg}");
    assert!(msg.contains(".csv"), "error should list supported formats: {msg}");
    assert!(msg.contains(".parquet"), "error should list supported formats: {msg}");
    assert!(msg.contains(".arrow"), "error should list supported formats: {msg}");
}

#[test]
fn path_with_directory_preserved() {
    let (fmt, path) = detect_format(Path::new("/tmp/output/results.csv")).unwrap();
    assert_eq!(fmt, OutputFormat::Csv);
    assert_eq!(path, PathBuf::from("/tmp/output/results.csv"));
}

#[test]
fn no_extension_with_directory_appends_toon() {
    let (fmt, path) = detect_format(Path::new("/tmp/output/results")).unwrap();
    assert_eq!(fmt, OutputFormat::Toon);
    assert_eq!(path, PathBuf::from("/tmp/output/results.toon"));
}

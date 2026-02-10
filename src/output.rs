use crate::error::DbtoonError;
use crate::format;
use std::path::Path;

/// Print TOON result to stdout.
pub fn print_result(toon_string: &str) {
    print!("{}", toon_string);
}

/// Print error to stderr in the contract format: error: <category>: <message>
pub fn print_error(err: &DbtoonError) {
    eprintln!("error: {}", err);
}

/// Print file output summary to stdout as TOON key-value pairs.
pub fn print_summary(rows: usize, path: &Path, truncated: bool) {
    let rows_str = rows.to_string();
    let path_str = path.display().to_string();
    let truncated_str = truncated.to_string();
    let summary = format::to_toon_kv(&[
        ("rows_written", &rows_str),
        ("file", &path_str),
        ("truncated", &truncated_str),
    ]);
    println!("{}", summary);
}

/// Print truncation metadata to stdout as TOON key-value pairs.
pub fn print_truncation_message(limit: usize) {
    let message = format!("Showing {} rows. Use --no-limit to return all rows.", limit);
    let toon = format::to_toon_kv(&[("truncated", "true"), ("message", &message)]);
    println!("{}", toon);
}

/// Write TOON string to a file.
pub fn write_file(toon_string: &str, path: &Path) -> Result<(), DbtoonError> {
    if let Some(parent) = path.parent()
        && !parent.exists() {
            return Err(DbtoonError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("parent directory does not exist: {}", parent.display()),
            )));
        }
    std::fs::write(path, toon_string)?;
    Ok(())
}

use crate::error::DbtoonError;
use std::path::Path;

/// Print TOON result to stdout.
pub fn print_result(toon_string: &str) {
    print!("{}", toon_string);
}

/// Print error to stderr in the contract format: error: <category>: <message>
pub fn print_error(err: &DbtoonError) {
    eprintln!("error: {}", err);
}

/// Print file output summary to stdout as a valid TOON object.
///
/// Summary includes: rows_written (number), file (string), truncated (bool),
/// and message (string, only when truncated).
pub fn print_summary(
    rows: usize,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "rows_written".to_string(),
        serde_json::Value::Number(serde_json::Number::from(rows)),
    );
    map.insert(
        "file".to_string(),
        serde_json::Value::String(path.display().to_string()),
    );
    map.insert(
        "truncated".to_string(),
        serde_json::Value::Bool(truncated),
    );
    if let Some(msg) = message {
        map.insert(
            "message".to_string(),
            serde_json::Value::String(msg.to_string()),
        );
    }

    let toon = toon_format::encode_default(&serde_json::Value::Object(map))
        .map_err(|e| DbtoonError::Format {
            message: e.to_string(),
        })?;
    print!("{}", toon);
    Ok(())
}

/// Print a truncation warning to stderr for interactive visibility.
/// Format: "warning: {message}"
pub fn print_truncation_warning(message: &str) {
    eprintln!("warning: {}", message);
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

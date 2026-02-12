use std::path::{Path, PathBuf};

use crate::error::DbtoonError;

/// Supported output file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Toon,
    Csv,
    Parquet,
    Arrow,
}

/// Detect the output format from a file path extension.
/// Returns the format and the (possibly normalized) path.
///
/// - `.toon`, `.txt` → Toon
/// - `.csv` → Csv
/// - `.parquet` → Parquet
/// - `.arrow` → Arrow
/// - No extension → appends `.toon`, returns Toon
/// - Unrecognized → error with supported format list
pub fn detect_format(path: &Path) -> Result<(OutputFormat, PathBuf), DbtoonError> {
    let ext = match path.extension() {
        Some(e) => e.to_ascii_lowercase(),
        None => {
            let mut p = path.to_path_buf();
            p.set_extension("toon");
            return Ok((OutputFormat::Toon, p));
        }
    };

    let format = match ext.to_str().unwrap_or("") {
        "toon" | "txt" => OutputFormat::Toon,
        "csv" => OutputFormat::Csv,
        "parquet" => OutputFormat::Parquet,
        "arrow" => OutputFormat::Arrow,
        other => {
            return Err(DbtoonError::Format {
                message: format!(
                    "unsupported output format \".{other}\" \
                     — supported: .toon, .txt, .csv, .parquet, .arrow"
                ),
            });
        }
    };

    Ok((format, path.to_path_buf()))
}

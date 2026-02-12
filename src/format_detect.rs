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
pub fn detect_format(_path: &Path) -> Result<(OutputFormat, PathBuf), DbtoonError> {
    todo!()
}

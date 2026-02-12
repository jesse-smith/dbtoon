use std::path::Path;

use crate::backend::QueryResult;
use crate::error::DbtoonError;

/// Write query results as RFC 4180 CSV.
pub fn write_csv(_result: &QueryResult, _path: &Path) -> Result<(), DbtoonError> {
    todo!()
}

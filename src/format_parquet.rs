use std::path::Path;

use crate::backend::QueryResult;
use crate::error::DbtoonError;

/// Write query results as a Parquet file with typed columns.
pub fn write_parquet(_result: &QueryResult, _path: &Path) -> Result<(), DbtoonError> {
    todo!()
}

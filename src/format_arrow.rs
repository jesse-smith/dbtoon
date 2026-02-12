use std::path::Path;

use crate::backend::QueryResult;
use crate::error::DbtoonError;

/// Write query results as an Arrow IPC file with typed columns.
pub fn write_arrow(_result: &QueryResult, _path: &Path) -> Result<(), DbtoonError> {
    todo!()
}

use std::fs::File;
use std::path::Path;

use parquet::arrow::ArrowWriter;

use crate::backend::QueryResult;
use crate::error::DbtoonError;
use crate::format_columnar;

/// Write query results as a Parquet file with typed columns.
pub fn write_parquet(result: &QueryResult, path: &Path) -> Result<(), DbtoonError> {
    let (schema, batch) = format_columnar::build_record_batch(result)?;

    let file = File::create(path)?;
    let mut writer =
        ArrowWriter::try_new(file, schema, None).map_err(|e| DbtoonError::Format {
            message: format!("failed to create Parquet writer: {e}"),
        })?;

    if batch.num_rows() > 0 {
        writer
            .write(&batch)
            .map_err(|e| DbtoonError::Format {
                message: format!("failed to write Parquet data: {e}"),
            })?;
    }

    writer.close().map_err(|e| DbtoonError::Format {
        message: format!("failed to finalize Parquet file: {e}"),
    })?;

    Ok(())
}

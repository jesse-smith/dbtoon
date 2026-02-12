use std::fs::File;
use std::path::Path;

use arrow::ipc::writer::FileWriter;

use crate::backend::QueryResult;
use crate::error::DbtoonError;
use crate::format_columnar;

/// Write query results as an Arrow IPC file with typed columns.
pub fn write_arrow(result: &QueryResult, path: &Path) -> Result<(), DbtoonError> {
    let (schema, batch) = format_columnar::build_record_batch(result)?;

    let file = File::create(path)?;
    let mut writer =
        FileWriter::try_new(file, &schema).map_err(|e| DbtoonError::Format {
            message: format!("failed to create Arrow IPC writer: {e}"),
        })?;

    if batch.num_rows() > 0 {
        writer
            .write(&batch)
            .map_err(|e| DbtoonError::Format {
                message: format!("failed to write Arrow IPC data: {e}"),
            })?;
    }

    writer.finish().map_err(|e| DbtoonError::Format {
        message: format!("failed to finalize Arrow IPC file: {e}"),
    })?;

    Ok(())
}

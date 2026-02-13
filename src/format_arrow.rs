use std::fs::File;
use std::path::Path;

use arrow::ipc::writer::FileWriter;

use crate::backend::QueryResult;
use crate::error::DbtoonError;
use crate::format_columnar;

/// Write query results as an Arrow IPC file with typed columns and optional truncation metadata.
///
/// When truncated, schema metadata includes `dbtoon:truncated` and `dbtoon:message` keys.
pub fn write_arrow(
    result: &QueryResult,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError> {
    let (schema, batch) = format_columnar::build_record_batch(result)?;
    let schema = format_columnar::with_truncation_metadata(schema, truncated, message);

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

use std::fs::File;
use std::path::Path;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

use crate::backend::QueryResult;
use crate::error::DbtoonError;
use crate::format_columnar;

/// Write query results as a Parquet file with typed columns and optional truncation metadata.
///
/// When truncated, file metadata includes `dbtoon:truncated` and `dbtoon:message` keys.
pub fn write_parquet(
    result: &QueryResult,
    path: &Path,
    truncated: bool,
    message: Option<&str>,
) -> Result<(), DbtoonError> {
    let (schema, batch) = format_columnar::build_record_batch(result)?;

    let props = if truncated {
        let mut kv = vec![
            parquet::file::metadata::KeyValue {
                key: "dbtoon:truncated".to_string(),
                value: Some("true".to_string()),
            },
        ];
        if let Some(msg) = message {
            kv.push(parquet::file::metadata::KeyValue {
                key: "dbtoon:message".to_string(),
                value: Some(msg.to_string()),
            });
        }
        Some(
            WriterProperties::builder()
                .set_key_value_metadata(Some(kv))
                .build(),
        )
    } else {
        None
    };

    let file = File::create(path)?;
    let mut writer =
        ArrowWriter::try_new(file, schema, props).map_err(|e| DbtoonError::Format {
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

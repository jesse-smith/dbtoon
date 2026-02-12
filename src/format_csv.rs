use std::fs::File;
use std::io::Write;
use std::path::Path;

use csv::WriterBuilder;

use crate::backend::{CellValue, QueryResult};
use crate::error::DbtoonError;

/// Write query results as RFC 4180 CSV to a generic writer.
pub fn write_csv_to_writer<W: Write>(result: &QueryResult, writer: W) -> Result<(), DbtoonError> {
    let mut wtr = WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(writer);

    // Header row from column names
    let headers: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers)
        .map_err(|e| DbtoonError::Format {
            message: format!("failed to write CSV header: {e}"),
        })?;

    // Data rows
    for row in &result.rows {
        let fields: Vec<&str> = row
            .iter()
            .map(|cell| match cell {
                CellValue::Text(s) => s.as_str(),
                CellValue::Null => "",
            })
            .collect();
        wtr.write_record(&fields)
            .map_err(|e| DbtoonError::Format {
                message: format!("failed to write CSV row: {e}"),
            })?;
    }

    wtr.flush().map_err(|e| DbtoonError::Format {
        message: format!("failed to flush CSV output: {e}"),
    })?;

    Ok(())
}

/// Write query results as RFC 4180 CSV to a file.
pub fn write_csv(result: &QueryResult, path: &Path) -> Result<(), DbtoonError> {
    let file = File::create(path)?;
    write_csv_to_writer(result, file)
}

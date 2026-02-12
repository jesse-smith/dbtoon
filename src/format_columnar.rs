use std::sync::Arc;

use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;

use crate::backend::QueryResult;
use crate::error::DbtoonError;

/// Map a SQL type string to an Arrow DataType.
/// Unknown types map to Utf8 (string fallback).
pub fn sql_type_to_arrow(_type_name: &str) -> DataType {
    todo!()
}

/// Build an Arrow RecordBatch from a QueryResult.
/// Each column is converted to a typed Arrow array based on its type_name.
/// If value parsing fails for a column, that column falls back to Utf8.
pub fn build_record_batch(
    _result: &QueryResult,
) -> Result<(Arc<Schema>, RecordBatch), DbtoonError> {
    todo!()
}

pub mod databricks;
pub mod sqlserver;

use crate::error::DbtoonError;

/// Metadata for a single result column.
#[derive(Debug, Clone)]
pub struct ColumnMeta {
    pub name: String,
    pub type_name: String,
}

/// A single cell value from a query result.
#[derive(Debug, Clone)]
pub enum CellValue {
    Text(String),
    Null,
}

/// The output of executing a query, before TOON serialization.
#[derive(Debug)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<CellValue>>,
    pub total_rows: Option<usize>,
    pub truncated: bool,
}

/// Trait for database backends.
pub trait Backend {
    fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> impl std::future::Future<Output = Result<QueryResult, DbtoonError>> + Send;
}

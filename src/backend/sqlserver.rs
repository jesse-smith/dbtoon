use crate::backend::{Backend, CellValue, ColumnMeta, QueryResult};
use crate::config::SqlServerAuth;
use crate::error::DbtoonError;
use futures_util::TryStreamExt;
use tiberius::{AuthMethod, Client, ColumnData, ColumnType, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub struct SqlServerBackend {
    server: String,
    database: Option<String>,
    auth: SqlServerAuth,
    trust_server_certificate: bool,
}

impl SqlServerBackend {
    pub fn new(
        server: String,
        database: Option<String>,
        auth: SqlServerAuth,
        trust_server_certificate: bool,
    ) -> Self {
        Self {
            server,
            database,
            auth,
            trust_server_certificate,
        }
    }

    fn build_tiberius_config(&self) -> Result<Config, DbtoonError> {
        todo!()
    }
}

/// Parse user-provided server string into (host, port, instance_name).
/// Formats: "host", "host,port", "host\instance", "host\instance,port", "tcp:host,port"
/// Returns Err(DbtoonError::Config) for invalid port values.
pub fn parse_server_address(
    server: &str,
) -> Result<(String, Option<u16>, Option<String>), DbtoonError> {
    todo!()
}

/// Best-effort mapping from tiberius ColumnType to SQL type string.
/// Used when DMV-based describe fails. Omits precision/scale/length.
pub fn normalize_tiberius_type(col_type: ColumnType) -> String {
    todo!()
}

/// Convert a tiberius ColumnData value to a CellValue string.
pub fn column_data_to_string(data: &ColumnData<'_>) -> CellValue {
    todo!()
}

/// Query sys.dm_exec_describe_first_result_set to get column type names.
/// Falls back to ColumnType-based mapping on failure.
async fn describe_result_columns(
    client: &mut Client<Compat<TcpStream>>,
    sql: &str,
) -> Result<Vec<ColumnMeta>, DbtoonError> {
    todo!()
}

impl Backend for SqlServerBackend {
    async fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> Result<QueryResult, DbtoonError> {
        todo!()
    }
}

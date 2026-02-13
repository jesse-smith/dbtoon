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
    let server = server.strip_prefix("tcp:").unwrap_or(server);

    if let Some((host, instance_part)) = server.split_once('\\') {
        if let Some((instance, port_str)) = instance_part.split_once(',') {
            let port: u16 = port_str.parse().map_err(|_| DbtoonError::Config {
                message: format!("invalid port value: '{}'", port_str),
            })?;
            Ok((host.to_string(), Some(port), Some(instance.to_string())))
        } else {
            Ok((host.to_string(), None, Some(instance_part.to_string())))
        }
    } else if let Some((host, port_str)) = server.split_once(',') {
        let port: u16 = port_str.parse().map_err(|_| DbtoonError::Config {
            message: format!("invalid port value: '{}'", port_str),
        })?;
        Ok((host.to_string(), Some(port), None))
    } else {
        Ok((server.to_string(), None, None))
    }
}

/// Best-effort mapping from tiberius ColumnType to SQL type string.
/// Used when DMV-based describe fails. Omits precision/scale/length.
pub fn normalize_tiberius_type(col_type: ColumnType) -> String {
    match col_type {
        ColumnType::Null => "UNKNOWN",
        ColumnType::Bit | ColumnType::Bitn => "BIT",
        ColumnType::Int1 => "TINYINT",
        ColumnType::Int2 => "SMALLINT",
        ColumnType::Int4 | ColumnType::Intn => "INT",
        ColumnType::Int8 => "BIGINT",
        ColumnType::Float4 => "REAL",
        ColumnType::Float8 | ColumnType::Floatn => "FLOAT",
        ColumnType::Money => "MONEY",
        ColumnType::Money4 => "SMALLMONEY",
        ColumnType::Datetime | ColumnType::Datetimen => "DATETIME",
        ColumnType::Datetime4 => "SMALLDATETIME",
        ColumnType::Datetime2 => "DATETIME2",
        ColumnType::Daten => "DATE",
        ColumnType::Timen => "TIME",
        ColumnType::DatetimeOffsetn => "DATETIMEOFFSET",
        ColumnType::Decimaln => "DECIMAL",
        ColumnType::Numericn => "NUMERIC",
        ColumnType::BigVarChar => "VARCHAR",
        ColumnType::BigChar => "CHAR",
        ColumnType::NVarchar => "NVARCHAR",
        ColumnType::NChar => "NCHAR",
        ColumnType::BigVarBin => "VARBINARY",
        ColumnType::BigBinary => "BINARY",
        ColumnType::Guid => "UNIQUEIDENTIFIER",
        ColumnType::Xml => "XML",
        ColumnType::Text => "TEXT",
        ColumnType::NText => "NTEXT",
        ColumnType::Image => "IMAGE",
        ColumnType::SSVariant => "SQL_VARIANT",
        ColumnType::Udt => "UNKNOWN",
    }
    .to_string()
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

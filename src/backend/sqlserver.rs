use crate::backend::{Backend, CellValue, ColumnMeta, QueryResult};
use crate::config::SqlServerAuth;
use crate::error::DbtoonError;
use futures_util::TryStreamExt;
use secrecy::ExposeSecret;
use std::time::Duration;
use tiberius::{AuthMethod, Client, ColumnData, ColumnType, Config, EncryptionLevel, SqlBrowser};
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

    async fn execute_inner(
        &self,
        sql: &str,
        limit: Option<usize>,
    ) -> Result<QueryResult, DbtoonError> {
        let config = self.build_tiberius_config()?;

        // connect_named handles both named instances (SQL Browser resolution)
        // and direct connections (no instance name).
        let tcp = TcpStream::connect_named(&config).await.map_err(|e| {
            DbtoonError::Connection {
                message: format!("{}", e),
            }
        })?;
        tcp.set_nodelay(true)?;

        let mut client =
            Client::connect(config, tcp.compat_write())
                .await
                .map_err(|e| {
                    let msg = e.to_string();
                    let lower = msg.to_lowercase();
                    if lower.contains("login")
                        || lower.contains("authentication")
                        || lower.contains("gssapi")
                    {
                        DbtoonError::Auth { message: msg }
                    } else {
                        DbtoonError::Connection { message: msg }
                    }
                })?;

        // Try DMV describe for precise column type names; Ok(cols) or Err (ignored).
        let dmv_columns = describe_result_columns(&mut client, sql).await.ok();

        // Execute user query.
        let mut stream = client.query(sql, &[]).await.map_err(|e| DbtoonError::Query {
            message: format!("{}", e),
        })?;

        // Column metadata: prefer DMV, fall back to stream metadata + normalize.
        let columns = if let Some(dmv_cols) = dmv_columns {
            dmv_cols
        } else {
            eprintln!(
                "warning: DMV describe unavailable, falling back to tiberius column metadata"
            );
            let stream_cols =
                stream
                    .columns()
                    .await
                    .map_err(|e| DbtoonError::Query {
                        message: format!("{}", e),
                    })?;
            match stream_cols {
                Some(cols) => cols
                    .iter()
                    .map(|c| ColumnMeta {
                        name: c.name().to_string(),
                        type_name: normalize_tiberius_type(c.column_type()),
                    })
                    .collect(),
                None => Vec::new(),
            }
        };

        // Stream rows, enforcing the row limit.
        let mut row_stream = stream.into_row_stream();
        let mut rows = Vec::new();
        let mut truncated = false;

        while let Some(row) = row_stream
            .try_next()
            .await
            .map_err(|e| DbtoonError::Query {
                message: format!("{}", e),
            })?
        {
            if let Some(max) = limit
                && rows.len() >= max
            {
                truncated = true;
                break;
            }
            let row_data: Vec<CellValue> =
                row.cells().map(|(_, data)| column_data_to_string(data)).collect();
            rows.push(row_data);
        }

        let total_rows = if truncated { None } else { Some(rows.len()) };

        Ok(QueryResult {
            columns,
            rows,
            total_rows,
            truncated,
        })
    }

    fn build_tiberius_config(&self) -> Result<Config, DbtoonError> {
        let (host, port, instance) = parse_server_address(&self.server)?;

        // Resolve hostname to FQDN for Kerberos SPN construction.
        // Tiberius builds the SPN as MSSQLSvc/{host}:{port}, and Active Directory
        // typically registers SPNs with the FQDN, not the short hostname.
        let host = resolve_fqdn(&host);

        let mut config = Config::new();
        config.host(&host);
        config.port(port.unwrap_or(1433));
        if let Some(ref inst) = instance {
            config.instance_name(inst);
        }
        if let Some(ref db) = self.database {
            config.database(db);
        }

        match &self.auth {
            SqlServerAuth::WindowsIntegrated => {
                config.authentication(AuthMethod::Integrated);
            }
            SqlServerAuth::SqlLogin { username, password } => {
                config.authentication(AuthMethod::sql_server(
                    username,
                    password.expose_secret(),
                ));
            }
        }

        if self.trust_server_certificate {
            config.trust_cert();
        }

        config.encryption(EncryptionLevel::Required);

        Ok(config)
    }
}

/// Resolve a hostname to its FQDN via DNS lookup.
/// Falls back to the original hostname if resolution fails.
fn resolve_fqdn(host: &str) -> String {
    use std::net::ToSocketAddrs;
    // If it already looks like a FQDN (contains a dot), use as-is.
    if host.contains('.') {
        return host.to_string();
    }
    // Resolve and extract the canonical name from the first result.
    match (host, 0u16).to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                // Reverse-lookup the IP to get the FQDN.
                match dns_lookup::lookup_addr(&addr.ip()) {
                    Ok(fqdn) if fqdn.contains('.') => fqdn,
                    _ => host.to_string(),
                }
            } else {
                host.to_string()
            }
        }
        Err(_) => host.to_string(),
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
    match data {
        // Integer types
        ColumnData::U8(Some(v)) => CellValue::Text(v.to_string()),
        ColumnData::I16(Some(v)) => CellValue::Text(v.to_string()),
        ColumnData::I32(Some(v)) => CellValue::Text(v.to_string()),
        ColumnData::I64(Some(v)) => CellValue::Text(v.to_string()),
        // Float types
        ColumnData::F32(Some(v)) => CellValue::Text(v.to_string()),
        ColumnData::F64(Some(v)) => CellValue::Text(v.to_string()),
        // Bit type — 0/1 not true/false
        ColumnData::Bit(Some(v)) => CellValue::Text(if *v { "1" } else { "0" }.to_string()),
        // String — as-is
        ColumnData::String(Some(s)) => CellValue::Text(s.to_string()),
        // Guid — hyphenated lowercase UUID
        ColumnData::Guid(Some(g)) => CellValue::Text(g.to_string()),
        // Binary — 0x hex prefix, uppercase hex digits
        ColumnData::Binary(Some(b)) => {
            let hex: String = b.iter().map(|byte| format!("{:02X}", byte)).collect();
            CellValue::Text(format!("0x{}", hex))
        }
        // Numeric — preserve trailing zeros via scale
        ColumnData::Numeric(Some(n)) => {
            if n.scale() == 0 {
                CellValue::Text(n.value().to_string())
            } else {
                CellValue::Text(n.to_string())
            }
        }
        // DateTime — YYYY-MM-DD HH:MM:SS.mmm (epoch: 1900-01-01, 1/300 sec)
        ColumnData::DateTime(Some(dt)) => {
            let (y, m, d) = days_to_ymd(dt.days() as i64 - EPOCH_1900_UNIX_DAYS);
            let frags = dt.seconds_fragments() as u64;
            let total_ms = (frags * 1000 + 150) / 300;
            let hours = total_ms / 3_600_000;
            let minutes = (total_ms % 3_600_000) / 60_000;
            let seconds = (total_ms % 60_000) / 1_000;
            let millis = total_ms % 1_000;
            CellValue::Text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
                y, m, d, hours, minutes, seconds, millis
            ))
        }
        // SmallDateTime — YYYY-MM-DD HH:MM:SS (epoch: 1900-01-01, minutes)
        ColumnData::SmallDateTime(Some(dt)) => {
            let (y, m, d) = days_to_ymd(dt.days() as i64 - EPOCH_1900_UNIX_DAYS);
            let total_minutes = dt.seconds_fragments() as u64;
            let hours = total_minutes / 60;
            let minutes = total_minutes % 60;
            CellValue::Text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                y, m, d, hours, minutes, 0
            ))
        }
        // Date — YYYY-MM-DD (epoch: 0001-01-01)
        ColumnData::Date(Some(date)) => {
            let (y, m, d) = days_to_ymd(date.days() as i64 - EPOCH_YEAR1_UNIX_DAYS);
            CellValue::Text(format!("{:04}-{:02}-{:02}", y, m, d))
        }
        // Time — HH:MM:SS.nnnnnnn (scale-dependent fractional seconds)
        ColumnData::Time(Some(t)) => {
            let increments = t.increments();
            let scale = t.scale() as u32;
            let divisor = 10u64.pow(scale);
            let total_seconds = increments / divisor;
            let frac = increments % divisor;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            CellValue::Text(format!(
                "{:02}:{:02}:{:02}.{:0width$}",
                hours,
                minutes,
                seconds,
                frac,
                width = scale as usize
            ))
        }
        // DateTime2 — YYYY-MM-DD HH:MM:SS.nnnnnnn
        ColumnData::DateTime2(Some(dt2)) => {
            let date = dt2.date();
            let time = dt2.time();
            let (y, m, d) = days_to_ymd(date.days() as i64 - EPOCH_YEAR1_UNIX_DAYS);
            let increments = time.increments();
            let scale = time.scale() as u32;
            let divisor = 10u64.pow(scale);
            let total_seconds = increments / divisor;
            let frac = increments % divisor;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            CellValue::Text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:0width$}",
                y,
                m,
                d,
                hours,
                minutes,
                seconds,
                frac,
                width = scale as usize
            ))
        }
        // DateTimeOffset — YYYY-MM-DD HH:MM:SS.nnnnnnn +HH:MM
        ColumnData::DateTimeOffset(Some(dto)) => {
            let dt2 = dto.datetime2();
            let date = dt2.date();
            let time = dt2.time();
            let (y, m, d) = days_to_ymd(date.days() as i64 - EPOCH_YEAR1_UNIX_DAYS);
            let increments = time.increments();
            let scale = time.scale() as u32;
            let divisor = 10u64.pow(scale);
            let total_seconds = increments / divisor;
            let frac = increments % divisor;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            let offset = dto.offset();
            let sign = if offset < 0 { '-' } else { '+' };
            let off_hours = offset.unsigned_abs() / 60;
            let off_minutes = offset.unsigned_abs() % 60;
            CellValue::Text(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:0width$} {}{:02}:{:02}",
                y,
                m,
                d,
                hours,
                minutes,
                seconds,
                frac,
                sign,
                off_hours,
                off_minutes,
                width = scale as usize
            ))
        }
        // Xml — as-is
        ColumnData::Xml(Some(x)) => CellValue::Text(x.to_string()),
        // All None variants → Null
        ColumnData::U8(None)
        | ColumnData::I16(None)
        | ColumnData::I32(None)
        | ColumnData::I64(None)
        | ColumnData::F32(None)
        | ColumnData::F64(None)
        | ColumnData::Bit(None)
        | ColumnData::String(None)
        | ColumnData::Guid(None)
        | ColumnData::Binary(None)
        | ColumnData::Numeric(None)
        | ColumnData::DateTime(None)
        | ColumnData::SmallDateTime(None)
        | ColumnData::Date(None)
        | ColumnData::Time(None)
        | ColumnData::DateTime2(None)
        | ColumnData::DateTimeOffset(None)
        | ColumnData::Xml(None) => CellValue::Null,
    }
}

/// Days from 1900-01-01 to Unix epoch (1970-01-01).
const EPOCH_1900_UNIX_DAYS: i64 = 25567;
/// Days from 0001-01-01 to Unix epoch (1970-01-01).
const EPOCH_YEAR1_UNIX_DAYS: i64 = 719162;

/// Convert Unix-epoch days to (year, month, day) using Hinnant's civil algorithm.
fn days_to_ymd(unix_days: i64) -> (i32, u32, u32) {
    let z = unix_days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// Query sys.dm_exec_describe_first_result_set to get precise column type names.
/// Returns Err on any failure (permissions, unsupported query); caller should
/// fall back to QueryStream column metadata + normalize_tiberius_type.
async fn describe_result_columns(
    client: &mut Client<Compat<TcpStream>>,
    sql: &str,
) -> Result<Vec<ColumnMeta>, DbtoonError> {
    let stream = client
        .query(
            "SELECT name, system_type_name \
             FROM sys.dm_exec_describe_first_result_set(@P1, NULL, 0) \
             ORDER BY column_ordinal",
            &[&sql],
        )
        .await
        .map_err(|e| DbtoonError::Query {
            message: format!("DMV describe failed: {}", e),
        })?;

    let rows = stream
        .into_first_result()
        .await
        .map_err(|e| DbtoonError::Query {
            message: format!("DMV describe failed: {}", e),
        })?;

    let mut columns = Vec::new();
    for row in &rows {
        let name: Option<&str> = row.get(0);
        let type_name: Option<&str> = row.get(1);
        columns.push(ColumnMeta {
            name: name.unwrap_or("?").to_string(),
            type_name: type_name
                .map(|s| s.to_uppercase())
                .unwrap_or_else(|| "UNKNOWN".to_string()),
        });
    }

    Ok(columns)
}

impl Backend for SqlServerBackend {
    async fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> Result<QueryResult, DbtoonError> {
        match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.execute_inner(sql, limit),
        )
        .await
        {
            Ok(result) => result,
            Err(_elapsed) => Err(DbtoonError::Timeout {
                seconds: timeout_secs,
            }),
        }
    }
}

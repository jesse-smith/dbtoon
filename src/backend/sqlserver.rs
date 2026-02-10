use crate::backend::{Backend, CellValue, ColumnMeta, QueryResult};
use crate::config::SqlServerAuth;
use crate::error::DbtoonError;
use odbc_api::buffers::{BufferDesc, ColumnarAnyBuffer};
use odbc_api::{ColumnDescription, ConnectionOptions, Cursor, Environment, Nullability, ResultSetMetadata};

pub struct SqlServerBackend {
    server: String,
    database: Option<String>,
    auth: SqlServerAuth,
}

impl SqlServerBackend {
    pub fn new(server: String, database: Option<String>, auth: SqlServerAuth) -> Self {
        Self {
            server,
            database,
            auth,
        }
    }

    fn connection_string(&self) -> String {
        let mut parts = vec![
            "Driver={ODBC Driver 18 for SQL Server}".to_string(),
            format!("Server={}", self.server),
        ];

        if let Some(ref db) = self.database {
            parts.push(format!("Database={}", db));
        }

        match &self.auth {
            SqlServerAuth::WindowsIntegrated => {
                parts.push("Trusted_Connection=yes".to_string());
            }
            SqlServerAuth::SqlLogin { username, password } => {
                use secrecy::ExposeSecret;
                parts.push(format!("UID={}", username));
                parts.push(format!(
                    "PWD={}",
                    odbc_api::escape_attribute_value(password.expose_secret())
                ));
            }
        }

        parts.join(";") + ";"
    }
}

impl Backend for SqlServerBackend {
    async fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> Result<QueryResult, DbtoonError> {
        let conn_str = self.connection_string();

        // odbc-api Environment and connection are not Send, so we run in spawn_blocking
        let sql = sql.to_string();
        let result = tokio::task::spawn_blocking(move || -> Result<QueryResult, DbtoonError> {
            let env = Environment::new().map_err(|e| DbtoonError::Connection {
                message: format!("ODBC environment error: {}", e),
            })?;

            let conn = env
                .connect_with_connection_string(
                    &conn_str,
                    ConnectionOptions {
                        login_timeout_sec: Some(30),
                        ..Default::default()
                    },
                )
                .map_err(|e| DbtoonError::Connection {
                    message: format!("connection failed: {}", e),
                })?;

            let cursor = conn
                .execute(&sql, (), Some(timeout_secs as usize))
                .map_err(|e| DbtoonError::Query {
                    message: format!("query execution failed: {}", e),
                })?;

            let Some(mut cursor) = cursor else {
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    total_rows: None,
                    truncated: false,
                });
            };

            // Extract column metadata
            let num_cols = cursor.num_result_cols().map_err(|e| DbtoonError::Query {
                message: format!("failed to get column count: {}", e),
            })? as usize;

            let mut columns = Vec::with_capacity(num_cols);
            let mut buffer_descs = Vec::with_capacity(num_cols);

            for i in 1..=num_cols as u16 {
                let mut col_desc = ColumnDescription::default();
                cursor
                    .describe_col(i, &mut col_desc)
                    .map_err(|e| DbtoonError::Query {
                        message: format!("failed to describe column {}: {}", i, e),
                    })?;

                let name = col_desc.name_to_string().map_err(|e| DbtoonError::Query {
                    message: format!("failed to decode column name {}: {}", i, e),
                })?;

                columns.push(ColumnMeta {
                    name,
                    type_name: format!("{:?}", col_desc.data_type),
                });

                let nullable = col_desc.nullability != Nullability::NoNulls;
                let desc = BufferDesc::from_data_type(col_desc.data_type, nullable)
                    .unwrap_or(BufferDesc::Text { max_str_len: 255 });
                buffer_descs.push(desc);
            }

            let batch_size = 5000;
            let buffer = ColumnarAnyBuffer::try_from_descs(batch_size, buffer_descs)
                .map_err(|e| DbtoonError::Query {
                    message: format!("failed to create buffer: {}", e),
                })?;

            let mut row_set_cursor =
                cursor.bind_buffer(buffer).map_err(|e| DbtoonError::Query {
                    message: format!("failed to bind buffer: {}", e),
                })?;

            let mut rows: Vec<Vec<CellValue>> = Vec::new();
            let mut truncated = false;

            while let Some(batch) = row_set_cursor.fetch().map_err(|e| DbtoonError::Query {
                message: format!("fetch error: {}", e),
            })? {
                let num_rows_in_batch = batch.num_rows();
                for row_idx in 0..num_rows_in_batch {
                    if let Some(lim) = limit
                        && rows.len() >= lim {
                            truncated = true;
                            break;
                        }

                    let mut row = Vec::with_capacity(num_cols);
                    for col_idx in 0..num_cols {
                        let col = batch.column(col_idx);
                        let text_col = col.as_text_view();
                        match text_col {
                            Some(text_view) => match text_view.get(row_idx) {
                                Some(bytes) => {
                                    let s = String::from_utf8_lossy(bytes).to_string();
                                    row.push(CellValue::Text(s));
                                }
                                None => row.push(CellValue::Null),
                            },
                            None => {
                                row.push(CellValue::Null);
                            }
                        }
                    }
                    rows.push(row);
                }

                if truncated {
                    break;
                }
            }

            Ok(QueryResult {
                columns,
                rows,
                total_rows: None,
                truncated,
            })
        })
        .await
        .map_err(|e| DbtoonError::Query {
            message: format!("task join error: {}", e),
        })??;

        Ok(result)
    }
}

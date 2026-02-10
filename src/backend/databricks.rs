use crate::backend::{Backend, CellValue, ColumnMeta, QueryResult};
use crate::error::DbtoonError;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub struct DatabricksBackend {
    host: String,
    token: SecretString,
    warehouse_id: String,
    catalog: Option<String>,
    schema: Option<String>,
    client: Client,
}

/// Warehouse info for list-warehouses subcommand.
#[derive(Debug)]
pub struct WarehouseInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    pub cluster_size: String,
    pub warehouse_type: Option<String>,
}

#[derive(Serialize)]
struct StatementRequest {
    warehouse_id: String,
    statement: String,
    wait_timeout: String,
    on_wait_timeout: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    row_limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
}

#[derive(Deserialize)]
struct StatementResponse {
    statement_id: Option<String>,
    status: StatementStatus,
    manifest: Option<Manifest>,
    result: Option<ResultData>,
}

#[derive(Deserialize)]
struct StatementStatus {
    state: String,
    error: Option<StatementError>,
}

#[derive(Deserialize)]
struct StatementError {
    error_code: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    schema: Option<SchemaInfo>,
    truncated: Option<bool>,
}

#[derive(Deserialize)]
struct SchemaInfo {
    columns: Option<Vec<ColumnInfo>>,
}

#[derive(Deserialize)]
struct ColumnInfo {
    name: String,
    type_name: Option<String>,
}

#[derive(Deserialize)]
struct ResultData {
    data_array: Option<Vec<Vec<Option<String>>>>,
}

#[derive(Deserialize)]
struct WarehouseListResponse {
    warehouses: Option<Vec<WarehouseEntry>>,
}

#[derive(Deserialize)]
struct WarehouseEntry {
    id: Option<String>,
    name: Option<String>,
    state: Option<String>,
    cluster_size: Option<String>,
    warehouse_type: Option<String>,
}

impl DatabricksBackend {
    pub fn new(
        host: String,
        token: SecretString,
        warehouse_id: String,
        catalog: Option<String>,
        schema: Option<String>,
    ) -> Self {
        let client = Client::new();
        Self {
            host,
            token,
            warehouse_id,
            catalog,
            schema,
            client,
        }
    }

    fn base_url(&self) -> String {
        format!("https://{}/api/2.0/sql", self.host)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token.expose_secret())
    }

    async fn poll_statement(&self, statement_id: &str, timeout_secs: u64) -> Result<StatementResponse, DbtoonError> {
        let start = Instant::now();
        let url = format!("{}/statements/{}", self.base_url(), statement_id);

        loop {
            if start.elapsed().as_secs() >= timeout_secs {
                // Cancel the statement
                let cancel_url = format!("{}/statements/{}/cancel", self.base_url(), statement_id);
                let _ = self
                    .client
                    .post(&cancel_url)
                    .header("Authorization", self.auth_header())
                    .send()
                    .await;
                return Err(DbtoonError::Timeout {
                    seconds: timeout_secs,
                });
            }

            tokio::time::sleep(Duration::from_secs(2)).await;

            let resp = self
                .client
                .get(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await
                .map_err(|e| DbtoonError::Connection {
                    message: format!("failed to poll statement: {}", e),
                })?;

            let status_code = resp.status();
            if !status_code.is_success() {
                return Err(Self::map_http_error(status_code.as_u16()));
            }

            let response: StatementResponse =
                resp.json().await.map_err(|e| DbtoonError::Query {
                    message: format!("failed to parse poll response: {}", e),
                })?;

            match response.status.state.as_str() {
                "SUCCEEDED" => return Ok(response),
                "FAILED" => {
                    let msg = response
                        .status
                        .error
                        .and_then(|e| e.message)
                        .unwrap_or_else(|| "unknown error".to_string());
                    return Err(DbtoonError::Query { message: msg });
                }
                "CANCELED" => {
                    return Err(DbtoonError::Query {
                        message: "statement was canceled".to_string(),
                    });
                }
                _ => continue, // PENDING or RUNNING
            }
        }
    }

    fn map_http_error(status: u16) -> DbtoonError {
        match status {
            401 => DbtoonError::Auth {
                message: "invalid or expired token".to_string(),
            },
            403 => DbtoonError::Auth {
                message: "insufficient warehouse permissions".to_string(),
            },
            404 => DbtoonError::Config {
                message: "warehouse not found".to_string(),
            },
            _ => DbtoonError::Connection {
                message: format!("HTTP error: {}", status),
            },
        }
    }

    fn parse_response(response: StatementResponse) -> Result<QueryResult, DbtoonError> {
        let manifest = response.manifest.unwrap_or(Manifest {
            schema: None,
            truncated: None,
        });

        let columns = manifest
            .schema
            .and_then(|s| s.columns)
            .unwrap_or_default()
            .into_iter()
            .map(|c| ColumnMeta {
                name: c.name,
                type_name: c.type_name.unwrap_or_else(|| "STRING".to_string()),
            })
            .collect::<Vec<_>>();

        let rows = response
            .result
            .and_then(|r| r.data_array)
            .unwrap_or_default()
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| match cell {
                        Some(s) => CellValue::Text(s),
                        None => CellValue::Null,
                    })
                    .collect()
            })
            .collect();

        let truncated = manifest.truncated.unwrap_or(false);

        Ok(QueryResult {
            columns,
            rows,
            total_rows: None,
            truncated,
        })
    }
}

impl Backend for DatabricksBackend {
    async fn execute(
        &self,
        sql: &str,
        limit: Option<usize>,
        timeout_secs: u64,
    ) -> Result<QueryResult, DbtoonError> {
        let url = format!("{}/statements/", self.base_url());

        let request = StatementRequest {
            warehouse_id: self.warehouse_id.clone(),
            statement: sql.to_string(),
            wait_timeout: "50s".to_string(),
            on_wait_timeout: "CONTINUE".to_string(),
            row_limit: limit,
            catalog: self.catalog.clone(),
            schema: self.schema.clone(),
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&request)
            .send()
            .await
            .map_err(|e| DbtoonError::Connection {
                message: format!("failed to execute statement: {}", e),
            })?;

        let status_code = resp.status();
        if !status_code.is_success() {
            return Err(Self::map_http_error(status_code.as_u16()));
        }

        let response: StatementResponse =
            resp.json().await.map_err(|e| DbtoonError::Query {
                message: format!("failed to parse response: {}", e),
            })?;

        match response.status.state.as_str() {
            "SUCCEEDED" => Self::parse_response(response),
            "FAILED" => {
                let msg = response
                    .status
                    .error
                    .and_then(|e| e.message)
                    .unwrap_or_else(|| "unknown error".to_string());
                Err(DbtoonError::Query { message: msg })
            }
            "PENDING" | "RUNNING" => {
                let statement_id = response
                    .statement_id
                    .ok_or_else(|| DbtoonError::Query {
                        message: "no statement_id in pending response".to_string(),
                    })?;
                let polled = self.poll_statement(&statement_id, timeout_secs).await?;
                Self::parse_response(polled)
            }
            other => Err(DbtoonError::Query {
                message: format!("unexpected statement state: {}", other),
            }),
        }
    }
}

/// List available Databricks SQL warehouses.
pub async fn list_warehouses(
    host: &str,
    token: &SecretString,
) -> Result<Vec<WarehouseInfo>, DbtoonError> {
    let client = Client::new();
    let url = format!("https://{}/api/2.0/sql/warehouses/", host);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token.expose_secret()))
        .send()
        .await
        .map_err(|e| DbtoonError::Connection {
            message: format!("failed to list warehouses: {}", e),
        })?;

    let status_code = resp.status();
    if !status_code.is_success() {
        return Err(DatabricksBackend::map_http_error(status_code.as_u16()));
    }

    let response: WarehouseListResponse =
        resp.json().await.map_err(|e| DbtoonError::Query {
            message: format!("failed to parse warehouse list: {}", e),
        })?;

    Ok(response
        .warehouses
        .unwrap_or_default()
        .into_iter()
        .map(|w| WarehouseInfo {
            id: w.id.unwrap_or_default(),
            name: w.name.unwrap_or_default(),
            state: w.state.unwrap_or_default(),
            cluster_size: w.cluster_size.unwrap_or_default(),
            warehouse_type: w.warehouse_type,
        })
        .collect())
}

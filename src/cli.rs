use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dbtoon", about = "Multi-database query CLI with TOON output")]
pub struct Cli {
    /// Path to config file
    #[arg(short = 'c', long, global = true, env = "DBTOON_CONFIG")]
    pub config: Option<PathBuf>,

    /// Emit diagnostics to stderr
    #[arg(short = 'v', long, global = true, env = "DBTOON_VERBOSE")]
    pub verbose: bool,

    /// Disable credential masking
    #[arg(long, global = true, env = "DBTOON_SHOW_SECRETS")]
    pub show_secrets: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Execute a read-only query
    #[command(name = "exec-read")]
    ExecRead(ExecArgs),

    /// Execute a query without read-only validation (requires DBTOON_ALLOW_WRITE=true)
    #[command(name = "exec-write")]
    ExecWrite(ExecArgs),

    /// List available Databricks SQL warehouses
    #[command(name = "list-warehouses")]
    ListWarehouses(ListWarehousesArgs),

    /// Update dbtoon to the latest release
    Update,
}

#[derive(Parser, Debug)]
pub struct ExecArgs {
    /// SQL query text
    pub sql: Option<String>,

    /// Read SQL from file
    #[arg(short = 'f', long = "file", conflicts_with = "sql")]
    pub sql_file: Option<PathBuf>,

    /// Backend type: sqlserver or databricks
    #[arg(short = 'b', long, env = "DBTOON_BACKEND")]
    pub backend: Option<String>,

    /// SQL Server hostname
    #[arg(short = 's', long, env = "DBTOON_SERVER")]
    pub server: Option<String>,

    /// Database name
    #[arg(short = 'd', long, env = "DBTOON_DATABASE")]
    pub database: Option<String>,

    /// SQL Auth username
    #[arg(short = 'u', long, env = "DBTOON_USERNAME")]
    pub username: Option<String>,

    /// SQL Auth password
    #[arg(short = 'p', long, env = "DBTOON_PASSWORD")]
    pub password: Option<String>,

    /// Use Windows Integrated Auth
    #[arg(short = 'w', long, env = "DBTOON_WINDOWS_AUTH")]
    pub windows_auth: bool,

    /// Trust SQL Server certificate (for self-signed/dev instances)
    #[arg(long, env = "DBTOON_TRUST_SERVER_CERT")]
    pub trust_server_certificate: bool,

    /// Databricks workspace host
    #[arg(long, env = "DBTOON_DATABRICKS_HOST")]
    pub host: Option<String>,

    /// Databricks bearer token
    #[arg(long, env = "DBTOON_DATABRICKS_TOKEN")]
    pub token: Option<String>,

    /// Databricks SQL warehouse ID
    #[arg(long, env = "DBTOON_WAREHOUSE_ID")]
    pub warehouse: Option<String>,

    /// Databricks catalog
    #[arg(long, env = "DBTOON_CATALOG")]
    pub catalog: Option<String>,

    /// Databricks schema
    #[arg(long, env = "DBTOON_SCHEMA")]
    pub schema: Option<String>,

    /// Max rows to return (default: 500)
    #[arg(short = 'l', long, env = "DBTOON_ROW_LIMIT")]
    pub limit: Option<usize>,

    /// Disable row limit
    #[arg(long)]
    pub no_limit: bool,

    /// Query timeout in seconds (default: 60)
    #[arg(short = 't', long, env = "DBTOON_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Write results to file instead of stdout
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Config file profile name
    #[arg(short = 'P', long, env = "DBTOON_PROFILE")]
    pub profile: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ListWarehousesArgs {
    /// Databricks workspace host
    #[arg(long, env = "DBTOON_DATABRICKS_HOST")]
    pub host: Option<String>,

    /// Databricks bearer token
    #[arg(long, env = "DBTOON_DATABRICKS_TOKEN")]
    pub token: Option<String>,

    /// Config file profile name
    #[arg(short = 'P', long, env = "DBTOON_PROFILE")]
    pub profile: Option<String>,
}

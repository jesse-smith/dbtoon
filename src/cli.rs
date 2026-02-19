use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dbtoon", about = "Multi-database query CLI with TOON output")]
pub struct Cli {
    /// Path to config file
    #[arg(short = 'c', long, global = true)]
    pub config: Option<PathBuf>,

    /// Emit diagnostics to stderr
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    /// Disable credential masking
    #[arg(long, global = true)]
    pub show_secrets: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a config file with defaults and example profiles
    Init,

    /// Execute a SQL query against a profile
    Query(QueryArgs),

    /// Manage connection profiles
    #[command(subcommand)]
    Profile(ProfileCommand),

    /// Databricks warehouse operations
    Warehouse(WarehouseArgs),

    /// Update dbtoon to the latest release
    Update,
}

#[derive(Parser, Debug)]
pub struct QueryArgs {
    /// SQL query text
    #[arg(conflicts_with = "file")]
    pub sql: Option<String>,

    /// Read SQL from file
    #[arg(short = 'f', long)]
    pub file: Option<PathBuf>,

    /// Profile name
    #[arg(short = 'P', long, required = true)]
    pub profile: String,

    /// Override database/catalog
    #[arg(short = 'd', long, conflicts_with = "catalog")]
    pub database: Option<String>,

    /// Override catalog (alias for --database)
    #[arg(long, conflicts_with = "database")]
    pub catalog: Option<String>,

    /// Override schema
    #[arg(short = 's', long)]
    pub schema: Option<String>,

    /// Override row limit
    #[arg(short = 'l', long)]
    pub limit: Option<usize>,

    /// Disable row limit
    #[arg(long)]
    pub no_limit: bool,

    /// Override timeout in seconds
    #[arg(short = 't', long)]
    pub timeout: Option<u64>,

    /// Write results to file instead of stdout (format detected by extension)
    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    /// Bypass read-only safety validation
    #[arg(long)]
    pub allow_write: bool,
}

#[derive(Subcommand, Debug)]
pub enum ProfileCommand {
    /// Create a new connection profile
    Create(ProfileCreateArgs),

    /// Edit an existing profile
    Edit(ProfileEditArgs),

    /// Display profile with resolved values
    Show(ProfileNameArgs),

    /// List all profile names
    List,

    /// Test connectivity for a profile
    Test(ProfileNameArgs),

    /// Delete a profile
    Delete(ProfileNameArgs),

    /// Rename a profile
    Rename(ProfileRenameArgs),
}

#[derive(Parser, Debug)]
pub struct ProfileCreateArgs {
    /// Profile name
    pub name: String,

    /// Backend type: databricks or sqlserver
    #[arg(long, required = true)]
    pub backend: String,

    /// Set field values (repeatable, e.g., --set host=example.com)
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub set_fields: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct ProfileEditArgs {
    /// Profile name
    pub name: String,

    /// Set field values (repeatable, e.g., --set host=example.com; --set key= removes field)
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub set_fields: Vec<String>,

    /// Remove fields (repeatable)
    #[arg(long = "unset", value_name = "KEY")]
    pub unset_fields: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct ProfileNameArgs {
    /// Profile name
    pub name: String,
}

#[derive(Parser, Debug)]
pub struct ProfileRenameArgs {
    /// Current profile name
    pub old: String,

    /// New profile name
    pub new: String,
}

#[derive(Parser, Debug)]
pub struct WarehouseArgs {
    #[command(subcommand)]
    pub command: WarehouseCommand,
}

#[derive(Subcommand, Debug)]
pub enum WarehouseCommand {
    /// List available Databricks SQL warehouses
    List(WarehouseListArgs),
}

#[derive(Parser, Debug)]
pub struct WarehouseListArgs {
    /// Databricks profile name
    #[arg(short = 'P', long, required = true)]
    pub profile: String,
}

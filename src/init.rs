//! Config file initialization for `dbtoon init`.

use crate::config::env_non_empty;
use crate::error::DbtoonError;
use std::path::Path;

const DEFAULT_TEMPLATE: &str = r#"[defaults]
row_limit = 500
timeout = 60

# Example profiles — uncomment and configure:
#
# [profiles.sqlserver_example]
# backend = "sqlserver"
# server = "localhost"
# database = "mydb"
# username = "sa"
# password = "$SA_PASSWORD"
#
# [profiles.databricks_example]
# backend = "databricks"
# host = "$DATABRICKS_HOST"
# token = "$DATABRICKS_TOKEN"
# warehouse_id = "$DATABRICKS_SQL_WAREHOUSE_ID"
# catalog = "$DATABRICKS_CATALOG"
# schema = "$DATABRICKS_SCHEMA"
"#;

const DATABRICKS_ACTIVE_TEMPLATE: &str = r#"[defaults]
row_limit = 500
timeout = 60

# Example SQL Server profile — uncomment and configure:
#
# [profiles.sqlserver_example]
# backend = "sqlserver"
# server = "localhost"
# database = "mydb"
# username = "sa"
# password = "$SA_PASSWORD"

[profiles.databricks]
backend = "databricks"
host = "$DATABRICKS_HOST"
token = "$DATABRICKS_TOKEN"
warehouse_id = "$DATABRICKS_SQL_WAREHOUSE_ID"
catalog = "$DATABRICKS_CATALOG"
schema = "$DATABRICKS_SCHEMA"
"#;

/// Run the `dbtoon init` command: create config file with defaults and example profiles.
pub fn run_init(config_path: &Path) -> Result<(), DbtoonError> {
    if config_path.exists() {
        return Err(DbtoonError::Config {
            message: format!(
                "config file already exists: {}\n\nTo recreate, delete it first.",
                config_path.display()
            ),
        });
    }

    // Create parent directory tree
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DbtoonError::Config {
            message: format!("cannot create config directory {}: {}", parent.display(), e),
        })?;
    }

    // Detect Databricks env vars
    let has_databricks = env_non_empty("DATABRICKS_HOST").is_some()
        || env_non_empty("DATABRICKS_TOKEN").is_some();

    let template = if has_databricks {
        DATABRICKS_ACTIVE_TEMPLATE
    } else {
        DEFAULT_TEMPLATE
    };

    std::fs::write(config_path, template).map_err(|e| DbtoonError::Config {
        message: format!("cannot write config file {}: {}", config_path.display(), e),
    })?;

    // Print guidance
    eprintln!("Created config file: {}", config_path.display());

    if has_databricks {
        eprintln!("\nDetected Databricks environment variables.");
        eprintln!("A 'databricks' profile has been created with $VAR references.");

        // Check which required fields are still missing
        let mut missing = Vec::new();
        if env_non_empty("DATABRICKS_HOST").is_none() {
            missing.push("DATABRICKS_HOST");
        }
        if env_non_empty("DATABRICKS_TOKEN").is_none() {
            missing.push("DATABRICKS_TOKEN");
        }
        if env_non_empty("DATABRICKS_SQL_WAREHOUSE_ID").is_none() {
            missing.push("DATABRICKS_SQL_WAREHOUSE_ID");
        }
        if !missing.is_empty() {
            eprintln!("\nRequired env vars still missing: {}", missing.join(", "));
        }

        eprintln!("\nNext: dbtoon query -P databricks \"SELECT 1\"");
    } else {
        eprintln!("\nNext steps:");
        eprintln!("  1. Create a profile: dbtoon profile create mydb --backend sqlserver");
        eprintln!("  2. Or edit the config file directly: {}", config_path.display());
    }

    Ok(())
}

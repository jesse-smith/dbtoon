use clap::Parser;
use dbtoon::cli::{self, Cli, Command, ProfileCommand};
use dbtoon::error::DbtoonError;
use dbtoon::verbose::{self, Timer};
use dbtoon::format_detect::{self, OutputFormat};
use dbtoon::{backend, config, format, output, validation};
use std::process;

#[tokio::main]
async fn main() {
    // Load .env file (optional, ignore if missing)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Init => run_init(cli.config.as_ref()),
        Command::Query(ref args) => {
            run_query(args, cli.verbose, cli.show_secrets, cli.config.as_ref()).await
        }
        Command::Profile(ref cmd) => {
            run_profile(cmd, cli.verbose, cli.show_secrets, cli.config.as_ref())
        }
        Command::Warehouse(ref args) => {
            run_warehouse(args, cli.verbose, cli.show_secrets, cli.config.as_ref()).await
        }
        Command::Update => dbtoon::update::run_update().map_err(|e| DbtoonError::Config {
            message: e.to_string(),
        }),
    };

    if let Err(err) = result {
        output::print_error(&err);
        process::exit(1);
    }
}

fn run_init(config_path: Option<&std::path::PathBuf>) -> Result<(), DbtoonError> {
    let path = match config_path {
        Some(p) => p.clone(),
        None => config::default_config_path().ok_or_else(|| DbtoonError::Config {
            message: "cannot determine config file location (HOME not set)".to_string(),
        })?,
    };
    dbtoon::init::run_init(&path)
}

async fn run_query(
    args: &cli::QueryArgs,
    verbose: bool,
    show_secrets: bool,
    config_path: Option<&std::path::PathBuf>,
) -> Result<(), DbtoonError> {
    let (toml_config, _config_file_path) = config::load_toml_config_required(config_path)?;
    let app_config = config::load_from_query_args(args, &toml_config, verbose, show_secrets)?;
    let verbose = app_config.verbose;

    // Resolve SQL input
    let sql = resolve_sql(&args.sql, &args.file)?;

    // Validation: block write queries unless --allow-write
    let dialect = match &app_config.backend {
        config::BackendConfig::SqlServer { .. } => validation::BackendDialect::SqlServer,
        config::BackendConfig::Databricks { .. } => validation::BackendDialect::Databricks,
    };

    if !app_config.allow_write {
        verbose::emit(verbose, "validating query (read-only mode)...");
        let vtimer = Timer::start();
        let validation_result = validation::validate(&sql, dialect);
        match validation_result {
            validation::ValidationResult::Safe => {
                verbose::emit(
                    verbose,
                    &format!("validation passed ({}ms)", vtimer.elapsed_ms()),
                );
            }
            validation::ValidationResult::Denied { reasons } => {
                verbose::emit(verbose, "validation failed");
                let detail = reasons
                    .iter()
                    .map(|r| r.detail.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(DbtoonError::Validation { reason: detail });
            }
        }
    } else {
        verbose::emit(verbose, "write mode enabled — skipping validation");
    }

    // Detect output format before query (fail-fast on bad extension)
    let format_info = if let Some(ref path) = app_config.output_file {
        Some(format_detect::detect_format(path)?)
    } else {
        None
    };

    // Execute query
    let result = execute_query(&app_config, &sql, verbose).await?;

    // Format and output
    let format_label = match &format_info {
        Some((OutputFormat::Toon, _)) => "TOON",
        Some((OutputFormat::Csv, _)) => "CSV",
        Some((OutputFormat::Parquet, _)) => "Parquet",
        Some((OutputFormat::Arrow, _)) => "Arrow IPC",
        None => "TOON",
    };
    verbose::emit(verbose, &format!("formatting {format_label} output..."));
    output_result(&app_config, &result, format_info)?;

    Ok(())
}

fn run_profile(
    cmd: &ProfileCommand,
    _verbose: bool,
    show_secrets: bool,
    config_path: Option<&std::path::PathBuf>,
) -> Result<(), DbtoonError> {
    let (_, config_file_path) = config::load_toml_config_required(config_path)?;

    match cmd {
        ProfileCommand::Create(args) => {
            dbtoon::profile::create_profile(&config_file_path, &args.name, &args.backend, &args.set_fields)
        }
        ProfileCommand::Edit(args) => {
            dbtoon::profile::edit_profile(&config_file_path, &args.name, &args.set_fields, &args.unset_fields)
        }
        ProfileCommand::Show(args) => {
            let output = dbtoon::profile::show_profile(&config_file_path, &args.name, show_secrets)?;
            print!("{}", output);
            Ok(())
        }
        ProfileCommand::List => {
            let names = dbtoon::profile::list_profiles(&config_file_path)?;
            for name in &names {
                println!("{}", name);
            }
            Ok(())
        }
        ProfileCommand::Test(args) => {
            dbtoon::profile::test_profile(&config_file_path, &args.name)
        }
        ProfileCommand::Delete(args) => {
            dbtoon::profile::delete_profile(&config_file_path, &args.name)
        }
        ProfileCommand::Rename(args) => {
            dbtoon::profile::rename_profile(&config_file_path, &args.old, &args.new)
        }
    }
}

async fn run_warehouse(
    args: &cli::WarehouseArgs,
    verbose: bool,
    show_secrets: bool,
    config_path: Option<&std::path::PathBuf>,
) -> Result<(), DbtoonError> {
    match &args.command {
        cli::WarehouseCommand::List(list_args) => {
            let (toml_config, _) = config::load_toml_config_required(config_path)?;
            let app_config = config::load_from_warehouse_list_args(
                list_args, &toml_config, verbose, show_secrets,
            )?;
            let verbose = app_config.verbose;

            let (host, token) = match &app_config.backend {
                config::BackendConfig::Databricks { host, token, .. } => (host.clone(), token.clone()),
                _ => {
                    return Err(DbtoonError::Config {
                        message: "warehouse list requires a databricks profile".to_string(),
                    });
                }
            };

            verbose::emit(verbose, &format!("listing warehouses on {}...", host));
            let timer = Timer::start();
            let warehouses = backend::databricks::list_warehouses(&host, &token).await?;
            verbose::emit(
                verbose,
                &format!(
                    "warehouse list retrieved ({}ms, {} warehouses)",
                    timer.elapsed_ms(),
                    warehouses.len()
                ),
            );

            let columns = vec![
                backend::ColumnMeta { name: "id".to_string(), type_name: "STRING".to_string() },
                backend::ColumnMeta { name: "name".to_string(), type_name: "STRING".to_string() },
                backend::ColumnMeta { name: "state".to_string(), type_name: "STRING".to_string() },
                backend::ColumnMeta { name: "cluster_size".to_string(), type_name: "STRING".to_string() },
                backend::ColumnMeta { name: "type".to_string(), type_name: "STRING".to_string() },
            ];

            let rows = warehouses
                .into_iter()
                .map(|w| {
                    vec![
                        backend::CellValue::Text(w.id),
                        backend::CellValue::Text(w.name),
                        backend::CellValue::Text(w.state),
                        backend::CellValue::Text(w.cluster_size),
                        match w.warehouse_type {
                            Some(t) => backend::CellValue::Text(t),
                            None => backend::CellValue::Null,
                        },
                    ]
                })
                .collect();

            let query_result = backend::QueryResult {
                columns,
                rows,
                total_rows: None,
                truncated: false,
            };

            let toon = format::to_toon(&query_result, false, None)?;
            output::print_result(&toon);

            Ok(())
        }
    }
}

// --- Helpers ---

fn resolve_sql(sql: &Option<String>, file: &Option<std::path::PathBuf>) -> Result<String, DbtoonError> {
    if let Some(sql) = sql {
        return Ok(sql.clone());
    }
    if let Some(path) = file {
        let content = std::fs::read_to_string(path).map_err(|e| DbtoonError::Config {
            message: format!("cannot read SQL file {}: {}", path.display(), e),
        })?;
        return Ok(content);
    }
    Err(DbtoonError::Config {
        message: "no SQL provided — use positional argument or --file".to_string(),
    })
}

async fn execute_query(
    app_config: &config::AppConfig,
    sql: &str,
    verbose: bool,
) -> Result<backend::QueryResult, DbtoonError> {
    use backend::Backend;

    match &app_config.backend {
        config::BackendConfig::SqlServer {
            server,
            database,
            auth,
            trust_server_certificate,
        } => {
            verbose::emit(verbose, &format!("connecting to sqlserver at {}...", server));
            let timer = Timer::start();
            let backend_impl = backend::sqlserver::SqlServerBackend::new(
                server.clone(),
                database.clone(),
                clone_auth(auth),
                *trust_server_certificate,
            );
            verbose::emit(verbose, "executing query...");
            let result = backend_impl
                .execute(sql, app_config.default_row_limit, app_config.query_timeout_secs)
                .await?;
            verbose::emit(
                verbose,
                &format!(
                    "query complete ({}ms, {} rows)",
                    timer.elapsed_ms(),
                    result.rows.len()
                ),
            );
            Ok(result)
        }
        config::BackendConfig::Databricks {
            host,
            token,
            warehouse_id,
            catalog,
            schema,
        } => {
            verbose::emit(verbose, &format!("connecting to databricks at {}...", host));
            let timer = Timer::start();
            let backend_impl = backend::databricks::DatabricksBackend::new(
                host.clone(),
                clone_secret(token),
                warehouse_id.clone(),
                catalog.clone(),
                schema.clone(),
            );
            verbose::emit(verbose, "executing query...");
            let result = backend_impl
                .execute(sql, app_config.default_row_limit, app_config.query_timeout_secs)
                .await?;
            verbose::emit(
                verbose,
                &format!(
                    "query complete ({}ms, {} rows)",
                    timer.elapsed_ms(),
                    result.rows.len()
                ),
            );
            Ok(result)
        }
    }
}

fn clone_auth(auth: &config::SqlServerAuth) -> config::SqlServerAuth {
    match auth {
        config::SqlServerAuth::WindowsIntegrated => config::SqlServerAuth::WindowsIntegrated,
        config::SqlServerAuth::SqlLogin { username, password } => {
            use secrecy::ExposeSecret;
            config::SqlServerAuth::SqlLogin {
                username: username.clone(),
                password: secrecy::SecretString::from(password.expose_secret().to_string()),
            }
        }
    }
}

fn clone_secret(secret: &secrecy::SecretString) -> secrecy::SecretString {
    use secrecy::ExposeSecret;
    secrecy::SecretString::from(secret.expose_secret().to_string())
}

fn output_result(
    app_config: &config::AppConfig,
    result: &backend::QueryResult,
    format_info: Option<(OutputFormat, std::path::PathBuf)>,
) -> Result<(), DbtoonError> {
    let message = if result.truncated {
        Some(format!(
            "Showing {} rows. Use --no-limit to return all rows.",
            result.rows.len()
        ))
    } else {
        None
    };

    if let Some((format, path)) = format_info {
        verbose::emit(
            app_config.verbose,
            &format!("writing output to {}...", path.display()),
        );
        match format {
            OutputFormat::Toon => {
                let toon = format::to_toon(result, result.truncated, message.as_deref())?;
                output::write_file(&toon, &path)?;
            }
            OutputFormat::Csv => {
                dbtoon::format_csv::write_csv(result, &path)?;
            }
            OutputFormat::Parquet => {
                dbtoon::format_parquet::write_parquet(
                    result, &path, result.truncated, message.as_deref(),
                )?;
            }
            OutputFormat::Arrow => {
                dbtoon::format_arrow::write_arrow(
                    result, &path, result.truncated, message.as_deref(),
                )?;
            }
        }
        output::print_summary(
            result.rows.len(), &path, result.truncated, message.as_deref(),
        )?;
    } else {
        let toon = format::to_toon(result, result.truncated, message.as_deref())?;
        output::print_result(&toon);
    }

    if let Some(ref msg) = message {
        output::print_truncation_warning(msg);
    }

    Ok(())
}

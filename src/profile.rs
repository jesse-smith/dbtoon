//! Profile management for `dbtoon profile` subcommands.

use crate::config::resolve_env_var;
use crate::error::DbtoonError;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table, value};

/// Valid fields per backend (excluding `backend` which is always set at creation).
const SQLSERVER_FIELDS: &[&str] = &[
    "server", "database", "username", "password",
    "windows_auth", "trust_server_certificate",
];

const DATABRICKS_FIELDS: &[&str] = &[
    "host", "token", "warehouse_id", "catalog", "schema",
];

/// Secret fields that should be masked in `profile show`.
const SECRET_FIELDS: &[&str] = &["password", "token"];

fn valid_fields(backend: &str) -> Result<&'static [&'static str], DbtoonError> {
    match backend {
        "sqlserver" => Ok(SQLSERVER_FIELDS),
        "databricks" => Ok(DATABRICKS_FIELDS),
        other => Err(DbtoonError::Config {
            message: format!("unknown backend type: '{}' (expected 'sqlserver' or 'databricks')", other),
        }),
    }
}

fn validate_field(backend: &str, field: &str) -> Result<(), DbtoonError> {
    let fields = valid_fields(backend)?;
    if !fields.contains(&field) && field != "backend" {
        return Err(DbtoonError::Config {
            message: format!(
                "invalid field '{}' for {} backend (valid: {})",
                field, backend, fields.join(", ")
            ),
        });
    }
    Ok(())
}

fn read_doc(path: &Path) -> Result<DocumentMut, DbtoonError> {
    let content = std::fs::read_to_string(path).map_err(|e| DbtoonError::Config {
        message: format!("cannot read config file {}: {}", path.display(), e),
    })?;
    content.parse::<DocumentMut>().map_err(|e| DbtoonError::Config {
        message: format!("invalid TOML in {}: {}", path.display(), e),
    })
}

fn write_doc(path: &Path, doc: &DocumentMut) -> Result<(), DbtoonError> {
    std::fs::write(path, doc.to_string()).map_err(|e| DbtoonError::Config {
        message: format!("cannot write config file {}: {}", path.display(), e),
    })
}

/// Ensure the `[profiles]` table exists in the document.
fn ensure_profiles_table(doc: &mut DocumentMut) {
    if !doc.contains_key("profiles") {
        doc["profiles"] = Item::Table(Table::new());
    }
}

fn parse_set_field(kv: &str) -> Result<(&str, &str), DbtoonError> {
    let Some((key, val)) = kv.split_once('=') else {
        return Err(DbtoonError::Config {
            message: format!("invalid --set syntax: '{}' (expected key=value)", kv),
        });
    };
    Ok((key.trim(), val.trim()))
}

/// Create a new profile in the config file.
pub fn create_profile(
    config_path: &Path,
    name: &str,
    backend: &str,
    set_fields: &[String],
) -> Result<(), DbtoonError> {
    // Validate backend
    let valid = valid_fields(backend)?;

    // Validate --set fields
    for kv in set_fields {
        let (key, _) = parse_set_field(kv)?;
        validate_field(backend, key)?;
    }

    let mut doc = read_doc(config_path)?;
    ensure_profiles_table(&mut doc);

    // Check for duplicate
    if let Some(profiles) = doc["profiles"].as_table()
        && profiles.contains_key(name) {
            return Err(DbtoonError::Config {
                message: format!("profile '{}' already exists", name),
            });
        }

    // Build profile table with defaults
    let mut profile = Table::new();
    profile["backend"] = value(backend);

    match backend {
        "databricks" => {
            profile["host"] = value("$DATABRICKS_HOST");
            profile["token"] = value("$DATABRICKS_TOKEN");
            profile["warehouse_id"] = value("$DATABRICKS_SQL_WAREHOUSE_ID");
            profile["catalog"] = value("$DATABRICKS_CATALOG");
            profile["schema"] = value("$DATABRICKS_SCHEMA");
        }
        "sqlserver" => {
            profile["server"] = value("localhost");
            profile["database"] = value("mydb");
            profile["username"] = value("sa");
            profile["password"] = value("$SA_PASSWORD");
        }
        _ => {} // Already validated above
    }

    // Apply --set overrides
    for kv in set_fields {
        let (key, val) = parse_set_field(kv)?;
        if val.is_empty() {
            profile.remove(key);
        } else {
            // Bool fields
            if key == "windows_auth" || key == "trust_server_certificate" {
                match val {
                    "true" => { profile[key] = value(true); }
                    "false" => { profile[key] = value(false); }
                    _ => {
                        return Err(DbtoonError::Config {
                            message: format!("field '{}' must be 'true' or 'false', got '{}'", key, val),
                        });
                    }
                }
            } else {
                profile[key] = value(val);
            }
        }
    }

    doc["profiles"][name] = Item::Table(profile);
    write_doc(config_path, &doc)?;

    eprintln!("Created profile '{}'", name);
    let _ = valid; // suppress unused warning
    Ok(())
}

/// Edit an existing profile.
pub fn edit_profile(
    config_path: &Path,
    name: &str,
    set_fields: &[String],
    unset_fields: &[String],
) -> Result<(), DbtoonError> {
    let mut doc = read_doc(config_path)?;

    // Get the profile and its backend
    let backend = {
        let profiles = doc.get("profiles")
            .and_then(|p| p.as_table())
            .ok_or_else(|| DbtoonError::Config {
                message: format!("profile '{}' not found in config file", name),
            })?;
        let profile = profiles.get(name).ok_or_else(|| DbtoonError::Config {
            message: format!("profile '{}' not found in config file", name),
        })?;
        profile.get("backend")
            .and_then(|b| b.as_str())
            .unwrap_or("unknown")
            .to_string()
    };

    // Validate fields
    for kv in set_fields {
        let (key, _) = parse_set_field(kv)?;
        if key != "backend" {
            validate_field(&backend, key)?;
        }
    }
    for key in unset_fields {
        if key != "backend" {
            validate_field(&backend, key)?;
        }
    }

    // Apply changes
    for kv in set_fields {
        let (key, val) = parse_set_field(kv)?;
        if val.is_empty() {
            // Remove field
            if let Some(profile) = doc["profiles"][name].as_table_mut() {
                profile.remove(key);
            }
        } else {
            // Bool fields
            if key == "windows_auth" || key == "trust_server_certificate" {
                match val {
                    "true" => { doc["profiles"][name][key] = value(true); }
                    "false" => { doc["profiles"][name][key] = value(false); }
                    _ => {
                        return Err(DbtoonError::Config {
                            message: format!("field '{}' must be 'true' or 'false', got '{}'", key, val),
                        });
                    }
                }
            } else {
                doc["profiles"][name][key] = value(val);
            }
        }
    }

    for key in unset_fields {
        if let Some(profile) = doc["profiles"][name].as_table_mut() {
            profile.remove(key.as_str());
        }
    }

    write_doc(config_path, &doc)?;
    eprintln!("Updated profile '{}'", name);
    Ok(())
}

/// Show a profile with resolved values and masking.
pub fn show_profile(
    config_path: &Path,
    name: &str,
    show_secrets: bool,
) -> Result<String, DbtoonError> {
    let doc = read_doc(config_path)?;

    let profiles = doc["profiles"].as_table().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", name),
    })?;
    let profile = profiles.get(name).ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", name),
    })?;
    let profile_table = profile.as_table().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' is not a valid table", name),
    })?;

    let mut output = format!("[profiles.{}]\n", name);

    for (key, item) in profile_table.iter() {
        let raw_value = item.as_str().unwrap_or_default();
        let is_secret = SECRET_FIELDS.contains(&key);

        if let Some(var_name) = raw_value.strip_prefix('$')
            && !var_name.starts_with('$') {
                // It's a $VAR reference
                match resolve_env_var(raw_value) {
                    Ok(resolved) => {
                        let display = if is_secret && !show_secrets {
                            "****".to_string()
                        } else {
                            resolved
                        };
                        output.push_str(&format!(
                            "{} = \"{}\" (${} = \"{}\")\n",
                            key, raw_value, var_name, display
                        ));
                    }
                    Err(_) => {
                        output.push_str(&format!(
                            "{} = \"{}\" (WARNING: ${} is not set)\n",
                            key, raw_value, var_name
                        ));
                    }
                }
                continue;
            }

        // Literal value or bool
        if is_secret && !show_secrets {
            if item.as_str().is_some() {
                output.push_str(&format!("{} = \"****\"\n", key));
            } else {
                output.push_str(&format!("{} = {}\n", key, item));
            }
        } else {
            output.push_str(&format!("{} = {}\n", key, item));
        }
    }

    Ok(output)
}

/// List all profile names.
pub fn list_profiles(config_path: &Path) -> Result<Vec<String>, DbtoonError> {
    let doc = read_doc(config_path)?;

    let names = match doc["profiles"].as_table() {
        Some(profiles) => profiles.iter().map(|(k, _)| k.to_string()).collect(),
        None => Vec::new(),
    };

    Ok(names)
}

/// Delete a profile.
pub fn delete_profile(config_path: &Path, name: &str) -> Result<(), DbtoonError> {
    let mut doc = read_doc(config_path)?;

    let profiles = doc["profiles"].as_table_mut().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", name),
    })?;

    if profiles.remove(name).is_none() {
        return Err(DbtoonError::Config {
            message: format!("profile '{}' not found in config file", name),
        });
    }

    write_doc(config_path, &doc)?;
    eprintln!("Deleted profile '{}'", name);
    Ok(())
}

/// Test a profile by validating required fields and attempting connection.
///
/// This validates that all required fields for the backend are present and resolvable.
/// Actual connectivity testing requires a backend connection, which is only attempted
/// if field validation passes.
pub fn test_profile(config_path: &Path, name: &str) -> Result<(), DbtoonError> {
    let doc = read_doc(config_path)?;

    let profiles = doc.get("profiles")
        .and_then(|p| p.as_table())
        .ok_or_else(|| DbtoonError::Config {
            message: format!("profile '{}' not found in config file", name),
        })?;
    let profile = profiles.get(name).ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", name),
    })?;
    let profile_table = profile.as_table().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' is not a valid table", name),
    })?;

    let backend = profile_table.get("backend")
        .and_then(|b| b.as_str())
        .ok_or_else(|| DbtoonError::Config {
            message: format!("profile '{}' has no 'backend' field", name),
        })?;

    // Check required fields
    let required: &[&str] = match backend {
        "sqlserver" => &["server"],
        "databricks" => &["host", "token", "warehouse_id"],
        other => return Err(DbtoonError::Config {
            message: format!("unknown backend type: '{}'", other),
        }),
    };

    let mut missing = Vec::new();
    for field in required {
        match profile_table.get(field) {
            Some(item) if item.as_str().is_some() => {
                // Try to resolve $VAR references
                let val = item.as_str().unwrap();
                if let Err(e) = resolve_env_var(val) {
                    return Err(DbtoonError::Config {
                        message: format!("profile '{}' field '{}': {}", name, field, e),
                    });
                }
            }
            _ => missing.push(*field),
        }
    }

    if !missing.is_empty() {
        return Err(DbtoonError::Config {
            message: format!(
                "profile '{}' is missing required fields: {}",
                name, missing.join(", ")
            ),
        });
    }

    // If we got here, all required fields are present and resolvable.
    // Actual connectivity testing would require backend connection here.
    eprintln!("Profile '{}' configuration is valid ({} backend)", name, backend);
    eprintln!("Note: connectivity testing requires a live connection (not yet implemented)");
    Ok(())
}

/// Rename a profile.
pub fn rename_profile(
    config_path: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<(), DbtoonError> {
    let mut doc = read_doc(config_path)?;

    let profiles = doc["profiles"].as_table_mut().ok_or_else(|| DbtoonError::Config {
        message: format!("profile '{}' not found in config file", old_name),
    })?;

    if !profiles.contains_key(old_name) {
        return Err(DbtoonError::Config {
            message: format!("profile '{}' not found in config file", old_name),
        });
    }

    if profiles.contains_key(new_name) {
        return Err(DbtoonError::Config {
            message: format!("profile '{}' already exists", new_name),
        });
    }

    // Get the old profile's item, clone it, insert with new name, remove old
    let old_item = profiles.get(old_name).unwrap().clone();
    profiles.insert(new_name, old_item);
    profiles.remove(old_name);

    write_doc(config_path, &doc)?;
    eprintln!("Renamed profile '{}' to '{}'", old_name, new_name);
    Ok(())
}

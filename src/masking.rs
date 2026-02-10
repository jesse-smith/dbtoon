use secrecy::{ExposeSecret, SecretString};

/// Format a secret value, respecting the show_secrets flag.
pub fn format_secret(secret: &SecretString, show_secrets: bool) -> String {
    if show_secrets {
        secret.expose_secret().to_string()
    } else {
        "[REDACTED]".to_string()
    }
}

/// Format an optional secret value.
pub fn format_optional_secret(secret: Option<&SecretString>, show_secrets: bool) -> String {
    match secret {
        Some(s) => format_secret(s, show_secrets),
        None => "(not set)".to_string(),
    }
}

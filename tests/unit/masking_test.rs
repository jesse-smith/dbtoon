use dbtoon::masking::{format_optional_secret, format_secret};
use secrecy::SecretString;

#[test]
fn test_secret_masked_by_default() {
    let secret = SecretString::from("my-super-secret-password".to_string());
    let output = format_secret(&secret, false);
    assert_eq!(output, "[REDACTED]");
    assert!(!output.contains("my-super-secret-password"));
}

#[test]
fn test_secret_exposed_with_show_secrets() {
    let secret = SecretString::from("my-super-secret-password".to_string());
    let output = format_secret(&secret, true);
    assert_eq!(output, "my-super-secret-password");
}

#[test]
fn test_secret_debug_is_redacted() {
    let secret = SecretString::from("password123".to_string());
    let debug_output = format!("{:?}", secret);
    assert!(!debug_output.contains("password123"), "Debug should not expose secret: {}", debug_output);
}

#[test]
fn test_optional_secret_none() {
    let output = format_optional_secret(None, false);
    assert_eq!(output, "(not set)");
}

#[test]
fn test_optional_secret_some_masked() {
    let secret = SecretString::from("token123".to_string());
    let output = format_optional_secret(Some(&secret), false);
    assert_eq!(output, "[REDACTED]");
}

#[test]
fn test_optional_secret_some_exposed() {
    let secret = SecretString::from("token123".to_string());
    let output = format_optional_secret(Some(&secret), true);
    assert_eq!(output, "token123");
}

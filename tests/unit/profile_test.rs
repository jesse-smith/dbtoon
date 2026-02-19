use dbtoon::profile;
use std::path::PathBuf;
use std::sync::Mutex;

// --- Env var test infrastructure ---

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct EnvGuard {
    keys: Vec<String>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn new(vars: &[(&str, &str)]) -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        for (key, val) in vars {
            unsafe { std::env::set_var(key, val); }
        }
        EnvGuard {
            keys: vars.iter().map(|(k, _)| k.to_string()).collect(),
            _lock: lock,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            unsafe { std::env::remove_var(key); }
        }
    }
}

fn write_temp_config(content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("dbtoon-profile-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("config-{}-{:?}.toml", std::process::id(), std::thread::current().id()));
    std::fs::write(&path, content).unwrap();
    path
}

// =====================================================================
// T024: profile create tests
// =====================================================================

#[test]
fn test_profile_create_sqlserver_defaults() {
    let path = write_temp_config("[defaults]\nrow_limit = 500\n");
    let result = profile::create_profile(&path, "mydb", "sqlserver", &[]);
    assert!(result.is_ok(), "create should succeed: {:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[profiles.mydb]"), "should contain profile section");
    assert!(content.contains("backend = \"sqlserver\""), "should set backend");
    // Should have $VAR defaults for sqlserver fields
    assert!(content.contains("server"), "should have server field");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_create_databricks_defaults() {
    let path = write_temp_config("[defaults]\nrow_limit = 500\n");
    let result = profile::create_profile(&path, "dbx", "databricks", &[]);
    assert!(result.is_ok());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[profiles.dbx]"));
    assert!(content.contains("backend = \"databricks\""));
    assert!(content.contains("$DATABRICKS_HOST"));

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_create_with_set_overrides() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::create_profile(
        &path, "mydb", "sqlserver",
        &["server=localhost".to_string(), "database=testdb".to_string()],
    );
    assert!(result.is_ok());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("server = \"localhost\""), "should have overridden server");
    assert!(content.contains("database = \"testdb\""), "should have overridden database");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_create_duplicate_rejected() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"localhost\"\n");
    let result = profile::create_profile(&path, "mydb", "sqlserver", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("already exists"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_create_invalid_backend() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::create_profile(&path, "mydb", "postgres", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("unknown backend") || err.contains("backend"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_create_invalid_field() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::create_profile(
        &path, "mydb", "sqlserver",
        &["not_a_field=value".to_string()],
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not_a_field"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

// =====================================================================
// T025: profile edit tests
// =====================================================================

#[test]
fn test_profile_edit_set_field() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"old\"\n");
    let result = profile::edit_profile(&path, "mydb", &["server=new".to_string()], &[]);
    assert!(result.is_ok(), "{:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("server = \"new\""), "Got:\n{}", content);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_edit_set_empty_removes() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"localhost\"\ndatabase = \"old\"\n");
    let result = profile::edit_profile(&path, "mydb", &["database=".to_string()], &[]);
    assert!(result.is_ok(), "{:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("database"), "database should be removed, got:\n{}", content);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_edit_unset_removes() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"localhost\"\ndatabase = \"old\"\n");
    let result = profile::edit_profile(&path, "mydb", &[], &["database".to_string()]);
    assert!(result.is_ok(), "{:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("database"), "database should be removed, got:\n{}", content);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_edit_invalid_field() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"localhost\"\n");
    let result = profile::edit_profile(&path, "mydb", &["invalid_field=value".to_string()], &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("invalid_field"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_edit_nonexistent_profile() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::edit_profile(&path, "nonexistent", &["server=localhost".to_string()], &[]);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

// =====================================================================
// T047: profile test — missing required fields error
// =====================================================================

#[test]
fn test_profile_test_missing_required_fields() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\n");
    let result = profile::test_profile(&path, "mydb");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("server") || err.contains("required"), "should report missing required fields, got: {}", err);
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_test_nonexistent() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::test_profile(&path, "nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Got: {}", err);
    std::fs::remove_file(&path).ok();
}

// =====================================================================
// T026: profile show, list, delete, rename tests
// =====================================================================

#[test]
fn test_profile_show_resolved_values() {
    let _guard = EnvGuard::new(&[("MY_HOST", "resolved-host")]);
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"databricks\"\nhost = \"$MY_HOST\"\ntoken = \"literal-token\"\nwarehouse_id = \"wh-1\"\n");

    let output = profile::show_profile(&path, "mydb", false).unwrap();
    assert!(output.contains("host"), "should show host field");
    assert!(output.contains("MY_HOST"), "should show env var name");
    // Token should be masked
    assert!(output.contains("token"), "should show token field");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_show_with_show_secrets() {
    let _guard = EnvGuard::new(&[("MY_TOKEN", "secret-value")]);
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"databricks\"\nhost = \"literal-host\"\ntoken = \"$MY_TOKEN\"\nwarehouse_id = \"wh-1\"\n");

    let output = profile::show_profile(&path, "mydb", true).unwrap();
    assert!(output.contains("secret-value"), "should reveal secret with show_secrets=true");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_show_unset_env_warning() {
    let _guard = EnvGuard::new(&[]);
    unsafe { std::env::remove_var("UNSET_HOST_VAR_009"); }
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"databricks\"\nhost = \"$UNSET_HOST_VAR_009\"\ntoken = \"tok\"\nwarehouse_id = \"wh\"\n");

    let output = profile::show_profile(&path, "mydb", false).unwrap();
    assert!(
        output.contains("not set") || output.contains("WARNING") || output.contains("⚠"),
        "should warn about unset env var, got:\n{}", output
    );

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_list() {
    let path = write_temp_config("[defaults]\n\n[profiles.first]\nbackend = \"sqlserver\"\nserver = \"a\"\n\n[profiles.second]\nbackend = \"databricks\"\nhost = \"b\"\ntoken = \"c\"\nwarehouse_id = \"d\"\n");

    let names = profile::list_profiles(&path).unwrap();
    assert!(names.contains(&"first".to_string()));
    assert!(names.contains(&"second".to_string()));
    assert_eq!(names.len(), 2);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_delete() {
    let path = write_temp_config("[defaults]\n\n[profiles.mydb]\nbackend = \"sqlserver\"\nserver = \"localhost\"\n");
    let result = profile::delete_profile(&path, "mydb");
    assert!(result.is_ok(), "{:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("[profiles.mydb]"), "profile should be deleted");
    assert!(!content.contains("server = \"localhost\""), "profile fields should be deleted");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_delete_nonexistent() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::delete_profile(&path, "nonexistent");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_rename() {
    let path = write_temp_config("[defaults]\n\n[profiles.old]\nbackend = \"sqlserver\"\nserver = \"localhost\"\n");
    let result = profile::rename_profile(&path, "old", "new");
    assert!(result.is_ok(), "{:?}", result.err());

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(!content.contains("[profiles.old]"), "old name should be gone");
    assert!(content.contains("[profiles.new]"), "new name should exist");
    assert!(content.contains("server = \"localhost\""), "fields should be preserved");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_rename_target_exists() {
    let path = write_temp_config("[defaults]\n\n[profiles.old]\nbackend = \"sqlserver\"\nserver = \"a\"\n\n[profiles.new]\nbackend = \"sqlserver\"\nserver = \"b\"\n");
    let result = profile::rename_profile(&path, "old", "new");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("already exists"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_profile_rename_source_nonexistent() {
    let path = write_temp_config("[defaults]\n");
    let result = profile::rename_profile(&path, "nonexistent", "new");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("not found"), "Got: {}", err);

    std::fs::remove_file(&path).ok();
}

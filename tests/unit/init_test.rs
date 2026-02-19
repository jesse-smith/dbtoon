use dbtoon::init;
use std::sync::Mutex;

// --- Env var test infrastructure (shared pattern) ---

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

// =====================================================================
// T013: Init template generation tests
// =====================================================================

#[test]
fn test_init_creates_config_file() {
    let dir = std::env::temp_dir().join("dbtoon-init-test-creates");
    let _ = std::fs::remove_dir_all(&dir);
    let config_path = dir.join("config.toml");

    let result = init::run_init(&config_path);
    assert!(result.is_ok(), "init should succeed: {:?}", result.err());
    assert!(config_path.exists(), "config file should be created");

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[defaults]"), "should contain [defaults] section");
    assert!(content.contains("row_limit"), "should contain row_limit");
    assert!(content.contains("timeout"), "should contain timeout");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_init_creates_directory_tree() {
    let dir = std::env::temp_dir().join("dbtoon-init-test-mkdir/deep/nested");
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("dbtoon-init-test-mkdir"));
    let config_path = dir.join("config.toml");

    let result = init::run_init(&config_path);
    assert!(result.is_ok(), "init should create directories: {:?}", result.err());
    assert!(config_path.exists());

    std::fs::remove_dir_all(std::env::temp_dir().join("dbtoon-init-test-mkdir")).ok();
}

#[test]
fn test_init_default_template_has_commented_profiles() {
    let _guard = EnvGuard::new(&[]);
    // Ensure no Databricks env vars are set
    unsafe {
        std::env::remove_var("DATABRICKS_HOST");
        std::env::remove_var("DATABRICKS_TOKEN");
    }

    let dir = std::env::temp_dir().join("dbtoon-init-test-commented");
    let _ = std::fs::remove_dir_all(&dir);
    let config_path = dir.join("config.toml");

    init::run_init(&config_path).unwrap();

    let content = std::fs::read_to_string(&config_path).unwrap();
    // When no Databricks env vars are set, profiles should be commented out
    assert!(content.contains("# [profiles."), "profiles should be commented out");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_init_databricks_env_detected_uncomments_profile() {
    let _guard = EnvGuard::new(&[
        ("DATABRICKS_HOST", "https://ws.azuredatabricks.net"),
        ("DATABRICKS_TOKEN", "dapi-test-token"),
    ]);

    let dir = std::env::temp_dir().join("dbtoon-init-test-dbx");
    let _ = std::fs::remove_dir_all(&dir);
    let config_path = dir.join("config.toml");

    init::run_init(&config_path).unwrap();

    let content = std::fs::read_to_string(&config_path).unwrap();
    // When Databricks env vars are detected, the Databricks profile should be uncommented
    assert!(
        content.contains("[profiles.databricks]") || content.contains("[profiles.databricks_example]"),
        "Databricks profile should be uncommented, got:\n{}",
        content
    );
    assert!(content.contains("$DATABRICKS_HOST"), "should reference $DATABRICKS_HOST");

    std::fs::remove_dir_all(&dir).ok();
}

// =====================================================================
// T014: Init when config already exists
// =====================================================================

#[test]
fn test_init_already_exists_does_not_overwrite() {
    let dir = std::env::temp_dir().join("dbtoon-init-test-exists");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.toml");

    // Write an existing config
    let original_content = "# existing config\n[defaults]\nrow_limit = 999\n";
    std::fs::write(&config_path, original_content).unwrap();

    let result = init::run_init(&config_path);
    assert!(result.is_err(), "init should error when config exists");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("already exists"), "Error should mention 'already exists', got: {}", err);

    // Verify file was NOT overwritten
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(content, original_content, "file should not be overwritten");

    std::fs::remove_dir_all(&dir).ok();
}

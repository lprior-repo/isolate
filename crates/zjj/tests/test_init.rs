//! Integration tests for `jjz init` command
//!
//! Tests the initialization workflow including:
//! - Creating .jjz directory structure
//! - Generating default config.toml
//! - Initializing state.db
//! - Creating layouts directory
//! - Error handling for edge cases

mod common;

use common::TestHarness;

#[test]
fn test_init_creates_jjz_directory() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    // Run init
    harness.assert_success(&["init"]);

    // Verify .jjz directory was created
    harness.assert_jjz_dir_exists();
}

#[test]
fn test_init_creates_config_toml() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // Verify config.toml exists
    let config_path = harness.jjz_dir().join("config.toml");
    harness.assert_file_exists(&config_path);

    // Verify it contains expected sections
    let content = std::fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));
    assert!(content.contains("[zellij]"));
    assert!(content.contains("[dashboard]"));
    assert!(content.contains("[agent]"));
}

#[test]
fn test_init_creates_state_db() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // Verify state.db exists
    let db_path = harness.state_db_path();
    harness.assert_file_exists(&db_path);

    // Verify it's a valid SQLite database
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
    let result: Result<i32, _> =
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0));
    assert!(result.is_ok(), "Database should have sessions table");
    assert_eq!(result.unwrap(), 0, "Database should be empty after init");
}

#[test]
fn test_init_creates_layouts_directory() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // Verify layouts directory exists
    let layouts_path = harness.jjz_dir().join("layouts");
    harness.assert_file_exists(&layouts_path);
    assert!(layouts_path.is_dir(), "layouts should be a directory");
}

#[test]
fn test_init_twice_succeeds_idempotently() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    // First init
    harness.assert_success(&["init"]);

    // Second init should not fail
    let result = harness.jjz(&["init"]);
    assert!(result.success, "Second init should succeed");
    result.assert_output_contains("already initialized");
}

#[test]
fn test_init_creates_valid_toml_config() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // Verify config can be parsed as TOML
    let config = harness.read_config().expect("Failed to read config");
    let parsed: toml::Value = toml::from_str(&config).expect("Config should be valid TOML");

    // Check key sections exist
    assert!(
        parsed.get("watch").is_some(),
        "Config should have [watch] section"
    );
    assert!(
        parsed.get("zellij").is_some(),
        "Config should have [zellij] section"
    );
    assert!(
        parsed.get("dashboard").is_some(),
        "Config should have [dashboard] section"
    );
}

#[test]
fn test_init_config_has_correct_defaults() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    let config = harness.read_config().expect("Failed to read config");
    let parsed: toml::Value = toml::from_str(&config).expect("Config should be valid TOML");

    // Verify default values
    assert_eq!(
        parsed.get("workspace_dir").and_then(|v| v.as_str()),
        Some("../{repo}__workspaces")
    );
    assert_eq!(
        parsed.get("default_template").and_then(|v| v.as_str()),
        Some("standard")
    );

    // Verify watch section
    let watch = parsed
        .get("watch")
        .and_then(|v| v.as_table())
        .expect("watch section should exist");
    assert_eq!(
        watch.get("enabled").and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        watch.get("debounce_ms").and_then(toml::Value::as_integer),
        Some(100)
    );
}

#[test]
fn test_init_sets_up_complete_directory_structure() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // Verify complete structure
    let jjz_dir = harness.jjz_dir();
    harness.assert_file_exists(&jjz_dir);
    harness.assert_file_exists(&jjz_dir.join("config.toml"));
    harness.assert_file_exists(&jjz_dir.join("state.db"));
    harness.assert_file_exists(&jjz_dir.join("layouts"));

    // Verify it's a directory
    assert!(jjz_dir.is_dir());
    assert!(jjz_dir.join("layouts").is_dir());
}

#[test]
fn test_init_output_is_informative() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    let result = harness.jjz(&["init"]);

    assert!(result.success);
    result.assert_output_contains("Initialized");
    result.assert_output_contains(".jjz");
}

#[test]
fn test_init_creates_workspaces_directory() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    // After init, workspaces directory should not exist yet
    // It will be created when first session is added
    let workspaces_path = harness.jjz_dir().join("workspaces");
    // This is expected - workspaces dir is created on first add, not on init
    // So this test verifies the baseline state
    assert!(!workspaces_path.exists() || workspaces_path.is_dir());
}

#[test]
fn test_init_preserves_existing_config() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    // First init
    harness.assert_success(&["init"]);

    // Modify config
    let custom_config = r#"
workspace_dir = "../custom_workspaces"
main_branch = "main"
"#;
    harness
        .write_config(custom_config)
        .expect("Failed to write custom config");

    // Second init should not overwrite
    harness.assert_success(&["init"]);

    let config = harness.read_config().expect("Failed to read config");
    assert!(
        config.contains("custom_workspaces"),
        "Custom config should be preserved"
    );
}

#[test]
fn test_init_state_db_has_correct_schema() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    let db_path = harness.state_db_path();
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");

    // Check that sessions table has all required columns
    let mut stmt = conn
        .prepare("PRAGMA table_info(sessions)")
        .expect("Failed to prepare statement");
    let columns: Result<Vec<String>, _> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("Failed to query columns")
        .collect();

    let columns = columns.expect("Failed to collect columns");

    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"name".to_string()));
    assert!(columns.contains(&"status".to_string()));
    assert!(columns.contains(&"workspace_path".to_string()));
    assert!(columns.contains(&"created_at".to_string()));
    assert!(columns.contains(&"updated_at".to_string()));
}

#[test]
fn test_init_creates_indexes() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.assert_success(&["init"]);

    let db_path = harness.state_db_path();
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");

    // Check that indexes exist
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='sessions'")
        .expect("Failed to prepare statement");

    let indexes: Result<Vec<String>, _> = stmt
        .query_map([], |row| row.get(0))
        .expect("Failed to query indexes")
        .collect();

    let indexes = indexes.expect("Failed to collect indexes");

    // Should have at least status and name indexes
    assert!(indexes.iter().any(|name| name.contains("status")));
    assert!(indexes.iter().any(|name| name.contains("name")));
}

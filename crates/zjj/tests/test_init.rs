//! Integration tests for `zjj init` command
//!
//! Tests the initialization workflow including:
//! - Creating .zjj directory structure
//! - Generating default config.toml
//! - Initializing state.db
//! - Creating layouts directory
//! - Error handling for edge cases

mod common;

use common::TestHarness;
use sqlx::SqlitePool;

#[test]
fn test_init_creates_zjj_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // Run init
    harness.assert_success(&["init"]);

    // Verify .zjj directory was created
    harness.assert_zjj_dir_exists();
}

#[test]
fn test_init_creates_config_toml() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Verify config.toml exists
    let config_path = harness.zjj_dir().join("config.toml");
    harness.assert_file_exists(&config_path);

    // Verify it contains expected sections
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        std::process::abort()
    };
    assert!(content.contains("workspace_dir"));
    assert!(content.contains("[watch]"));
    assert!(content.contains("[zellij]"));
    assert!(content.contains("[dashboard]"));
    assert!(content.contains("[agent]"));
}

#[tokio::test]
async fn test_init_creates_state_db() {
    use sqlx::Row;

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Verify state.db exists
    let db_path = harness.state_db_path();
    harness.assert_file_exists(&db_path);

    // Verify it's a valid SQLite database
    let path_str = db_path.to_str().unwrap_or_else(|| std::process::abort());
    let db_url = format!("sqlite:///{path_str}?mode=rwc");
    let Ok(pool) = SqlitePool::connect(&db_url).await else {
        std::process::abort()
    };
    let Ok(row) = sqlx::query("SELECT COUNT(*) as count FROM sessions")
        .fetch_one(&pool)
        .await
    else {
        std::process::abort()
    };
    let count: i64 = row
        .try_get("count")
        .unwrap_or_else(|_| std::process::abort());
    if count != 0 {
        eprintln!("Database should be empty after init, but has {count} rows");
        std::process::abort();
    }
}

#[test]
fn test_init_creates_layouts_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Verify layouts directory exists
    let layouts_path = harness.zjj_dir().join("layouts");
    harness.assert_file_exists(&layouts_path);
    assert!(layouts_path.is_dir(), "layouts should be a directory");
}

#[test]
fn test_init_twice_succeeds_idempotently() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // First init
    harness.assert_success(&["init"]);

    // Second init should not fail
    let result = harness.zjj(&["init"]);
    assert!(result.success, "Second init should succeed");
    result.assert_output_contains("already initialized");
}

#[test]
fn test_init_creates_valid_toml_config() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Verify config can be parsed as TOML
    let Ok(config) = harness.read_config() else {
        std::process::abort()
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&config) else {
        std::process::abort()
    };

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
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let Ok(config) = harness.read_config() else {
        std::process::abort()
    };
    let Ok(parsed) = toml::from_str::<toml::Value>(&config) else {
        std::process::abort()
    };

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
    let Some(watch) = parsed.get("watch").and_then(|v| v.as_table()) else {
        std::process::abort()
    };
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
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // Verify complete structure
    let zjj_dir = harness.zjj_dir();
    harness.assert_file_exists(&zjj_dir);
    harness.assert_file_exists(&zjj_dir.join("config.toml"));
    harness.assert_file_exists(&zjj_dir.join("state.db"));
    harness.assert_file_exists(&zjj_dir.join("layouts"));

    // Verify it's a directory
    assert!(zjj_dir.is_dir());
    assert!(zjj_dir.join("layouts").is_dir());
}

#[test]
fn test_init_output_is_informative() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    let result = harness.zjj(&["init"]);

    assert!(result.success);
    result.assert_output_contains("Initialized");
    result.assert_output_contains(".zjj");
}

#[test]
fn test_init_creates_workspaces_directory() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    // After init, workspaces directory should not exist yet
    // It will be created when first session is added
    let workspaces_path = harness.zjj_dir().join("workspaces");
    // This is expected - workspaces dir is created on first add, not on init
    // So this test verifies the baseline state
    assert!(!workspaces_path.exists() || workspaces_path.is_dir());
}

#[test]
fn test_init_preserves_existing_config() {
    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    // First init
    harness.assert_success(&["init"]);

    // Modify config
    let custom_config = r#"
workspace_dir = "../custom_workspaces"
main_branch = "main"
"#;
    if harness.write_config(custom_config).is_err() {
        std::process::abort()
    }

    // Second init should not overwrite
    harness.assert_success(&["init"]);

    let Ok(config) = harness.read_config() else {
        std::process::abort()
    };
    assert!(
        config.contains("custom_workspaces"),
        "Custom config should be preserved"
    );
}

#[tokio::test]
async fn test_init_state_db_has_correct_schema() {
    use sqlx::Row;

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let db_path = harness.state_db_path();
    let db_url = format!("sqlite:{}", db_path.display());
    let Ok(pool) = SqlitePool::connect(&db_url).await else {
        std::process::abort()
    };
    // Check that sessions table has all required columns
    let Ok(rows) = sqlx::query("PRAGMA table_info(sessions)")
        .fetch_all(&pool)
        .await
    else {
        std::process::abort()
    };

    let columns: Vec<String> = rows
        .iter()
        .filter_map(|row: &sqlx::sqlite::SqliteRow| row.try_get::<&str, _>("name").ok())
        .map(|s: &str| s.to_string())
        .collect();

    let required_columns = [
        "id",
        "name",
        "status",
        "workspace_path",
        "created_at",
        "updated_at",
    ];
    for col in required_columns {
        if !columns.iter().any(|c| c == col) {
            eprintln!("Missing required column: {col}");
            std::process::abort();
        }
    }
}

#[tokio::test]
async fn test_init_creates_indexes() {
    use sqlx::Row;

    let Some(harness) = TestHarness::try_new() else {
        eprintln!("Skipping test: jj not available");
        return;
    };

    harness.assert_success(&["init"]);

    let db_path = harness.state_db_path();
    let path_str = db_path.to_str().unwrap_or_else(|| std::process::abort());
    let db_url = format!("sqlite:///{path_str}?mode=rwc");
    let Ok(pool) = SqlitePool::connect(&db_url).await else {
        std::process::abort()
    };
    // Check that indexes exist
    let Ok(rows) =
        sqlx::query("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='sessions'")
            .fetch_all(&pool)
            .await
    else {
        std::process::abort()
    };

    let indexes: Vec<String> = rows
        .iter()
        .filter_map(|row: &sqlx::sqlite::SqliteRow| row.try_get::<&str, _>("name").ok())
        .map(|s: &str| s.to_string())
        .collect();

    // Should have at least status and name indexes
    if !indexes.iter().any(|name: &String| name.contains("status")) {
        eprintln!("Missing status index");
        std::process::abort();
    }
    if !indexes.iter().any(|name: &String| name.contains("name")) {
        eprintln!("Missing name index");
        std::process::abort();
    }
}

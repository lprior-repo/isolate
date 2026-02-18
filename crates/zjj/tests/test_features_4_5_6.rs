//! Integration tests for Features 4, 5, and 6
//!
//! - Feature 4: Checkpoint 100MiB limit with metadata-only fallback
//! - Feature 5: WAL recovery signaling with backup suggestion
//! - Feature 6: Recovery + integrity validation workflow integration

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;

use common::TestHarness;
use rand::RngCore;

#[test]
fn test_checkpoint_size_limit_compressible_data() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    // Initialize zjj
    let init_result = harness.zjj(&["init"]);
    assert!(
        init_result.success,
        "zjj init failed: {}",
        init_result.stderr
    );

    // Create session
    let add_result = harness.zjj(&["add", "test-session", "--no-zellij"]);
    assert!(add_result.success, "zjj add failed: {}", add_result.stderr);

    // Create compressible 150MB file (zeros)
    let workspace_path = harness.repo_path.join("workspaces").join("test-session");
    let large_file = workspace_path.join("large-zeros.bin");
    let zeros = vec![0u8; 150 * 1024 * 1024];
    fs::write(&large_file, zeros).expect("Failed to write test file");

    // Create checkpoint - should succeed because it compresses well
    let checkpoint_result = harness.zjj(&[
        "checkpoint",
        "create",
        "--description",
        "compressible test",
        "--json",
    ]);
    assert!(
        checkpoint_result.success,
        "Checkpoint creation failed: {}",
        checkpoint_result.stderr
    );

    let json: serde_json::Value =
        serde_json::from_str(&checkpoint_result.stdout).expect("Invalid JSON");

    assert_eq!(json["type"], "Created");
    assert!(json["checkpoint_id"].is_string());
    assert_eq!(
        json["metadata_only"].as_array().unwrap().len(),
        0,
        "Should not be metadata-only for compressible data"
    );
}

#[test]
fn test_checkpoint_size_limit_uncompressible_data() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    let init_result = harness.zjj(&["init"]);
    assert!(
        init_result.success,
        "zjj init failed: {}",
        init_result.stderr
    );

    let add_result = harness.zjj(&["add", "test-session", "--no-zellij"]);
    assert!(add_result.success, "zjj add failed: {}", add_result.stderr);

    // Create uncompressible 150MB file (random data)
    let workspace_path = harness.repo_path.join("workspaces").join("test-session");
    let large_file = workspace_path.join("large-random.bin");

    // Generate random data
    let mut rng = rand::thread_rng();
    let mut random_data = vec![0u8; 150 * 1024 * 1024];
    rng.fill_bytes(&mut random_data);
    fs::write(&large_file, random_data).expect("Failed to write test file");

    // Create checkpoint - should fall back to metadata-only
    let checkpoint_result = harness.zjj(&[
        "checkpoint",
        "create",
        "--description",
        "uncompressible test",
        "--json",
    ]);
    assert!(
        checkpoint_result.success,
        "Checkpoint creation failed: {}",
        checkpoint_result.stderr
    );

    let json: serde_json::Value =
        serde_json::from_str(&checkpoint_result.stdout).expect("Invalid JSON");

    assert_eq!(json["type"], "Created");
    assert!(json["checkpoint_id"].is_string());
    let metadata_only = json["metadata_only"]
        .as_array()
        .expect("metadata_only should be array");
    assert_eq!(
        metadata_only.len(),
        1,
        "Should have exactly one metadata-only session"
    );
    assert_eq!(metadata_only[0], "test-session");
}

#[test]
fn test_checkpoint_create_json_output() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);

    let result = harness.zjj(&[
        "checkpoint",
        "create",
        "--description",
        "json test",
        "--json",
    ]);
    assert!(result.success, "Command failed: {}", result.stderr);

    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    assert_eq!(json["type"], "Created");
    assert!(json["checkpoint_id"].is_string());
    assert!(json["metadata_only"].is_array());
}

#[test]
fn test_checkpoint_list_json_output() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);
    harness.zjj(&["checkpoint", "create", "--description", "test1"]);
    harness.zjj(&["checkpoint", "create", "--description", "test2"]);

    let result = harness.zjj(&["checkpoint", "list", "--json"]);
    assert!(result.success, "Command failed: {}", result.stderr);

    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    assert_eq!(json["type"], "List");
    let checkpoints = json["checkpoints"]
        .as_array()
        .expect("checkpoints should be array");
    assert!(checkpoints.len() >= 2, "Should have at least 2 checkpoints");

    // Verify checkpoint structure
    let first = &checkpoints[0];
    assert!(first["id"].is_string());
    assert!(first["created_at"].is_string());
    assert!(first["session_count"].is_number());
    assert!(first["description"].is_string());
}

#[test]
fn test_checkpoint_on_success_hook() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);

    let result = harness.zjj(&[
        "checkpoint",
        "create",
        "--description",
        "hook test",
        "--on-success",
        "echo HOOK_EXECUTED",
    ]);

    assert!(result.success, "Command failed: {}", result.stderr);
    assert!(
        result.stdout.contains("HOOK_EXECUTED") || result.stderr.contains("HOOK_EXECUTED"),
        "On-success hook should execute"
    );
}

#[test]
fn test_doctor_json_output() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);

    let result = harness.zjj(&["doctor", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    assert!(json["checks"].is_array());
    assert!(json["summary"].is_object());
    assert!(json["summary"]["passed"].is_number());
    assert!(json["summary"]["warnings"].is_number());
    assert!(json["summary"]["failed"].is_number());
}

#[test]
fn test_doctor_recovery_detection() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);

    // Create recovery.log with CURRENT timestamp to simulate recent recovery
    let recovery_log = harness.repo_path.join(".zjj").join("recovery.log");
    let now = chrono::Utc::now();
    let log_entry = format!(
        "[{}] Recovered from corruption: test corruption\n",
        now.to_rfc3339()
    );
    fs::write(&recovery_log, log_entry).expect("Failed to write recovery log");

    let result = harness.zjj(&["doctor", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    let checks = json["checks"].as_array().expect("checks should be array");
    let db_check = checks
        .iter()
        .find(|c| c["name"].as_str().unwrap().contains("Database"))
        .expect("Should have database check");

    assert_eq!(db_check["status"], "warn", "Should warn about recovery");
    let message = db_check["message"].as_str().unwrap();
    assert!(
        message.contains("recovered") || message.contains("Recovery"),
        "Should mention recovery"
    );

    let suggestion = db_check["suggestion"].as_str().unwrap();
    assert!(
        suggestion.contains("backup --create"),
        "Should suggest creating backup"
    );
}

#[test]
fn test_doctor_recovery_with_integrity_issues() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);

    // Create recovery.log with CURRENT timestamp
    let recovery_log = harness.repo_path.join(".zjj").join("recovery.log");
    let now = chrono::Utc::now();
    let log_entry = format!(
        "[{}] Recovered from corruption: test corruption\n",
        now.to_rfc3339()
    );
    fs::write(&recovery_log, log_entry).expect("Failed to write recovery log");

    // Remove workspace directory to create integrity issue
    let workspace_path = harness.repo_path.join("workspaces").join("test-session");
    fs::remove_dir_all(&workspace_path).ok();

    let result = harness.zjj(&["doctor", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    let checks = json["checks"].as_array().expect("checks should be array");
    let db_check = checks
        .iter()
        .find(|c| c["name"].as_str().unwrap().contains("Database"))
        .expect("Should have database check");

    assert_eq!(db_check["status"], "warn");

    // Check for combined recovery + integrity detection
    let details = &db_check["details"];
    assert!(
        details["recovered"].as_bool().unwrap_or(false),
        "Should detect recovery"
    );

    if let Some(integrity_issues) = details.get("integrity_issues") {
        assert!(
            integrity_issues["invalid_workspaces"].as_u64().unwrap() > 0,
            "Should detect integrity issues"
        );
    }
}

#[test]
fn test_integrity_validate_json_output() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);

    let result = harness.zjj(&["integrity", "validate", "test-session", "--json"]);
    assert!(result.success, "Command failed: {}", result.stderr);

    let json: serde_json::Value = serde_json::from_str(&result.stdout).expect("Invalid JSON");

    assert!(json["is_valid"].is_boolean());
    assert!(json["workspace"].is_string());
    assert!(json["validation"].is_object());
}

#[test]
fn test_integrity_repair_help_has_rebind_flag() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    let result = harness.zjj(&["integrity", "repair", "--help"]);

    assert!(
        result.stdout.contains("--rebind"),
        "Help should document --rebind flag"
    );
    assert!(
        result
            .stdout
            .contains("Update session record when workspace is detected in a new location"),
        "Should explain --rebind functionality"
    );
}

#[test]
fn test_integrity_repair_rebind_flag_parses() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);

    // Try to use --rebind flag (may fail but should parse the flag)
    let result = harness.zjj(&[
        "integrity",
        "repair",
        "test-session",
        "--rebind",
        "--force",
        "--json",
    ]);

    // Should not fail with "unexpected argument" error
    let combined_output = format!("{}{}", result.stdout, result.stderr);
    assert!(
        !combined_output.contains("unexpected argument '--rebind'"),
        "Should accept --rebind flag without parse error"
    );
}

#[test]
fn test_doctor_on_failure_hook() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);

    // Create a failing condition - remove .jj directory
    let jj_dir = harness.repo_path.join(".jj");
    fs::remove_dir_all(&jj_dir).ok();

    let result = harness.zjj(&["doctor", "--on-failure", "echo FAILURE_HOOK_EXECUTED"]);

    let combined_output = format!("{}{}", result.stdout, result.stderr);
    assert!(
        combined_output.contains("FAILURE_HOOK_EXECUTED"),
        "On-failure hook should execute when checks fail"
    );
}

#[test]
fn test_checkpoint_list_human_format() {
    let harness = TestHarness::new().expect("Failed to create test harness");

    harness.zjj(&["init"]);
    harness.zjj(&["add", "test-session", "--no-zellij"]);
    harness.zjj(&["checkpoint", "create", "--description", "test checkpoint"]);

    let result = harness.zjj(&["checkpoint", "list"]);
    assert!(result.success, "Command failed: {}", result.stderr);

    assert!(result.stdout.contains("ID"), "Should have ID column header");
    assert!(
        result.stdout.contains("Created"),
        "Should have Created column header"
    );
    assert!(
        result.stdout.contains("Sessions"),
        "Should have Sessions column header"
    );
    assert!(
        result.stdout.contains("Description"),
        "Should have Description column header"
    );
    assert!(
        result.stdout.contains("test checkpoint"),
        "Should show checkpoint description"
    );
}

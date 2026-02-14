mod common;

use common::TestHarness;

#[tokio::test]
async fn test_dashboard_json_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Test dashboard --json (should output JSON, not fail with TUI error)
    // Note: It might fail with "No sessions found" or return empty list, but it shouldn't try to
    // launch TUI
    let result = harness.zjj(&["dashboard", "--json"]);
    if !result.success {
        println!("Dashboard stdout: {}", result.stdout);
        println!("Dashboard stderr: {}", result.stderr);
    }
    assert!(result.success, "dashboard --json should succeed");

    // It should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&result.stdout).expect("output should be valid JSON");
    assert!(
        parsed.is_array() || parsed.is_object(),
        "output should be JSON array or object"
    );
}

#[tokio::test]
async fn test_switch_no_zellij_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // Create a dummy session so we have something to switch to
    harness.assert_success(&["add", "test-session", "--no-open", "--no-hooks"]);

    // Test switch --no-zellij
    // This should succeed even if we are not in zellij (TestHarness is not in zellij)
    let result = harness.zjj(&["switch", "test-session", "--no-zellij"]);
    assert!(result.success, "switch --no-zellij should succeed");
    assert!(
        result.stdout.contains("Switched to: test-session"),
        "Should output success message"
    );
}

#[tokio::test]
async fn test_spawn_idempotent_flag() {
    let Some(harness) = TestHarness::try_new() else {
        return;
    };

    harness.assert_success(&["init"]);

    // We need a mock bead for spawn to work
    // Since we don't have easy access to internal bead db, we rely on spawn failing validation
    // or we can try to mock the bead if possible.
    // However, `spawn` checks if we are on main branch.

    // Let's test checking if the argument is accepted at least.
    // If we pass an invalid bead, it should fail with "Bead not found", but not "unexpected
    // argument"

    let result = harness.zjj(&["spawn", "nonexistent-bead", "--idempotent"]);
    // It should fail because bead doesn't exist, but NOT because of the flag
    assert!(!result.success);
    assert!(!result.stderr.contains("unexpected argument"));
    assert!(
        result.stderr.contains("not found")
            || result.stderr.contains("Bead")
            || result.stderr.contains("failed")
    );

    // To test true idempotency we'd need a full setup.
    // For now, verifying the flag is accepted is a good first step.
}

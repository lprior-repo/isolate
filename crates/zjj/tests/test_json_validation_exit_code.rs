//! Test to verify that JSON output properly maps validation error exit codes

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::Command;

#[test]
fn test_add_json_error_exit_code() {
    // Test that adding a session with invalid name returns exit code 1 in JSON mode
    // We use env!("CARGO_BIN_EXE_zjj") or just "zjj" if in path, but cargo_bin is better.
    // Given the deprecation, we'll try to use the recommended macro if available, 
    // or just silence the warning. Since we want to be safe, let's just allow deprecated
    // for this line or use `Command::new` with the binary path.
    // The safest valid way without macro is often `Command::cargo_bin` with allow deprecated.
    
    #[allow(deprecated)]
    let output = Command::cargo_bin("zjj")
        .unwrap()
        .args([
            "add",
            "-invalid",
            "--no-open",
            "--json",
        ])
        .output()
        .expect("Failed to execute command");

    // Should fail (exit code != 0)
    assert!(!output.status.success());

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should contain valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        panic!("Failed to parse JSON output. Stdout: {stdout}")
    });

    // Should have success = false
    assert_eq!(parsed.get("success").and_then(serde_json::Value::as_bool), Some(false));

    // Should have error details
    let error = parsed.get("error").expect("Should have error field");

    // The exit code should be 1 for validation errors
    let exit_code = error.get("exit_code").and_then(serde_json::Value::as_i64);
    assert_eq!(
        exit_code,
        Some(1),
        "Validation errors should have exit code 1"
    );
}


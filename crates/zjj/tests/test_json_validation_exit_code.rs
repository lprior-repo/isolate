//! Test to verify that JSON output properly maps validation error exit codes

use std::process::Command;

#[test]
fn test_add_json_error_exit_code() {
    // Test that adding a session with invalid name returns exit code 1 in JSON mode
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "zjj",
            "--",
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
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect(&format!(
        "Failed to parse JSON output. Stdout: {}, Stderr: {}",
        stdout, stderr
    ));

    // Should have success = false
    assert_eq!(parsed.get("success").and_then(|v| v.as_bool()), Some(false));

    // Should have error details
    let error = parsed.get("error").expect("Should have error field");

    // The exit code should be 1 for validation errors
    let exit_code = error.get("exit_code").and_then(|v| v.as_i64());
    assert_eq!(
        exit_code,
        Some(1),
        "Validation errors should have exit code 1"
    );

    println!("Test passed: JSON validation error correctly reports exit code 1");
}

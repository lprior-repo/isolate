//! Test to verify that JSON output properly maps validation error exit codes

// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

use anyhow::Result;

#[test]
fn test_add_json_error_exit_code() -> Result<()> {
    // Test that adding a session with invalid name returns exit code 1 in JSON mode
    let output = assert_cmd::cargo::cargo_bin_cmd!("zjj")
        .args(["add", "-invalid", "--no-open", "--json"])
        .output()?;

    // Should fail (exit code != 0)
    assert!(!output.status.success());

    // Should output JSON to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        anyhow::anyhow!("Failed to parse JSON output. Stdout: {stdout}. Error: {e}")
    })?;

    // Should have success = false
    assert_eq!(
        parsed.get("success").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert!(
        parsed.get("$schema").is_some(),
        "Should have '$schema' field"
    );
    assert_eq!(
        parsed
            .get("schema_type")
            .and_then(serde_json::Value::as_str),
        Some("single")
    );

    // Should have error details
    let error = parsed
        .get("error")
        .ok_or_else(|| anyhow::anyhow!("Should have error field"))?;

    // The exit code should be 1 for validation errors
    let exit_code = error.get("exit_code").and_then(serde_json::Value::as_i64);
    assert_eq!(
        exit_code,
        Some(1),
        "Validation errors should have exit code 1"
    );

    Ok(())
}

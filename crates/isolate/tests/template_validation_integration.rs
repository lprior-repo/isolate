// Integration tests have relaxed clippy settings for brutal test scenarios.
// Production code (src/) must use strict zero-unwrap/panic patterns.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable,
    // Test code ergonomics
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments,
    // Format string ergonomics for tests
    clippy::uninlined_format_args,
    // Documentation relaxations for test-only code
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // Pattern matching relaxations
    clippy::manual_let_else,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::ignored_unit_patterns,
)]
//! KDL Validation Integration Tests for Template Creation
//!
//! This test module verifies that template creation properly validates KDL syntax.
//! Tests cover both valid and invalid KDL scenarios.

use std::io::Write;

use tempfile::TempDir;
use isolate_core::kdl_validation;

/// Helper: Create a temporary KDL file with given content
#[allow(dead_code)]
fn create_temp_kdl_file(content: &str) -> Result<(TempDir, String), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let file_path = temp_dir.path().join("test_layout.kdl");

    let mut file = std::fs::File::create(&file_path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;

    Ok((temp_dir, file_path.to_string_lossy().to_string()))
}

/// Test: Valid KDL should pass validation
#[test]
fn test_kdl_validation_accepts_valid_kdl() {
    let valid_kdl = r#"layout {
    pane {
        command "bash"
    }
}"#;

    let result = kdl_validation::validate_kdl_syntax(valid_kdl);
    assert!(
        result.is_ok(),
        "Valid KDL should pass validation. Error: {:?}",
        result.err()
    );
}

/// Test: Invalid KDL with missing closing brace should fail validation
#[test]
fn test_kdl_validation_rejects_missing_brace() {
    let invalid_kdl = r#"layout {
    pane {
        command "bash"
"#; // Missing closing braces

    let result = kdl_validation::validate_kdl_syntax(invalid_kdl);
    assert!(
        result.is_err(),
        "KDL with missing braces should fail validation, but it passed. \
         This indicates KDL validation is NOT working!"
    );

    let error_msg = match result {
        Err(e) => e.to_string(),
        Ok(()) => return,
    };
    assert!(
        error_msg.contains("KDL") || error_msg.contains("syntax"),
        "Error message should mention KDL or syntax. Got: {error_msg}"
    );
}

/// Test: Completely invalid syntax should fail validation
#[test]
fn test_kdl_validation_rejects_invalid_syntax() {
    let invalid_kdl = "this is not valid KDL at all {{{";

    let result = kdl_validation::validate_kdl_syntax(invalid_kdl);
    assert!(
        result.is_err(),
        "Completely invalid KDL should fail validation, but it passed. \
         This indicates KDL validation is NOT working!"
    );

    let error_msg = match result {
        Err(e) => e.to_string(),
        Ok(()) => return,
    };
    assert!(
        error_msg.contains("KDL") || error_msg.contains("syntax") || error_msg.contains("parse"),
        "Error message should mention KDL, syntax, or parse. Got: {error_msg}"
    );
}

/// Test: KDL without required 'layout' node should fail validation
#[test]
fn test_kdl_validation_rejects_missing_layout() {
    let invalid_kdl = r#"pane {
    command "bash"
}"#;

    let result = kdl_validation::validate_kdl_syntax(invalid_kdl);
    assert!(
        result.is_err(),
        "KDL without 'layout' node should fail validation, but it passed. \
         This indicates Zellij-specific validation is NOT working!"
    );

    let error_msg = match result {
        Err(e) => e.to_string(),
        Ok(()) => return,
    };
    assert!(
        error_msg.contains("layout"),
        "Error message should mention 'layout'. Got: {error_msg}"
    );
}

/// Test: KDL without required 'pane' node should fail validation
#[test]
fn test_kdl_validation_rejects_missing_pane() {
    let invalid_kdl = "layout { }";

    let result = kdl_validation::validate_kdl_syntax(invalid_kdl);
    assert!(
        result.is_err(),
        "KDL without 'pane' node should fail validation, but it passed. \
         This indicates Zellij-specific validation is NOT working!"
    );

    let error_msg = match result {
        Err(e) => e.to_string(),
        Ok(()) => return,
    };
    assert!(
        error_msg.contains("pane"),
        "Error message should mention 'pane'. Got: {error_msg}"
    );
}

/// Test: Complex valid KDL should pass validation
#[test]
fn test_kdl_validation_accepts_complex_kdl() {
    let complex_kdl = r#"layout {
    pane split_direction="horizontal" {
        pane {
            command "nvim"
            size "70%"
        }
        pane {
            command "bash"
            size "30%"
        }
    }
}"#;

    let result = kdl_validation::validate_kdl_syntax(complex_kdl);
    assert!(
        result.is_ok(),
        "Complex valid KDL should pass validation. Error: {:?}",
        result.err()
    );
}

/// Test: KDL with comments should pass validation
#[test]
fn test_kdl_validation_accepts_kdl_with_comments() {
    let kdl_with_comments = r#"// This is a comment
layout {
    pane {
        command "bash" // inline comment
        // Another comment
    }
}"#;

    let result = kdl_validation::validate_kdl_syntax(kdl_with_comments);
    assert!(
        result.is_ok(),
        "KDL with comments should pass validation. Error: {:?}",
        result.err()
    );
}

/// Test: KDL with floating panes should pass validation
#[test]
fn test_kdl_validation_accepts_floating_panes() {
    let kdl_floating = r#"layout {
    pane {
        command "bash"
    }
    floating_panes {
        pane {
            command "htop"
            x "10%"
            y "10%"
        }
    }
}"#;

    let result = kdl_validation::validate_kdl_syntax(kdl_floating);
    assert!(
        result.is_ok(),
        "KDL with floating panes should pass validation. Error: {:?}",
        result.err()
    );
}

/// Test: Multiple syntax errors should be caught
#[test]
fn test_kdl_validation_catches_multiple_errors() {
    // Missing closing brace
    let invalid_kdl = "layout { pane {";

    let result = kdl_validation::validate_kdl_syntax(invalid_kdl);
    assert!(result.is_err(), "Invalid KDL should fail validation");

    let error_msg = match result {
        Err(e) => e.to_string(),
        Ok(()) => return,
    };
    assert!(!error_msg.is_empty(), "Error message should not be empty");
}

/// Test: Valid KDL with properties and arguments
#[test]
fn test_kdl_validation_accepts_properties_and_arguments() {
    let kdl = r#"layout {
    pane name="main" size="80%" {
        command "nvim"
        args "--cmd" "set number"
        cwd "/home/user"
        focus true
    }
}"#;

    let result = kdl_validation::validate_kdl_syntax(kdl);
    assert!(
        result.is_ok(),
        "KDL with properties and arguments should pass validation. Error: {:?}",
        result.err()
    );
}

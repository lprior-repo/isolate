//! KDL parsing and validation
//!
//! This module provides KDL syntax validation using the kdl-rs parser.
//! It validates KDL documents and provides detailed error messages with
//! line and column numbers.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]

use std::str::FromStr;

use crate::{Error, Result};

/// Validate KDL syntax with detailed error reporting
///
/// Uses the kdl-rs parser to check syntax and provide line/column information.
///
/// # Errors
///
/// Returns `Error::ValidationError` with:
/// - Line number (1-indexed)
/// - Column number (1-indexed)
/// - Specific syntax error
///
/// # Examples
///
/// ```
/// use zjj_core::kdl_validation::validate_kdl_syntax;
///
/// let valid_kdl = "layout { pane { command \"bash\" } }";
/// assert!(validate_kdl_syntax(valid_kdl).is_ok());
///
/// let invalid_kdl = "layout { pane { mismatched brace ";
/// assert!(validate_kdl_syntax(invalid_kdl).is_err());
/// ```
pub fn validate_kdl_syntax(content: &str) -> Result<()> {
    // Parse the KDL content using kdl-rs
    let parse_result = kdl::KdlDocument::from_str(content);

    parse_result.map_err(|kdl_error| {
        // Extract error details from kdl-rs error
        let error_msg = format_kdl_error(&kdl_error, content);
        Error::ValidationError(error_msg)
    })?;

    // Additional Zellij-specific validation
    validate_zellij_requirements(content)?;

    Ok(())
}

/// Format KDL parsing error with line, column, and context
fn format_kdl_error(error: &kdl::KdlError, content: &str) -> String {
    // Get line and column from error span if available
    let (line, col) = error
        .span()
        .map(|span| {
            // Calculate line and column from byte offset
            let before_error = &content[..span.start().min(content.len())];
            let line_num = before_error.lines().count() + 1; // 1-indexed
            let last_newline = before_error.rfind('\n').map_or(0, |pos| pos + 1);
            let col_num = span.start().saturating_sub(last_newline) + 1; // 1-indexed
            (line_num, col_num)
        })
        .map_or((0, 0), |v| v);

    if line > 0 {
        format!(
            "KDL syntax error at line {line}, column {col}: {}",
            error.to_string()
        )
    } else {
        format!("KDL syntax error: {}", error.to_string())
    }
}

/// Validate Zellij-specific KDL requirements
///
/// Ensures the KDL document contains required Zellij layout structure
fn validate_zellij_requirements(content: &str) -> Result<()> {
    let doc = kdl::KdlDocument::from_str(content)
        .map_err(|e| Error::ValidationError(format!("Failed to parse KDL for validation: {e}")))?;

    // Check for root 'layout' node
    let has_layout = doc.nodes().iter().any(|node| {
        node.name()
            .and_then(|name| name.as_str())
            .map_or(false, |s| s == "layout")
    });

    if !has_layout {
        return Err(Error::ValidationError(
            "Zellij KDL must contain a 'layout' node at the root level".to_string(),
        ));
    }

    // Check for at least one 'pane' node (in children or at root)
    let has_pane = doc.nodes().iter().any(|node| {
        // Check direct children of layout node
        node.children()
            .map_or(false, |children| {
                children.nodes().iter().any(|child| {
                    child.name().and_then(|n| n.as_str()).map_or(false, |s| s == "pane")
                })
            })
            // Also check if the node itself is a pane (for simple layouts)
            || node
                .name()
                .and_then(|n| n.as_str())
                .map_or(false, |s| s == "pane")
    });

    if !has_pane {
        return Err(Error::ValidationError(
            "Zellij KDL must contain at least one 'pane' node".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test: Valid KDL document
    #[test]
    fn test_valid_kdl_document() {
        let valid_kdl = r#"layout {
    pane {
        command "bash"
    }
}"#;

        assert!(validate_kdl_syntax(valid_kdl).is_ok());
    }

    // Test: Invalid KDL - unbalanced braces
    #[test]
    fn test_invalid_kdl_unbalanced_braces() {
        let invalid_kdl = "layout { pane { ";

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());

        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("KDL syntax error") || msg.contains("unexpected"));
        }
    }

    // Test: Invalid KDL - missing layout node
    #[test]
    fn test_invalid_kdl_missing_layout() {
        let invalid_kdl = r#"pane {
    command "bash"
}"#;

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());

        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("layout"));
        }
    }

    // Test: Invalid KDL - missing pane node
    #[test]
    fn test_invalid_kdl_missing_pane() {
        let invalid_kdl = "layout { }";

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());

        if let Err(Error::ValidationError(msg)) = result {
            assert!(msg.contains("pane"));
        }
    }

    // Test: Complex valid KDL with nested panes
    #[test]
    fn test_valid_complex_kdl() {
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

        assert!(validate_kdl_syntax(complex_kdl).is_ok());
    }

    // Test: KDL with comments (should be valid)
    #[test]
    fn test_kdl_with_comments() {
        let kdl_with_comments = r#"// This is a comment
layout {
    pane {
        command "bash" // inline comment
        // Another comment
    }
}"#;

        assert!(validate_kdl_syntax(kdl_with_comments).is_ok());
    }

    // Test: KDL with invalid syntax (bad identifier)
    #[test]
    fn test_kdl_invalid_identifier() {
        let invalid_kdl = "layout { 123invalid { } }";

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());
    }

    // Test: Empty KDL document
    #[test]
    fn test_empty_kdl() {
        let result = validate_kdl_syntax("");
        assert!(result.is_err());
    }

    // Test: KDL with floating panes (Zellij-specific)
    #[test]
    fn test_kdl_with_floating_panes() {
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

        assert!(validate_kdl_syntax(kdl_floating).is_ok());
    }

    // Test: KDL with arguments and properties
    #[test]
    fn test_kdl_with_arguments_and_properties() {
        let kdl_complex = r#"layout {
    pane name="main" size="80%" {
        command "nvim"
        args "--cmd" "set number"
        cwd "/home/user"
        focus true
    }
}"#;

        assert!(validate_kdl_syntax(kdl_complex).is_ok());
    }
}

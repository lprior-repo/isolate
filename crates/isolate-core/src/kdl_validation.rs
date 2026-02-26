//! KDL parsing and validation
//!
//! This module provides KDL syntax validation using the kdl-rs parser.
//! It validates KDL documents and provides detailed error messages.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]
#![cfg_attr(not(test), deny(clippy::panic))]
#![warn(clippy::pedantic)]

use std::str::FromStr;

use crate::{Error, Result};

/// Validate KDL syntax with detailed error reporting
///
/// Uses the kdl-rs parser to check syntax and provide error information.
///
/// # Errors
///
/// Returns `Error::ValidationError` with:
/// - Specific syntax error from the parser
///
/// # Examples
///
/// ```
/// use isolate_core::kdl_validation::validate_kdl_syntax;
///
/// let valid_kdl = "layout {\n    pane {\n        command \"bash\"\n    }\n}";
/// assert!(validate_kdl_syntax(valid_kdl).is_ok());
///
/// let invalid_kdl = "layout { pane { mismatched brace ";
/// assert!(validate_kdl_syntax(invalid_kdl).is_err());
/// ```
pub fn validate_kdl_syntax(content: &str) -> Result<()> {
    // Parse the KDL content using kdl-rs
    let doc = kdl::KdlDocument::from_str(content).map_err(|kdl_error| {
        // Convert KDL error to validation error
        Error::ValidationError {
            message: format!("KDL syntax error: {kdl_error}"),
            field: None,
            value: None,
            constraints: Vec::new(),
        }
    })?;

    // Zellij-specific validation: check for 'layout' node
    let has_layout = doc
        .nodes()
        .iter()
        .any(|node| node.name().value() == "layout");

    if !has_layout {
        return Err(Error::ValidationError {
            message: "Missing required 'layout' node".to_string(),
            field: Some("layout".to_string()),
            value: None,
            constraints: vec!["Must contain at least one 'layout' node".to_string()],
        });
    }

    // Check for 'pane' or 'floating_panes' node inside 'layout'
    let layout_node = doc
        .nodes()
        .iter()
        .find(|node| node.name().value() == "layout");

    if let Some(layout) = layout_node {
        let has_pane_or_floating = layout.children().map_or(false, |children| {
            children.nodes().iter().any(|node| {
                let name = node.name().value();
                name == "pane" || name == "floating_panes"
            })
        });

        if !has_pane_or_floating {
            return Err(Error::ValidationError {
                message: "Missing required 'pane' node inside 'layout'".to_string(),
                field: Some("pane".to_string()),
                value: None,
                constraints: vec!["Layout must contain at least one 'pane' node".to_string()],
            });
        }
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

        if let Err(Error::ValidationError { message: msg, .. }) = result {
            assert!(msg.contains("KDL syntax error") || msg.contains("unexpected"));
        }
    }

    // Test: Invalid KDL - simple node without layout
    #[test]
    fn test_invalid_kdl_simple_node() {
        let invalid_kdl = r#"pane {
    command "bash"
}"#;

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());
        if let Err(Error::ValidationError { message: msg, .. }) = result {
            assert!(msg.contains("layout"));
        }
    }

    // Test: Invalid KDL - empty layout
    #[test]
    fn test_invalid_kdl_empty_layout() {
        let invalid_kdl = "layout { }";

        let result = validate_kdl_syntax(invalid_kdl);
        assert!(result.is_err());
        if let Err(Error::ValidationError { message: msg, .. }) = result {
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

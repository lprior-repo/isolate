//! Pure KDL generation logic - Functional Core with zero I/O
//!
//! This module contains pure functions for generating KDL layout strings.
//! No file I/O, no system commands, no side effects whatsoever.
//! This is the Functional Core of the Zellij module.

use super::config::{LayoutConfig, LayoutTemplate};
use crate::{Error, Result};

/// Generate KDL content for a template (pure function)
pub fn generate_template_kdl(config: &LayoutConfig, template: LayoutTemplate) -> Result<String> {
    let kdl = match template {
        LayoutTemplate::Minimal => generate_minimal_kdl(config),
        LayoutTemplate::Standard => generate_standard_kdl(config),
        LayoutTemplate::Full => generate_full_kdl(config),
        LayoutTemplate::Split => generate_split_kdl(config),
        LayoutTemplate::Review => generate_review_kdl(config),
    };

    // Validate KDL syntax
    validate_kdl(&kdl)?;

    Ok(kdl)
}

/// Generate minimal template: single Claude pane (pure)
pub fn generate_minimal_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let cmd = &config.claude_command;

    format!(
        r#"layout {{
    pane {{
        command "{cmd}"
        cwd "{cwd}"
        focus true
    }}
}}
"#
    )
}

/// Generate standard template: 70% Claude + 30% sidebar (beads 15% + status 15%) (pure)
pub fn generate_standard_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            focus true
            size "70%"
        }}
        pane split_direction="vertical" {{
            pane {{
                command "{beads_cmd}"
                cwd "{cwd}"
                size "50%"
            }}
            pane {{
                command "jj"
                args "log" "--limit" "20"
                cwd "{cwd}"
                size "50%"
            }}
        }}
    }}
}}
"#
    )
}

/// Generate full template: standard + floating pane (pure)
pub fn generate_full_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            focus true
            size "70%"
        }}
        pane split_direction="vertical" {{
            pane {{
                command "{beads_cmd}"
                cwd "{cwd}"
                size "50%"
            }}
            pane {{
                command "jj"
                args "log" "--limit" "20"
                cwd "{cwd}"
                size "50%"
            }}
        }}
    }}
    floating_panes {{
        pane {{
            command "jj"
            args "status"
            cwd "{cwd}"
            x "20%"
            y "20%"
            width "60%"
            height "60%"
        }}
    }}
}}
"#
    )
}

/// Generate split template: two Claude instances side-by-side (pure)
pub fn generate_split_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let cmd = &config.claude_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "{cmd}"
            cwd "{cwd}"
            focus true
            size "50%"
        }}
        pane {{
            command "{cmd}"
            cwd "{cwd}"
            size "50%"
        }}
    }}
}}
"#
    )
}

/// Generate review template: diff view + beads + Claude (pure)
pub fn generate_review_kdl(config: &LayoutConfig) -> String {
    let cwd = config.workspace_path.display();
    let claude_cmd = &config.claude_command;
    let beads_cmd = &config.beads_command;

    format!(
        r#"layout {{
    pane split_direction="horizontal" {{
        pane {{
            command "jj"
            args "diff"
            cwd "{cwd}"
            focus true
            size "50%"
        }}
        pane {{
            command "{beads_cmd}"
            cwd "{cwd}"
            size "25%"
        }}
        pane {{
            command "{claude_cmd}"
            cwd "{cwd}"
            size "25%"
        }}
    }}
}}
"#
    )
}

/// Validate KDL syntax (pure)
///
/// Basic validation to ensure well-formed KDL:
/// - Balanced braces
/// - No empty node names
pub fn validate_kdl(content: &str) -> Result<()> {
    // Check balanced braces
    let open_braces = content.chars().filter(|c| *c == '{').count();
    let close_braces = content.chars().filter(|c| *c == '}').count();

    if open_braces != close_braces {
        return Err(Error::validation_error(format!(
            "Unbalanced braces: {open_braces} open, {close_braces} close"
        )));
    }

    // Check for basic structure
    if !content.contains("layout") {
        return Err(Error::validation_error(
            "KDL must contain 'layout' node".to_string(),
        ));
    }

    if !content.contains("pane") {
        return Err(Error::validation_error(
            "KDL must contain at least one 'pane' node".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn test_config() -> LayoutConfig {
        LayoutConfig::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        )
    }

    // Test Case 1: Generate minimal - Valid KDL with single pane
    #[test]
    fn test_generate_minimal_valid_kdl() {
        let config = test_config();
        let kdl = generate_minimal_kdl(&config);

        // Check it contains required elements
        assert!(kdl.contains("layout"));
        assert!(kdl.contains("pane"));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("/tmp/test-workspace"));
        assert!(kdl.contains("focus true"));

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 2: Generate standard - Valid KDL with 3 panes (70/15/15 split)
    #[test]
    fn test_generate_standard_valid_kdl() {
        let config = test_config();
        let kdl = generate_standard_kdl(&config);

        // Check structure
        assert!(kdl.contains("layout"));
        assert!(kdl.contains("split_direction=\"horizontal\""));
        assert!(kdl.contains("size \"70%\""));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("bv"));
        assert!(kdl.contains("jj"));

        // Count pane occurrences (should be 4: 1 container + 3 actual panes)
        let pane_count = kdl.matches("pane").count();
        assert!(pane_count >= 3);

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 3: Generate full - Valid KDL with floating pane
    #[test]
    fn test_generate_full_valid_kdl_with_floating() {
        let config = test_config();
        let kdl = generate_full_kdl(&config);

        // Check for floating pane
        assert!(kdl.contains("floating_panes"));
        assert!(kdl.contains("x \"20%\""));
        assert!(kdl.contains("y \"20%\""));
        assert!(kdl.contains("width \"60%\""));
        assert!(kdl.contains("height \"60%\""));

        // Validate KDL syntax
        assert!(validate_kdl(&kdl).is_ok());
    }

    // Additional test: Split template
    #[test]
    fn test_generate_split_template() {
        let config = test_config();
        let kdl = generate_split_kdl(&config);

        assert!(kdl.contains("split_direction=\"horizontal\""));
        assert!(kdl.contains("size \"50%\""));

        // Count claude commands (should be 2)
        let claude_count = kdl.matches("claude").count();
        assert_eq!(claude_count, 2);

        assert!(validate_kdl(&kdl).is_ok());
    }

    // Additional test: Review template
    #[test]
    fn test_generate_review_template() {
        let config = test_config();
        let kdl = generate_review_kdl(&config);

        assert!(kdl.contains("jj"));
        assert!(kdl.contains("diff"));
        assert!(kdl.contains("bv"));
        assert!(kdl.contains("claude"));
        assert!(kdl.contains("size \"50%\""));
        assert!(kdl.contains("size \"25%\""));

        assert!(validate_kdl(&kdl).is_ok());
    }

    // Test Case 9: Invalid KDL - Error with syntax details
    #[test]
    fn test_validate_kdl_unbalanced_braces() {
        let invalid_kdl = "layout { pane { ";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::validation_error(msg)) = result {
            assert!(msg.contains("Unbalanced braces"));
        }
    }

    #[test]
    fn test_validate_kdl_missing_layout() {
        let invalid_kdl = "pane { }";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::validation_error(msg)) = result {
            assert!(msg.contains("layout"));
        }
    }

    #[test]
    fn test_validate_kdl_missing_pane() {
        let invalid_kdl = "layout { }";
        let result = validate_kdl(invalid_kdl);

        assert!(result.is_err());
        if let Err(Error::validation_error(msg)) = result {
            assert!(msg.contains("pane"));
        }
    }
}

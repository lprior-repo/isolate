//! Integration tests for Zellij operations
//!
//! Tests Zellij layout generation, tab naming, and KDL validation.
//! These tests verify that the Zellij integration works correctly without
//! requiring an actual running Zellij instance.

use std::path::PathBuf;

use zjj_core::zellij::{self, LayoutConfig, LayoutTemplate, TabStatus};

// ============================================================================
// Layout Generation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_layout_generation_minimal() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    // Verify KDL structure
    assert!(kdl.contains("layout"), "KDL should contain layout node");
    assert!(kdl.contains("pane"), "KDL should contain pane node");
    assert!(kdl.contains("claude"), "KDL should contain claude command");
    assert!(
        kdl.contains("/tmp/test-workspace"),
        "KDL should contain workspace path"
    );
    assert!(
        kdl.contains("focus true"),
        "KDL should set focus on main pane"
    );

    // Verify no extra panes
    assert_eq!(
        kdl.matches("pane").count(),
        1,
        "Minimal layout should have exactly 1 pane"
    );
}

#[tokio::test]
async fn test_zellij_layout_generation_standard() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Standard)
        .expect("KDL generation should succeed");

    // Verify horizontal split
    assert!(
        kdl.contains("split_direction=\"horizontal\""),
        "Should split horizontally"
    );
    assert!(kdl.contains("size \"70%\""), "Main pane should be 70%");

    // Verify commands
    assert!(kdl.contains("claude"), "Should have claude command");
    assert!(kdl.contains("bv"), "Should have bv command for beads");
    assert!(kdl.contains("jj"), "Should have jj command for log");

    // Verify pane count (main container + 3 panes)
    let pane_count = kdl.matches("pane").count();
    assert!(
        pane_count >= 3,
        "Standard layout should have at least 3 panes"
    );
}

#[tokio::test]
async fn test_zellij_layout_generation_full() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Full)
        .expect("KDL generation should succeed");

    // Verify floating pane is present
    assert!(
        kdl.contains("floating_panes"),
        "Should have floating_panes section"
    );
    assert!(
        kdl.contains("x \"20%\""),
        "Floating pane should have x position"
    );
    assert!(
        kdl.contains("y \"20%\""),
        "Floating pane should have y position"
    );
    assert!(
        kdl.contains("width \"60%\""),
        "Floating pane should have width"
    );
    assert!(
        kdl.contains("height \"60%\""),
        "Floating pane should have height"
    );
}

#[tokio::test]
async fn test_zellij_layout_generation_split() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Split)
        .expect("KDL generation should succeed");

    // Verify two Claude instances
    let claude_count = kdl.matches("claude").count();
    assert_eq!(
        claude_count, 2,
        "Split layout should have 2 Claude instances"
    );

    // Verify equal sizing
    assert!(kdl.contains("size \"50%\""), "Panes should be 50% each");
}

#[tokio::test]
async fn test_zellij_layout_generation_review() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Review)
        .expect("KDL generation should succeed");

    // Verify review-specific commands
    assert!(kdl.contains("jj"), "Should have jj command");
    assert!(kdl.contains("diff"), "Should have diff argument");

    // Verify three panes with specific sizes
    assert!(kdl.contains("size \"50%\""), "Diff pane should be 50%");
    assert!(
        kdl.contains("size \"25%\""),
        "Other panes should be 25% each"
    );
}

// ============================================================================
// Tab Name Validation Tests
// ============================================================================

#[test]
fn test_zellij_tab_name_validation_valid_names() {
    let test_cases = vec![
        ("simple", "zjj:simple"),
        ("feature-branch", "zjj:feature-branch"),
        ("my-session-123", "zjj:my-session-123"),
        ("a", "zjj:a"), // Single character
    ];

    for (session_name, expected_tab_name) in test_cases {
        let config = LayoutConfig::new(session_name.to_string(), PathBuf::from("/tmp"));
        assert_eq!(
            config.tab_name(),
            expected_tab_name,
            "Tab name should be correct for session '{session_name}'"
        );
    }
}

#[test]
fn test_zellij_tab_name_validation_empty_name() {
    // Empty session name is technically valid but produces "zjj:" prefix
    let config = LayoutConfig::new("".to_string(), PathBuf::from("/tmp"));
    assert_eq!(
        config.tab_name(),
        "zjj:",
        "Empty session name should produce 'zjj:'"
    );
}

#[test]
fn test_zellij_tab_name_validation_very_long_name() {
    // Test with a very long name (more than 64 characters)
    let long_name = "a".repeat(100);
    let config = LayoutConfig::new(long_name.clone(), PathBuf::from("/tmp"));

    // The system should accept it (validation happens at a higher layer)
    let expected = format!("zjj:{}", long_name);
    assert_eq!(
        config.tab_name(),
        expected,
        "Very long session names should still work"
    );
}

#[test]
fn test_zellij_tab_name_validation_special_characters() {
    // Test with characters that might be problematic
    let test_cases = vec![
        ("session_with_underscore", "zjj:session_with_underscore"),
        ("session.with.dots", "zjj:session.with.dots"),
        ("session@with@at", "zjj:session@with@at"),
    ];

    for (session_name, expected_tab_name) in test_cases {
        let config = LayoutConfig::new(session_name.to_string(), PathBuf::from("/tmp"));
        assert_eq!(
            config.tab_name(),
            expected_tab_name,
            "Tab name should preserve special characters"
        );
    }
}

#[test]
fn test_zellij_tab_name_validation_custom_prefix() {
    let config = LayoutConfig::new("my-session".to_string(), PathBuf::from("/tmp"))
        .with_tab_prefix("custom".to_string());

    assert_eq!(
        config.tab_name(),
        "custom:my-session",
        "Custom prefix should be used"
    );
}

// ============================================================================
// KDL Template Validation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_minimal() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    // Verify KDL syntax validation passes
    let result = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal);
    assert!(result.is_ok(), "Generated KDL should be valid");

    // Count braces to ensure they're balanced
    let open_braces = kdl.chars().filter(|c| *c == '{').count();
    let close_braces = kdl.chars().filter(|c| *c == '}').count();
    assert_eq!(
        open_braces, close_braces,
        "KDL should have balanced braces: {} open, {} close",
        open_braces, close_braces
    );

    // Minimal template should have exactly 2 braces (layout + pane)
    assert_eq!(
        open_braces, 2,
        "Minimal template should have 2 opening braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_standard() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let result = zellij::generate_template_kdl(&config, LayoutTemplate::Standard);
    assert!(result.is_ok(), "Standard KDL should be valid");

    let kdl = result.unwrap();
    let open_braces = kdl.chars().filter(|c| *c == '{').count();
    let close_braces = kdl.chars().filter(|c| *c == '}').count();

    assert_eq!(
        open_braces, close_braces,
        "Standard KDL should have balanced braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_full() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let result = zellij::generate_template_kdl(&config, LayoutTemplate::Full);
    assert!(result.is_ok(), "Full KDL should be valid");

    let kdl = result.unwrap();
    let open_braces = kdl.chars().filter(|c| *c == '{').count();
    let close_braces = kdl.chars().filter(|c| *c == '}').count();

    assert_eq!(
        open_braces, close_braces,
        "Full KDL should have balanced braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_split() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let result = zellij::generate_template_kdl(&config, LayoutTemplate::Split);
    assert!(result.is_ok(), "Split KDL should be valid");

    let kdl = result.unwrap();
    let open_braces = kdl.chars().filter(|c| *c == '{').count();
    let close_braces = kdl.chars().filter(|c| *c == '}').count();

    assert_eq!(
        open_braces, close_braces,
        "Split KDL should have balanced braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_review() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let result = zellij::generate_template_kdl(&config, LayoutTemplate::Review);
    assert!(result.is_ok(), "Review KDL should be valid");

    let kdl = result.unwrap();
    let open_braces = kdl.chars().filter(|c| *c == '{').count();
    let close_braces = kdl.chars().filter(|c| *c == '}').count();

    assert_eq!(
        open_braces, close_braces,
        "Review KDL should have balanced braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_all_required_elements() {
    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    // Verify required KDL elements
    assert!(kdl.contains("layout"), "Must contain 'layout' node");
    assert!(
        kdl.contains("pane"),
        "Must contain at least one 'pane' node"
    );
    assert!(kdl.contains("command"), "Must contain 'command' attribute");
    assert!(kdl.contains("cwd"), "Must contain 'cwd' attribute");
}

// ============================================================================
// Workspace Path Validation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_workspace_path_in_layouts() {
    let workspace_path = PathBuf::from("/custom/workspace/path");
    let config = LayoutConfig::new("test-session".to_string(), workspace_path.clone());

    // Test all templates include the workspace path
    let templates = vec![
        LayoutTemplate::Minimal,
        LayoutTemplate::Standard,
        LayoutTemplate::Full,
        LayoutTemplate::Split,
        LayoutTemplate::Review,
    ];

    for template in templates {
        let kdl = zellij::generate_template_kdl(&config, template)
            .expect("KDL generation should succeed");

        assert!(
            kdl.contains("/custom/workspace/path"),
            "Template {:?} should contain workspace path",
            template
        );
    }
}

#[tokio::test]
async fn test_zellij_workspace_path_with_spaces() {
    // Test workspace paths with spaces (properly quoted in KDL)
    let workspace_path = PathBuf::from("/tmp/my workspace");
    let config = LayoutConfig::new("test-session".to_string(), workspace_path);

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    // The path should be in the KDL (KDL quoting handles spaces)
    assert!(
        kdl.contains("/tmp/my workspace"),
        "Should handle paths with spaces"
    );
}

#[tokio::test]
async fn test_zellij_workspace_path_with_special_chars() {
    // Test workspace paths with special characters
    let workspace_path = PathBuf::from("/tmp/work-space_123");
    let config = LayoutConfig::new("test-session".to_string(), workspace_path);

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    assert!(
        kdl.contains("/tmp/work-space_123"),
        "Should handle special characters"
    );
}

// ============================================================================
// Custom Command Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_custom_claude_command() {
    let config = LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
        .with_claude_command("custom-claude".to_string());

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Minimal)
        .expect("KDL generation should succeed");

    assert!(
        kdl.contains("custom-claude"),
        "Should use custom claude command"
    );
    assert!(
        !kdl.contains("command \"claude\""),
        "Should not contain default claude command"
    );
}

#[tokio::test]
async fn test_zellij_custom_beads_command() {
    let config = LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
        .with_beads_command("custom-bv".to_string());

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Standard)
        .expect("KDL generation should succeed");

    assert!(kdl.contains("custom-bv"), "Should use custom beads command");
    assert!(
        !kdl.contains("command \"bv\""),
        "Should not contain default bv command"
    );
}

#[tokio::test]
async fn test_zellij_both_custom_commands() {
    let config = LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
        .with_claude_command("my-claude".to_string())
        .with_beads_command("my-bv".to_string());

    let kdl = zellij::generate_template_kdl(&config, LayoutTemplate::Standard)
        .expect("KDL generation should succeed");

    assert!(
        kdl.contains("my-claude"),
        "Should use custom claude command"
    );
    assert!(kdl.contains("my-bv"), "Should use custom beads command");
}

// ============================================================================
// Layout File Generation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_layout_file_creation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    let result = zellij::layout_generate(&config, LayoutTemplate::Minimal, output_dir).await;

    assert!(result.is_ok(), "Layout generation should succeed");

    let layout = result.unwrap();
    assert!(layout.file_path.exists(), "Layout file should be created");
    assert!(
        layout.kdl_content.contains("layout"),
        "KDL content should be valid"
    );
    assert!(
        layout.file_path.ends_with("test-session.kdl"),
        "File should be named after session"
    );
}

#[tokio::test]
async fn test_zellij_layout_file_overwrite() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    let config = LayoutConfig::new(
        "test-session".to_string(),
        PathBuf::from("/tmp/test-workspace"),
    );

    // Generate layout twice
    let result1 = zellij::layout_generate(&config, LayoutTemplate::Minimal, output_dir).await;
    assert!(result1.is_ok(), "First generation should succeed");

    let result2 = zellij::layout_generate(&config, LayoutTemplate::Standard, output_dir).await;
    assert!(
        result2.is_ok(),
        "Second generation should succeed (overwrite)"
    );

    let layout = result2.unwrap();
    assert!(layout.file_path.exists(), "File should still exist");
}

// ============================================================================
// Tab Status Tests
// ============================================================================

#[test]
fn test_zellij_tab_status_display() {
    assert_eq!(TabStatus::Active.to_string(), "active");
    assert_eq!(TabStatus::Missing.to_string(), "missing");
    assert_eq!(TabStatus::Unknown.to_string(), "unknown");
}

#[test]
fn test_zellij_tab_status_equality() {
    assert_eq!(TabStatus::Active, TabStatus::Active);
    assert_ne!(TabStatus::Active, TabStatus::Missing);
    assert_ne!(TabStatus::Missing, TabStatus::Unknown);
    assert_ne!(TabStatus::Active, TabStatus::Unknown);
}

#[tokio::test]
async fn test_zellij_check_tab_exists_when_not_running() {
    // Save current ZELLIJ var
    let zellij_var = std::env::var("ZELLIJ");

    // Temporarily remove ZELLIJ environment variable
    std::env::remove_var("ZELLIJ");

    // Should return Unknown when Zellij is not running
    let status = zellij::check_tab_exists("zjj:test").await;
    assert_eq!(
        status,
        TabStatus::Unknown,
        "Should return Unknown when Zellij is not running"
    );

    // Restore ZELLIJ var if it existed
    if let Ok(val) = zellij_var {
        std::env::set_var("ZELLIJ", val);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_templates_all_pass_internal_validation() {
    // All templates should pass the internal KDL validation
    let config = LayoutConfig::new("test".to_string(), PathBuf::from("/tmp"));

    let templates = vec![
        LayoutTemplate::Minimal,
        LayoutTemplate::Standard,
        LayoutTemplate::Full,
        LayoutTemplate::Split,
        LayoutTemplate::Review,
    ];

    for template in templates {
        let result = zellij::generate_template_kdl(&config, template);
        assert!(
            result.is_ok(),
            "Template {:?} should pass validation",
            template
        );
    }
}

#[tokio::test]
async fn test_zellij_validation_happens_automatically() {
    // The generate_template_kdl function internally validates KDL
    // If validation fails, it returns an error

    let config = LayoutConfig::new("test".to_string(), PathBuf::from("/tmp"));

    // All our templates should be valid
    let templates = vec![
        LayoutTemplate::Minimal,
        LayoutTemplate::Standard,
        LayoutTemplate::Full,
        LayoutTemplate::Split,
        LayoutTemplate::Review,
    ];

    for template in templates {
        let result = zellij::generate_template_kdl(&config, template);
        assert!(
            result.is_ok(),
            "Template {:?} should generate valid KDL that passes validation",
            template
        );

        let kdl = result.unwrap();

        // Verify the KDL has the required structure
        assert!(
            kdl.contains("layout"),
            "Template {:?} should contain 'layout' node",
            template
        );
        assert!(
            kdl.contains("pane"),
            "Template {:?} should contain 'pane' node",
            template
        );
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_zellij_config_with_empty_session_name() {
    let config = LayoutConfig::new("".to_string(), PathBuf::from("/tmp"));
    assert_eq!(config.session_name, "");
    assert_eq!(config.tab_name(), "zjj:");
}

#[test]
fn test_zellij_config_builder_chain() {
    let config = LayoutConfig::new("session".to_string(), PathBuf::from("/tmp"))
        .with_claude_command("claude-v2".to_string())
        .with_beads_command("bv-v2".to_string())
        .with_tab_prefix("zjj-dev".to_string());

    assert_eq!(config.session_name, "session");
    assert_eq!(config.claude_command, "claude-v2");
    assert_eq!(config.beads_command, "bv-v2");
    assert_eq!(config.tab_prefix, "zjj-dev");
    assert_eq!(config.tab_name(), "zjj-dev:session");
}

#[tokio::test]
async fn test_zellij_all_templates_generate_valid_kdl() {
    let config = LayoutConfig::new("test".to_string(), PathBuf::from("/tmp"));

    let templates = vec![
        LayoutTemplate::Minimal,
        LayoutTemplate::Standard,
        LayoutTemplate::Full,
        LayoutTemplate::Split,
        LayoutTemplate::Review,
    ];

    for template in templates {
        let result = zellij::generate_template_kdl(&config, template);
        assert!(
            result.is_ok(),
            "Template {:?} should generate valid KDL",
            template
        );

        let kdl = result.unwrap();
        let open_braces = kdl.chars().filter(|c| *c == '{').count();
        let close_braces = kdl.chars().filter(|c| *c == '}').count();

        assert_eq!(
            open_braces, close_braces,
            "Template {:?} should have balanced braces",
            template
        );
    }
}

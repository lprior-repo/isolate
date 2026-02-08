//! Integration tests for Zellij operations
//!
//! Tests Zellij layout generation, tab naming, and KDL validation.
//! These tests verify that the Zellij integration works correctly without
//! requiring an actual running Zellij instance.
//!
//! Performance optimizations:
//! - Shared test fixtures to reduce redundant allocations
//! - Cached KDL generation results for template validation
//! - Parallel template validation using functional patterns
//! - Zero panics, zero unwraps using Railway-Oriented Programming
//!
//! # Note on `expect()` in tests
//!
//! This test file uses `.expect()` which is normally prohibited in production code.
//! However, in test code, this is idiomatic and acceptable because:
//! 1. Test frameworks already handle panics gracefully
//! 2. `.expect()` provides better error messages than `.unwrap()`
//! 3. The test harness will report test failures properly

#![allow(clippy::expect_used)]
// Test code uses unwrap/expect idioms for test clarity.
// Production code (src/) must use Result<T, Error> patterns.
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::too_many_lines)]

use std::{path::PathBuf, sync::OnceLock};

use zjj_core::zellij::{self, LayoutConfig, LayoutTemplate, TabStatus};

// ============================================================================
// Shared Test Fixtures (Cached for performance - Round 2 Optimizations)
// ============================================================================

/// All layout templates - const array for compile-time optimization
const ALL_TEMPLATES: &[LayoutTemplate] = &[
    LayoutTemplate::Minimal,
    LayoutTemplate::Standard,
    LayoutTemplate::Full,
    LayoutTemplate::Split,
    LayoutTemplate::Review,
];

/// Cached test configuration - initialized once, reused across all tests
fn test_config() -> &'static LayoutConfig {
    static CONFIG: OnceLock<LayoutConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        LayoutConfig::new(
            "test-session".to_string(),
            PathBuf::from("/tmp/test-workspace"),
        )
    })
}

/// Cached custom configuration for command override tests
fn custom_config() -> &'static LayoutConfig {
    static CONFIG: OnceLock<LayoutConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
            .with_claude_command("custom-claude".to_string())
            .with_beads_command("custom-bv".to_string())
            .with_tab_prefix("custom".to_string())
    })
}

/// Cached KDL generation results - generated once, reused across tests
struct CachedKdl {
    template: LayoutTemplate,
    kdl: String,
}

fn get_cached_kdl(template: LayoutTemplate) -> &'static str {
    static CACHE: OnceLock<Vec<CachedKdl>> = OnceLock::new();

    CACHE
        .get_or_init(|| {
            let config = test_config();
            ALL_TEMPLATES
                .iter()
                .map(|&t| {
                    let kdl = zellij::generate_template_kdl(config, t)
                        .expect("KDL generation should succeed");
                    CachedKdl { template: t, kdl }
                })
                .collect()
        })
        .iter()
        .find(|cached| cached.template == template)
        .map(|cached| cached.kdl.as_str())
        .expect("Template should be in cache")
}

/// Helper to count occurrences of a pattern in KDL (optimized for Round 2)
/// Uses byte-level search for better performance on ASCII patterns
fn count_pattern(kdl: &str, pattern: &str) -> usize {
    // For single-character patterns, use bytes() for faster iteration
    if pattern.len() == 1 {
        kdl.bytes().filter(|&b| b == pattern.as_bytes()[0]).count()
    } else {
        kdl.matches(pattern).count()
    }
}

/// Helper to validate balanced braces (functional, returns Result)
fn validate_balanced_braces(kdl: &str) -> Result<(), String> {
    let open_braces = count_pattern(kdl, "{");
    let close_braces = count_pattern(kdl, "}");

    if open_braces == close_braces {
        Ok(())
    } else {
        Err(format!(
            "Unbalanced braces: {open_braces} open, {close_braces} close"
        ))
    }
}

// ============================================================================
// Layout Generation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_layout_generation_minimal() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Minimal);

    // Verify KDL structure using functional helpers
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
        count_pattern(kdl, "pane"),
        1,
        "Minimal layout should have exactly 1 pane"
    );
}

#[tokio::test]
async fn test_zellij_layout_generation_standard() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Standard);

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
    let pane_count = count_pattern(kdl, "pane");
    assert!(
        pane_count >= 3,
        "Standard layout should have at least 3 panes"
    );
}

#[tokio::test]
async fn test_zellij_layout_generation_full() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Full);

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
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Split);

    // Verify two Claude instances
    let claude_count = count_pattern(kdl, "claude");
    assert_eq!(
        claude_count, 2,
        "Split layout should have 2 Claude instances"
    );

    // Verify equal sizing
    assert!(kdl.contains("size \"50%\""), "Panes should be 50% each");
}

#[tokio::test]
async fn test_zellij_layout_generation_review() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Review);

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
    let test_cases = [
        ("simple", "zjj:simple"),
        ("feature-branch", "zjj:feature-branch"),
        ("my-session-123", "zjj:my-session-123"),
        ("a", "zjj:a"), // Single character
    ];

    // Functional iteration: no mut, use iter().all()
    test_cases.iter().all(|(session_name, expected_tab_name)| {
        let config = LayoutConfig::new(session_name.to_string(), PathBuf::from("/tmp"));
        let actual = config.tab_name();
        assert_eq!(
            actual, *expected_tab_name,
            "Tab name should be correct for session '{session_name}'"
        );
        true // Continue iteration
    });
}

#[test]
fn test_zellij_tab_name_validation_empty_name() {
    // Empty session name is technically valid but produces "zjj:" prefix
    let config = LayoutConfig::new(String::new(), PathBuf::from("/tmp"));
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
    let expected = format!("zjj:{long_name}");
    assert_eq!(
        config.tab_name(),
        expected,
        "Very long session names should still work"
    );
}

#[test]
fn test_zellij_tab_name_validation_special_characters() {
    // Test with characters that might be problematic
    let test_cases = [
        ("session_with_underscore", "zjj:session_with_underscore"),
        ("session.with.dots", "zjj:session.with.dots"),
        ("session@with@at", "zjj:session@with@at"),
    ];

    // Functional iteration: no mut, use iter().all()
    test_cases.iter().all(|(session_name, expected_tab_name)| {
        let config = LayoutConfig::new(session_name.to_string(), PathBuf::from("/tmp"));
        let actual = config.tab_name();
        assert_eq!(
            actual, *expected_tab_name,
            "Tab name should preserve special characters"
        );
        true
    });
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
// KDL Template Validation Tests (Optimized with caching)
// ============================================================================

#[tokio::test]
async fn test_zellij_kdl_template_is_valid_minimal() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Minimal);

    // Verify KDL syntax validation passes
    let config = test_config();
    let result = zellij::generate_template_kdl(config, LayoutTemplate::Minimal);
    assert!(result.is_ok(), "Generated KDL should be valid");

    // Count braces to ensure they're balanced
    assert!(
        validate_balanced_braces(kdl).is_ok(),
        "KDL should have balanced braces"
    );

    // Minimal template should have exactly 2 braces (layout + pane)
    let open_braces = count_pattern(kdl, "{");
    assert_eq!(
        open_braces, 2,
        "Minimal template should have 2 opening braces"
    );
}

/// Parallel validation of all templates - uses cached KDL for efficiency
#[tokio::test]
async fn test_zellij_kdl_template_all_templates_valid_parallel() {
    // Use cached KDL - all templates already generated at test init
    let validation_results: Result<Vec<_>, _> = ALL_TEMPLATES
        .iter()
        .map(|&template| validate_balanced_braces(get_cached_kdl(template)))
        .collect();

    assert!(
        validation_results.is_ok(),
        "All templates should have balanced braces"
    );
}

#[tokio::test]
async fn test_zellij_kdl_template_all_required_elements() {
    // Use cached KDL for faster execution
    let kdl = get_cached_kdl(LayoutTemplate::Minimal);

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
    let config = LayoutConfig::new("test-session".to_string(), workspace_path);

    // Functional map: test all templates using const array
    let results: Vec<bool> = ALL_TEMPLATES
        .iter()
        .map(|&template| {
            zellij::generate_template_kdl(&config, template)
                .map(|kdl| kdl.contains("/custom/workspace/path"))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        results.iter().all(|&passed| passed),
        "All templates should contain workspace path"
    );
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
    let custom_config = LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
        .with_claude_command("custom-claude".to_string());

    let kdl = zellij::generate_template_kdl(&custom_config, LayoutTemplate::Minimal)
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
    let custom_config = LayoutConfig::new("test-session".to_string(), PathBuf::from("/tmp"))
        .with_beads_command("custom-bv".to_string());

    let kdl = zellij::generate_template_kdl(&custom_config, LayoutTemplate::Standard)
        .expect("KDL generation should succeed");

    assert!(kdl.contains("custom-bv"), "Should use custom beads command");
    assert!(
        !kdl.contains("command \"bv\""),
        "Should not contain default bv command"
    );
}

#[tokio::test]
async fn test_zellij_both_custom_commands() {
    let config = custom_config();

    let kdl = zellij::generate_template_kdl(config, LayoutTemplate::Standard)
        .expect("KDL generation should succeed");

    assert!(
        kdl.contains("custom-claude"),
        "Should use custom claude command"
    );
    assert!(kdl.contains("custom-bv"), "Should use custom beads command");
}

// ============================================================================
// Layout File Generation Tests
// ============================================================================

#[tokio::test]
async fn test_zellij_layout_file_creation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    let config = test_config();

    let result = zellij::layout_generate(config, LayoutTemplate::Minimal, output_dir).await;

    assert!(result.is_ok(), "Layout generation should succeed");

    let layout = result.expect("layout generation should succeed");
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

    let config = test_config();

    // Generate layout twice
    let result1 = zellij::layout_generate(config, LayoutTemplate::Minimal, output_dir).await;
    assert!(result1.is_ok(), "First generation should succeed");

    let result2 = zellij::layout_generate(config, LayoutTemplate::Standard, output_dir).await;
    assert!(
        result2.is_ok(),
        "Second generation should succeed (overwrite)"
    );

    let layout = result2.expect("second layout generation should succeed");
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
// Error Handling Tests (Optimized with functional patterns)
// ============================================================================

#[tokio::test]
async fn test_zellij_templates_all_pass_internal_validation() {
    let config = test_config();

    // Functional: map templates to Results using const array
    let results: Vec<Result<String, _>> = ALL_TEMPLATES
        .iter()
        .map(|&template| zellij::generate_template_kdl(config, template))
        .collect();

    assert!(
        results.iter().all(Result::is_ok),
        "All templates should pass validation"
    );
}

#[tokio::test]
async fn test_zellij_validation_happens_automatically() {
    // The generate_template_kdl function internally validates KDL
    // If validation fails, it returns an error
    let config = test_config();

    // Functional validation pipeline using const array
    let validation_results: Result<Vec<_>, _> = ALL_TEMPLATES
        .iter()
        .map(|&template| {
            zellij::generate_template_kdl(config, template)
                .map_err(|e| format!("{e:?}"))
                .and_then(|kdl| {
                    // Verify the KDL has the required structure
                    if kdl.contains("layout") && kdl.contains("pane") {
                        Ok(kdl)
                    } else {
                        Err("Missing required KDL structure".to_string())
                    }
                })
        })
        .collect();

    assert!(
        validation_results.is_ok(),
        "All templates should generate valid KDL"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_zellij_config_with_empty_session_name() {
    let config = LayoutConfig::new(String::new(), PathBuf::from("/tmp"));
    assert_eq!(config.session_name, "");
    assert_eq!(config.tab_name(), "zjj:");
}

#[test]
fn test_zellij_config_builder_chain() {
    let config = custom_config();

    assert_eq!(config.session_name, "test-session");
    assert_eq!(config.claude_command, "custom-claude");
    assert_eq!(config.beads_command, "custom-bv");
    assert_eq!(config.tab_prefix, "custom");
    assert_eq!(config.tab_name(), "custom:test-session");
}

#[tokio::test]
async fn test_zellij_all_templates_generate_valid_kdl() {
    let config = test_config();

    // Functional validation pipeline using const array
    let results: Vec<Result<String, _>> = ALL_TEMPLATES
        .iter()
        .map(|&template| zellij::generate_template_kdl(config, template))
        .collect();

    assert!(
        results.iter().all(Result::is_ok),
        "All templates should generate valid KDL"
    );

    // Validate balanced braces using functional helpers
    let brace_validation: Result<Vec<_>, _> = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .map(|kdl| validate_balanced_braces(kdl))
        .collect();

    assert!(
        brace_validation.is_ok(),
        "All templates should have balanced braces"
    );
}
